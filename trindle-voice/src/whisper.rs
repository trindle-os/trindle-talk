//! Whisper's two halves, written out step by step.
use candle_core::{Device, IndexOp, Result, Tensor};
use candle_nn::{
    Conv1d, Conv1dConfig, Embedding, LayerNorm, Linear, Module, VarBuilder, conv1d, embedding,
    layer_norm, linear, linear_no_bias,
};
use candle_transformers::models::whisper::Config;

struct Attention {
    query: Linear,
    key: Linear,
    value: Linear,
    out: Linear,
    heads: usize,

    kept: Option<(Tensor, Tensor)>,
}

impl Attention {
    fn load(size: usize, heads: usize, vb: VarBuilder) -> Result<Self> {
        Ok(Self {
            query: linear(size, size, vb.pp("q_proj"))?,
            key: linear_no_bias(size, size, vb.pp("k_proj"))?,
            value: linear(size, size, vb.pp("v_proj"))?,
            out: linear(size, size, vb.pp("out_proj"))?,
            heads,
            kept: None,
        })
    }

    fn split(&self, x: &Tensor) -> Result<Tensor> {
        let (b, n, size) = x.dims3()?;
        x.reshape((b, n, self.heads, size / self.heads))?.transpose(1, 2)?.contiguous()
    }

    fn keys_values(&self, x: &Tensor) -> Result<(Tensor, Tensor)> {
        Ok((self.split(&self.key.forward(x)?)?, self.split(&self.value.forward(x)?)?))
    }

    fn attend(&self, x: &Tensor, k: &Tensor, v: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        let (_, _, size) = x.dims3()?;
        let scale = ((size / self.heads) as f64).powf(-0.5);
        let q = (self.split(&self.query.forward(x)?)? * scale)?;
        let mut w = q.matmul(&k.t()?)?;
        if let Some(mask) = mask {
            w = w.broadcast_add(mask)?;
        }
        let w = candle_nn::ops::softmax_last_dim(&w)?;
        self.out.forward(&w.matmul(v)?.transpose(1, 2)?.flatten_from(2)?)
    }

    fn whole(&self, x: &Tensor) -> Result<Tensor> {
        let (k, v) = self.keys_values(x)?;
        self.attend(x, &k, &v, None)
    }

    fn growing(&mut self, x: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        let (k, v) = self.keys_values(x)?;
        let (k, v) = match self.kept.take() {
            Some((pk, pv)) => (Tensor::cat(&[pk, k], 2)?, Tensor::cat(&[pv, v], 2)?),
            None => (k, v),
        };
        let y = self.attend(x, &k, &v, mask);
        self.kept = Some((k, v));
        y
    }

    fn fixed(&self, x: &Tensor) -> Result<Tensor> {
        let (k, v) = self.kept.as_ref().expect("sound is set before writing");
        self.attend(x, k, v, None)
    }
}

struct Block {
    attention: Attention,
    attention_ln: LayerNorm,
    sound: Option<(Attention, LayerNorm)>,
    fc1: Linear,
    fc2: Linear,
    mlp_ln: LayerNorm,
}

impl Block {
    fn load(size: usize, heads: usize, listens: bool, vb: VarBuilder) -> Result<Self> {
        let sound = if listens {
            Some((
                Attention::load(size, heads, vb.pp("encoder_attn"))?,
                layer_norm(size, 1e-5, vb.pp("encoder_attn_layer_norm"))?,
            ))
        } else {
            None
        };
        Ok(Self {
            attention: Attention::load(size, heads, vb.pp("self_attn"))?,
            attention_ln: layer_norm(size, 1e-5, vb.pp("self_attn_layer_norm"))?,
            sound,
            fc1: linear(size, size * 4, vb.pp("fc1"))?,
            fc2: linear(size * 4, size, vb.pp("fc2"))?,
            mlp_ln: layer_norm(size, 1e-5, vb.pp("final_layer_norm"))?,
        })
    }

    fn mlp(&self, x: Tensor) -> Result<Tensor> {
        let y = self.fc2.forward(&self.fc1.forward(&self.mlp_ln.forward(&x)?)?.gelu()?)?;
        x + y
    }

    fn listen(&self, x: &Tensor) -> Result<Tensor> {
        let x = (x + self.attention.whole(&self.attention_ln.forward(x)?)?)?;
        self.mlp(x)
    }

