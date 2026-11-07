//! American to British spelling for English dictation.
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const OUR_STEMS: &[&str] = &[
    "color", "honor", "favor", "flavor", "savor", "labor", "neighbor", "harbor", "humor", "rumor",
    "vapor", "odor", "armor", "candor", "clamor", "splendor", "valor", "vigor", "parlor", "tumor",
    "endeavor", "behavior", "arbor", "ardor", "demeanor", "fervor", "rigor", "succor",
];
const OUR_ENDS: &[&str] = &[
    "", "s", "ed", "ing", "er", "ers", "ful", "fully", "less", "lessly", "able", "ably", "ite",
    "ites", "hood", "hoods", "al", "ally", "y", "ies",
];

const ISE_STEMS: &[&str] = &[
    "apolog", "author", "capital", "categor", "central", "colon", "critic", "custom", "digit",
    "emphas", "fertil", "harmon", "hospital", "hypothes", "idol", "immun", "individual",
    "industrial", "initial", "internal", "item", "jeopard", "legal", "liberal", "local", "magnet",
    "marginal", "material", "maxim", "mechan", "memor", "mesmer", "metabol", "militar", "minim",
    "mobil", "modern", "monopol", "moral", "national", "natural", "neutral", "normal", "notar",
    "optim", "organ", "ostrac", "oxid", "pasteur", "patron", "penal", "personal", "plagiar",
    "polar", "popular", "pressur", "priorit", "private", "public", "pulver", "quant", "radical",
    "random", "rational", "real", "recogn", "reorgan", "revital", "revolution", "ritual",
    "romantic", "satir", "scandal", "scrutin", "secular", "sensational", "social", "special",
    "stabil", "standard", "steril", "subsid", "summar", "sympath", "symbol", "synchron",
    "synthes", "systemat", "tantal", "terror", "theor", "trivial", "urban", "util", "vandal",
    "vapor", "verbal", "victim", "visual", "vital", "vocal", "vulcan", "western",
];
const ISE_ENDS: &[&str] =
    &["ize", "izes", "ized", "izing", "ization", "izations", "izer", "izers", "izable"];

const YSE_STEMS: &[&str] = &["anal", "paral", "catal", "breathal"];
const YSE_ENDS: &[&str] = &["yze", "yzes", "yzed", "yzing", "yzer", "yzers"];

const RE: &[(&str, &str)] = &[
    ("center", "centre"),
    ("centers", "centres"),
    ("centered", "centred"),
    ("centering", "centring"),
    ("centerpiece", "centrepiece"),
    ("epicenter", "epicentre"),
    ("theater", "theatre"),
    ("theaters", "theatres"),
    ("liter", "litre"),
    ("liters", "litres"),
    ("milliliter", "millilitre"),
    ("milliliters", "millilitres"),
    ("kilometer", "kilometre"),
    ("kilometers", "kilometres"),
    ("centimeter", "centimetre"),
    ("centimeters", "centimetres"),
    ("millimeter", "millimetre"),
    ("millimeters", "millimetres"),
    ("nanometer", "nanometre"),
    ("nanometers", "nanometres"),
    ("fiber", "fibre"),
    ("fibers", "fibres"),
    ("fiberglass", "fibreglass"),
    ("caliber", "calibre"),
    ("calibers", "calibres"),
    ("somber", "sombre"),
    ("somberly", "sombrely"),
    ("specter", "spectre"),
    ("specters", "spectres"),
    ("luster", "lustre"),
    ("lackluster", "lacklustre"),
    ("saber", "sabre"),
    ("sabers", "sabres"),
    ("scepter", "sceptre"),
    ("ocher", "ochre"),
    ("maneuver", "manoeuvre"),
    ("maneuvers", "manoeuvres"),
    ("maneuvered", "manoeuvred"),
    ("maneuvering", "manoeuvring"),
    ("maneuverable", "manoeuvrable"),
];

