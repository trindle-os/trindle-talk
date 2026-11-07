#!/bin/sh
# Builds Talk as a Trindle plugin folder and archive.
set -eu

top=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$top/dist}
id=$(sed -n 's/^Id=//p' "$top/plugin.ini")
version=$(sed -n 's/^Version=//p' "$top/plugin.ini")
cargo_version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$top/Cargo.toml" | head -n 1)
if [ "$version" != "$cargo_version" ]; then
    echo "plugin.ini says $version but Cargo.toml says $cargo_version" >&2
    exit 1
fi

cargo_home=${CARGO_HOME:-$HOME/.cargo}
rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
flags() {
    printf '%s' "${RUSTFLAGS:-} --remap-path-prefix=$HOME=/home --remap-path-prefix=$top=/src --remap-path-prefix=$cargo_home=/cargo --remap-path-prefix=$rustup_home=/rust --remap-path-prefix=$1=/target"
}
talk_target=$top/target/shipped
voice_target=$top/trindle-voice/target/shipped

cargo test --release --locked --manifest-path "$top/Cargo.toml"
cargo test --release --locked --manifest-path "$top/trindle-voice/Cargo.toml"
RUSTFLAGS=$(flags "$talk_target") CARGO_TARGET_DIR=$talk_target cargo build --release --locked --manifest-path "$top/Cargo.toml"
RUSTFLAGS=$(flags "$voice_target") CARGO_TARGET_DIR=$voice_target cargo build --release --locked --manifest-path "$top/trindle-voice/Cargo.toml"

mkdir -p "$out"
dir=$out/$id
rm -rf "$dir" "$dir.tar.zst" "$dir.description.txt" "$dir.svg"
mkdir -p "$dir/bin"
install -m 0755 "$talk_target/release/talk" "$dir/bin/talk"
install -m 0755 "$voice_target/release/trindle-voice" "$dir/bin/trindle-voice"
mkdir -p "$dir/licences"
install -m 0644 "$top"/trindle-voice/licences/* "$dir/licences/"
install -m 0644 "$top/plugin.ini" "$top/LICENSE" "$top/packaging/talk.svg" "$dir/"
install -m 0644 "$top/packaging/description.txt" "$dir.description.txt"
install -m 0644 "$top/packaging/talk.svg" "$dir.svg"

if [ -n "$(find "$dir" -type l)" ]; then echo "the folder holds a symbolic link" >&2; exit 1; fi
for name in ring.json settings.ini notify.jsonl status.txt; do
    if [ -n "$(find "$dir" -name "$name")" ]; then echo "$name must not be shipped" >&2; exit 1; fi
done
for run in $(sed -n 's/^Run=\([^ ]*\).*/\1/p' "$dir/plugin.ini"); do
    [ -x "$dir/$run" ] || { echo "$run is not an executable file" >&2; exit 1; }
done
for program in "$dir"/bin/*; do
    leaked=$(strings -a "$program" | grep -c -F -e "$HOME" -e "$top" -e "$cargo_home" -e "$rustup_home" || true)
    [ "$leaked" = 0 ] || { echo "$program holds $leaked paths from the build computer" >&2; exit 1; }
    extra=$(ldd "$program" | awk '{print $1}' | grep -v -e '^linux-vdso' -e '^/lib64/ld-linux' -e '^libc\.so' -e '^libm\.so' -e '^libgcc_s\.so' || true)
    [ -z "$extra" ] || { echo "$program needs $extra, which would have to be carried in the folder" >&2; exit 1; }
done
[ "$(du -sm "$dir" | cut -f1)" -lt 500 ] || { echo "the plugin is over 500 MB" >&2; exit 1; }

tar --sort=name --owner=0 --group=0 --numeric-owner --mtime=@0 \
    -C "$out" -cf - "$id" | zstd -q -19 -o "$dir.tar.zst"

echo "$id $version: $(du -sh "$dir" | cut -f1) as a folder, $(du -h "$dir.tar.zst" | cut -f1) to download"