    fn write(&mut self, x: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        let mut x = (x + self.attention.growing(&self.attention_ln.forward(x)?, mask)?)?;
        if let Some((attention, ln)) = &self.sound {
            x = (&x + attention.fixed(&ln.forward(&x)?)?)?;
        }
        self.mlp(x)
    }
}

fn sinusoids(length: usize, channels: usize) -> Result<Tensor> {
    let step = 10000f32.ln() / (channels / 2 - 1) as f32;
    let rates: Vec<f32> = (0..channels / 2).map(|i| (i as f32 * -step).exp()).collect();
    let rates = Tensor::new(rates.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
    let times = Tensor::arange(0f32, length as f32, &Device::Cpu)?.unsqueeze(1)?;
    let scaled = times.broadcast_mul(&rates)?;
    Tensor::cat(&[scaled.sin()?, scaled.cos()?], 1)
}

pub struct Whisper {
    conv1: Conv1d,
    conv2: Conv1d,
    listen_positions: Tensor,
    listening: Vec<Block>,
    listen_ln: LayerNorm,
    pieces: Embedding,
    write_positions: Tensor,
    writing: Vec<Block>,
    write_ln: LayerNorm,

    scorer: Tensor,
    written: usize,
    pub config: Config,
}

impl Whisper {
    pub fn load(vb: VarBuilder, config: Config) -> Result<Self> {
        let size = config.d_model;
        let (e, d) = (vb.pp("model.encoder"), vb.pp("model.decoder"));
        let one = Conv1dConfig { padding: 1, ..Default::default() };
        let two = Conv1dConfig { padding: 1, stride: 2, ..Default::default() };
        let pieces = embedding(config.vocab_size, size, d.pp("embed_tokens"))?;
        let scorer = pieces.embeddings().t()?.contiguous()?;
        Ok(Self {
            conv1: conv1d(config.num_mel_bins, size, 3, one, e.pp("conv1"))?,
            conv2: conv1d(size, size, 3, two, e.pp("conv2"))?,
            listen_positions: sinusoids(config.max_source_positions, size)?,
            listening: (0..config.encoder_layers)
                .map(|i| Block::load(size, config.encoder_attention_heads, false, e.pp(format!("layers.{i}"))))
                .collect::<Result<_>>()?,
            listen_ln: layer_norm(size, 1e-5, e.pp("layer_norm"))?,
            write_positions: d.get((config.max_target_positions, size), "embed_positions.weight")?,
            writing: (0..config.decoder_layers)
                .map(|i| Block::load(size, config.decoder_attention_heads, true, d.pp(format!("layers.{i}"))))
                .collect::<Result<_>>()?,
            write_ln: layer_norm(size, 1e-5, d.pp("layer_norm"))?,
            pieces,
            scorer,
            written: 0,
            config,
        })
    }

    pub fn hear(&mut self, mel: &Tensor) -> Result<()> {
        let x = self.conv1.forward(mel)?.gelu()?;
        let x = self.conv2.forward(&x)?.gelu()?.transpose(1, 2)?;
        let mut x = x.broadcast_add(&self.listen_positions.narrow(0, 0, x.dim(1)?)?)?;
        for block in &self.listening {
            x = block.listen(&x)?;
        }
        let sound = self.listen_ln.forward(&x)?;
        for block in &mut self.writing {
            block.attention.kept = None;
            if let Some((attention, _)) = &mut block.sound {
                attention.kept = Some(attention.keys_values(&sound)?);
            }
        }
        self.written = 0;
        Ok(())
    }

    pub fn next(&mut self, new: &[u32]) -> Result<Tensor> {
        let n = new.len();
        let x = self.pieces.forward(&Tensor::new(new, &Device::Cpu)?.unsqueeze(0)?)?;
        let mut x = x.broadcast_add(&self.write_positions.narrow(0, self.written, n)?)?;

        let mask = if n > 1 {
            let past = self.written;
            let cells: Vec<f32> = (0..n)
                .flat_map(|i| (0..past + n).map(move |j| if j > past + i { f32::NEG_INFINITY } else { 0.0 }))
                .collect();
            Some(Tensor::from_vec(cells, (n, past + n), &Device::Cpu)?)
        } else {
            None
        };
        for block in &mut self.writing {
            x = block.write(&x, mask.as_ref())?;
        }
        self.written += n;
        let last = self.write_ln.forward(&x.i((.., n - 1..n, ..))?)?;
        last.i(0)?.matmul(&self.scorer)?.i(0)
    }
}