const LL: &[(&str, &str)] = &[
    ("traveled", "travelled"),
    ("traveling", "travelling"),
    ("traveler", "traveller"),
    ("travelers", "travellers"),
    ("canceled", "cancelled"),
    ("canceling", "cancelling"),
    ("modeled", "modelled"),
    ("modeling", "modelling"),
    ("modeler", "modeller"),
    ("labeled", "labelled"),
    ("labeling", "labelling"),
    ("marveled", "marvelled"),
    ("marveling", "marvelling"),
    ("marvelous", "marvellous"),
    ("marvelously", "marvellously"),
    ("counseled", "counselled"),
    ("counseling", "counselling"),
    ("counselor", "counsellor"),
    ("counselors", "counsellors"),
    ("jeweler", "jeweller"),
    ("jewelers", "jewellers"),
    ("jewelry", "jewellery"),
    ("signaled", "signalled"),
    ("signaling", "signalling"),
    ("totaled", "totalled"),
    ("totaling", "totalling"),
    ("fueled", "fuelled"),
    ("fueling", "fuelling"),
    ("refueled", "refuelled"),
    ("refueling", "refuelling"),
    ("dialed", "dialled"),
    ("dialing", "dialling"),
    ("equaled", "equalled"),
    ("equaling", "equalling"),
    ("quarreled", "quarrelled"),
    ("quarreling", "quarrelling"),
    ("rivaled", "rivalled"),
    ("rivaling", "rivalling"),
    ("tunneled", "tunnelled"),
    ("tunneling", "tunnelling"),
    ("unraveled", "unravelled"),
    ("unraveling", "unravelling"),
    ("initialed", "initialled"),
    ("funneled", "funnelled"),
    ("funneling", "funnelling"),
    ("paneled", "panelled"),
    ("paneling", "panelling"),
    ("leveled", "levelled"),
    ("leveling", "levelling"),
    ("shoveled", "shovelled"),
    ("shoveling", "shovelling"),
    ("spiraled", "spiralled"),
    ("spiraling", "spiralling"),
    ("groveled", "grovelled"),
    ("snorkeled", "snorkelled"),
    ("worshiped", "worshipped"),
    ("worshiping", "worshipping"),
    ("worshiper", "worshipper"),
    ("woolen", "woollen"),
    ("woolens", "woollens"),
    ("wooly", "woolly"),
    ("tranquility", "tranquillity"),

    ("willful", "wilful"),
    ("willfully", "wilfully"),
    ("skillful", "skilful"),
    ("skillfully", "skilfully"),
    ("fulfill", "fulfil"),
    ("fulfills", "fulfils"),
    ("fulfillment", "fulfilment"),
    ("enroll", "enrol"),
    ("enrolls", "enrols"),
    ("enrollment", "enrolment"),
    ("installment", "instalment"),
    ("installments", "instalments"),
    ("distill", "distil"),
    ("distills", "distils"),
    ("instill", "instil"),
    ("instills", "instils"),
    ("appall", "appal"),
    ("appalls", "appals"),
    ("enthrall", "enthral"),
    ("enthralls", "enthrals"),
];

const ENCE: &[(&str, &str)] = &[
    ("defense", "defence"),
    ("defenses", "defences"),
    ("defenseless", "defenceless"),
    ("offense", "offence"),
    ("offenses", "offences"),
    ("pretense", "pretence"),
    ("pretenses", "pretences"),
];

const OGUE: &[(&str, &str)] = &[
    ("catalog", "catalogue"),
    ("catalogs", "catalogues"),
    ("cataloged", "catalogued"),
    ("cataloging", "cataloguing"),
    ("monolog", "monologue"),
    ("monologs", "monologues"),
    ("epilog", "epilogue"),
    ("epilogs", "epilogues"),
];

const AE: &[(&str, &str)] = &[
    ("anesthesia", "anaesthesia"),
    ("anesthetic", "anaesthetic"),
    ("anesthetics", "anaesthetics"),
    ("anesthetist", "anaesthetist"),
    ("archeology", "archaeology"),
    ("archeological", "archaeological"),
    ("archeologist", "archaeologist"),
    ("archeologists", "archaeologists"),
    ("paleontology", "palaeontology"),
    ("esthetic", "aesthetic"),
    ("esthetics", "aesthetics"),
    ("esthetically", "aesthetically"),
    ("anemia", "anaemia"),
    ("anemic", "anaemic"),
    ("diarrhea", "diarrhoea"),
    ("edema", "oedema"),
    ("celiac", "coeliac"),
    ("esophagus", "oesophagus"),
    ("estrogen", "oestrogen"),
    ("gynecology", "gynaecology"),
    ("gynecologist", "gynaecologist"),
    ("gynecological", "gynaecological"),
    ("hemoglobin", "haemoglobin"),
    ("hemorrhage", "haemorrhage"),
    ("hemorrhaging", "haemorrhaging"),
    ("hemorrhoids", "haemorrhoids"),
    ("leukemia", "leukaemia"),
    ("orthopedic", "orthopaedic"),
    ("orthopedics", "orthopaedics"),
    ("pediatric", "paediatric"),
    ("pediatrics", "paediatrics"),
    ("pediatrician", "paediatrician"),
];

const ODD: &[(&str, &str)] = &[
    ("airplane", "aeroplane"),
    ("airplanes", "aeroplanes"),
    ("aluminum", "aluminium"),
    ("ax", "axe"),
    ("gray", "grey"),
    ("grayer", "greyer"),
    ("grayish", "greyish"),
    ("grayscale", "greyscale"),
    ("mold", "mould"),
    ("molds", "moulds"),
    ("molded", "moulded"),
    ("molding", "moulding"),
    ("moldy", "mouldy"),
    ("molt", "moult"),
    ("molting", "moulting"),
    ("smolder", "smoulder"),
    ("smoldering", "smouldering"),
    ("mustache", "moustache"),
    ("mustaches", "moustaches"),
    ("pajamas", "pyjamas"),
    ("plow", "plough"),
    ("plows", "ploughs"),
    ("plowed", "ploughed"),
    ("plowing", "ploughing"),
    ("skeptic", "sceptic"),
    ("skeptics", "sceptics"),
    ("skeptical", "sceptical"),
    ("skeptically", "sceptically"),
    ("skepticism", "scepticism"),
    ("sulfur", "sulphur"),
    ("sulfuric", "sulphuric"),
    ("sulfide", "sulphide"),
    ("cozy", "cosy"),
    ("cozier", "cosier"),
    ("coziness", "cosiness"),
    ("donut", "doughnut"),
    ("donuts", "doughnuts"),
    ("tidbit", "titbit"),
    ("math", "maths"),
    ("aging", "ageing"),
    ("judgment", "judgement"),
    ("judgments", "judgements"),
    ("specialty", "speciality"),
    ("specialties", "specialities"),
    ("likable", "likeable"),
    ("sizable", "sizeable"),
    ("salable", "saleable"),
    ("unshakable", "unshakeable"),
    ("artifact", "artefact"),
    ("artifacts", "artefacts"),
    ("omelet", "omelette"),
    ("omelets", "omelettes"),
    ("chili", "chilli"),
    ("chilies", "chillies"),
];

fn grow(words: &mut HashMap<String, String>, stems: &[&str], ends: &[&str], old: &str, new: &str) {
    for stem in stems {
        for end in ends {
            words.insert(format!("{stem}{end}"), format!("{}{end}", stem.replace(old, new)));
        }
    }
}

fn grow_end(
    words: &mut HashMap<String, String>,
    stems: &[&str],
    ends: &[&str],
    old: &str,
    new: &str,
) {
    for stem in stems {
        for end in ends {
            words.insert(format!("{stem}{end}"), format!("{stem}{}", end.replace(old, new)));
        }
    }
}

fn build() -> HashMap<String, String> {
    let mut words = HashMap::new();
    grow(&mut words, OUR_STEMS, OUR_ENDS, "or", "our");
    grow_end(&mut words, ISE_STEMS, ISE_ENDS, "iz", "is");
    grow_end(&mut words, YSE_STEMS, YSE_ENDS, "yz", "ys");
    for group in [RE, LL, ENCE, OGUE, AE, ODD] {
        for (from, to) in group {
            words.insert((*from).to_string(), (*to).to_string());
        }
    }

    for word in ["humoral", "tumoral"] {
        words.remove(word);
    }

    let british: HashSet<String> = words.values().cloned().collect();
    words.retain(|from, _| !british.contains(from));
    words
}

pub fn british() -> &'static HashMap<String, String> {
    static LIST: OnceLock<HashMap<String, String>> = OnceLock::new();
    LIST.get_or_init(build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_everyday_ones_are_there() {
        let list = british();
        for (from, to) in [
            ("color", "colour"),
            ("colors", "colours"),
            ("colorful", "colourful"),
            ("behavior", "behaviour"),
            ("organize", "organise"),
            ("organization", "organisation"),
            ("analyze", "analyse"),
            ("center", "centre"),
            ("traveling", "travelling"),
            ("defense", "defence"),
            ("catalog", "catalogue"),
            ("pediatric", "paediatric"),
            ("gray", "grey"),
        ] {
            assert_eq!(list.get(from).map(String::as_str), Some(to), "{from}");
        }
    }

    #[test]
    fn the_ones_deliberately_left_alone_are_absent() {
        let list = british();
        for word in [
            "check", "practice", "program", "tire", "curb", "draft", "story", "disk", "burned",
            "yogurt", "enquiry", "medieval", "fetus", "honorary", "humorous", "laborious",
            "capsize", "diameter", "license", "defensive",
        ] {
            assert!(!list.contains_key(word), "{word} should be left alone");
        }
    }
}
