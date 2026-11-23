//! The kind of a material: its NL-SfB material code, and which material of the suite a material
//! of another program is by its name.
//!
//! - **The code** ([`code`], [`crate::Material::material_code`]) is the letter and digit Dutch
//!   models write before a material's name (`f2_...` concrete, `g2_...` fired clay, `i2_...`
//!   softwood), after table 3 of NL-SfB. It follows from the material's key. The codes are as
//!   Dutch Revit files written to that convention use them; they are not checked against the
//!   table itself here.
//! - **The match** ([`key_of`], [`crate::Library::material_like`]) reads the name a material has
//!   in a model that is taken over, Dutch or English ("g2_baksteen_rood", "Stainless steel,
//!   brushed"), and says which material of the suite it is a kind of, by the words in it. The
//!   more particular word wins: glass wool is no glass, reinforcing steel no concrete.
//!
//! In IFCX the code is the attribute `nlsfb::material` on the material's node.
//!
//! Not yet: the readers do not use the match; a material taken over keeps its own name and
//! colour.

use crate::material::Material;
use crate::Library;

/// The attribute of a material's node with its NL-SfB material code.
pub const MATERIAL_CODE: &str = "nlsfb::material";

/// The NL-SfB material code of the material with `key`, where the convention has one for it.
pub fn code(key: &str) -> Option<&'static str> {
    Some(match key {
        "natural-stone" => "e1",
        "sand-lime" => "f1",
        "concrete" | "hollow-core-concrete" | "wide-slab" | "topping-concrete" | "precast-concrete" | "unreinforced-concrete" | "screed" | "concrete-block" | "concrete-paver" | "fibre-cement" | "mortar" | "grout" => "f2",
        "aerated-concrete" => "f4",
        "gypsum-board" | "gypsum-block" | "anhydrite-screed" => "f7",
        "brick" | "brick-porous" | "roof-tile" => "g2",
        "ceramic-tile" => "g3",
        "cast-iron" => "h1",
        "steel" | "galvanised-steel" | "steel-deck" => "h2",
        "stainless-steel" => "h3",
        "aluminium" | "anodised-aluminium" => "h4",
        "copper" => "h5",
        "zinc" => "h7",
        "lead" => "h8",
        "bamboo" => "i1",
        "timber" | "timber-cladding" | "glulam" | "clt" | "timber-clt" => "i2",
        "hardwood" | "parquet" => "i3",
        "plywood" => "i4",
        "chipboard" => "j1",
        "reed" | "sedum" | "textile" => "j3",
        "air" => "j9",
        "insulation" | "mineral-wool" | "glass-wool" | "mineral-fibre" => "m1",
        "epdm" | "polyethylene" | "pvc" => "n6",
        "insulation-pir" | "insulation-pur" | "insulation-eps" | "insulation-xps" => "n7",
        "polyester" | "carpet" => "n9",
        "glass" | "toughened-glass" => "o1",
        "frosted-glass" => "o2",
        "mirror" => "o7",
        "earth" | "sand" | "gravel" | "clay" | "peat" | "roof-substrate" | "drainage-layer" => "p1",
        "asphalt" => "s1",
        "bitumen" => "s9",
        "water" => "z1",
        _ => return None,
    })
}

/// The words that say which material of the suite a name is of: every fragment of one of a
/// rule's alternatives is in the name. Tried in order, the more particular first. A fragment
/// with spaces round it is a whole word.
const WORDS: &[(&[&[&str]], &str)] = &[
    // what holds a word of another material in its name comes before that material
    (&[&["betonstaal"], &["wapening"], &["reinforc"], &["rebar"]], "steel"),
    (&[&["kalkzandsteen"], &["sand", "lime"], &["calcium", "silicate"]], "sand-lime"),
    (&[&["cellenbeton"], &["gasbeton"], &["aerated"]], "aerated-concrete"),
    (&[&["cementvezel"], &["vezelcement"], &["fibre", "cement"], &["fiber", "cement"]], "fibre-cement"),
    (&[&["anhydriet"], &["anhydrite"]], "anhydrite-screed"),
    (&[&["dekvloer"], &["zandcement"], &["screed"]], "screed"),
    (&[&["tegel", "beton"], &["tegel", "grind"], &["paver"], &["paving"]], "concrete-paver"),
    (&[&["tegel"], &["porcel"], &[" tile"], &["ceramic"], &["keramisch"]], "ceramic-tile"),
    (&[&["kanaalplaat"], &["hollow", "core"]], "hollow-core-concrete"),
    (&[&["breedplaat"]], "wide-slab"),
    (&[&["betonsteen"], &["betonblok"], &["concrete", "block"], &["masonry", "unit"]], "concrete-block"),
    (&[&["prefab"], &["precast"]], "precast-concrete"),
    (&[&["beton"], &["concrete"]], "concrete"),
    (&[&["stucwerk"], &["stuc"], &["plaster"], &["pleister"]], "plaster"),
    (&[&["gips", "blok"], &["gypsum", "block"]], "gypsum-block"),
    (&[&["gips"], &["gypsum"]], "gypsum-board"),
    (&[&["dakpan"], &["roof", "tile"]], "roof-tile"),
    (&[&["baksteen"], &["metselwerk"], &["poriso"], &["brick"], &["masonry"]], "brick"),
    (&[&["natuursteen"], &["hardsteen"], &["granite"], &["graniet"], &["marble"], &["marmer"], &[" stone"]], "natural-stone"),
    (&[&["voeg"], &["grout"]], "grout"),
    (&[&["mortel"], &["specie"], &["mortar"]], "mortar"),
    // metals
    (&[&["gietijzer"], &["cast", "iron"], &[" iron"]], "cast-iron"),
    (&[&["rvs"], &["roestvast"], &["stainless"]], "stainless-steel"),
    (&[&["verzinkt"], &["galvan"], &[" galv "]], "galvanised-steel"),
    (&[&["staalplaat"], &["steel", "deck"], &["metal", "deck"]], "steel-deck"),
    (&[&["staal"], &["steel"]], "steel"),
    (&[&["anodi"], &["geanodi"]], "anodised-aluminium"),
    (&[&["alumin"]], "aluminium"),
    (&[&["koper"], &["copper"]], "copper"),
    (&[&["messing"], &["brass"]], "brass"),
    (&[&["zink"], &["zinc"]], "zinc"),
    (&[&[" lood "], &[" lead "]], "lead"),
    // insulation: before glass and stone
    (&[&["glaswol"], &["glass", "wool"]], "glass-wool"),
    (&[&["steenwol"], &["minerale", "wol"], &["mineral", "wool"], &["stone", "wool"], &["rockwool"]], "mineral-wool"),
    (&[&[" eps"]], "insulation-eps"),
    (&[&[" xps"]], "insulation-xps"),
    (&[&[" pir"]], "insulation-pir"),
    (&[&["isolatie"], &["insulation"], &[" pur"]], "insulation"),
    // timber and boards
    (&[&["bamboe"], &["bamboo"]], "bamboo"),
    (&[&["multiplex"], &["plywood"], &["underlayment"]], "plywood"),
    (&[&[" osb"]], "osb"),
    (&[&["spaanplaat"], &["chipboard"], &["particle"]], "chipboard"),
    (&[&[" mdf"]], "mdf"),
    (&[&["parket"], &["flooring"], &["vloerafwerking", "hout"]], "parquet"),
    (&[&["kruislaag"], &[" clt"]], "clt"),
    (&[&["eiken"], &["mahonie"], &["meranti"], &["merbau"], &["azobe"], &["iroko"], &["bangkirai"], &["hardhout"], &["loofhout"], &["hout", "loof"], &["hardwood"], &[" oak"]], "hardwood"),
    (&[&["hout"], &["vuren"], &["grenen"], &["douglas"], &["wood"], &["timber"], &["lumber"]], "timber"),
    (&[&["riet"], &["reed"], &["thatch"]], "reed"),
    (&[&["sedum"]], "sedum"),
    (&[&["substraat"], &["substrate"]], "roof-substrate"),
    (&[&["drainage"]], "drainage-layer"),
    // membranes and plastics
    (&[&[" dpc"], &["waterkering"], &["damp", "proof"]], "damp-proof-course"),
    (&[&["epdm"]], "epdm"),
    (&[&["bitum"]], "bitumen"),
    (&[&[" pvc"], &["polyvinyl"]], "pvc"),
    (&[&[" pe "], &["polyeth"], &["polyetheen"]], "polyethylene"),
    (&[&["polyester"]], "polyester"),
    (&[&["polycarbona"]], "polycarbonate"),
    (&[&["rubber"]], "rubber"),
    (&[&["kunststof"], &["plastic"]], "plastic"),
    // glass
    (&[&["spiegel"], &["mirror"]], "mirror"),
    (&[&["gehard"], &["hardglas"], &["toughened"], &["tempered"]], "toughened-glass"),
    (&[&["glas", " mat"], &["satijn"], &["frosted"]], "frosted-glass"),
    (&[&["glas"], &["glass"], &["glazing"]], "glass"),
    // the ground of a site
    (&[&["asfalt"], &["asphalt"]], "asphalt"),
    (&[&["klei"], &[" clay"]], "clay"),
    (&[&["veen"], &[" peat"]], "peat"),
    (&[&["grind"], &["gravel"]], "gravel"),
    (&[&[" zand"], &[" sand "]], "sand"),
    (&[&["grond"], &[" earth"], &[" soil"]], "earth"),
    (&[&["beplanting"], &[" heg"], &["hedera"], &["planting"], &["hedge"]], "planting"),
    (&[&["gras"], &["grass"]], "grass"),
    // (water is in many names that are no water: a pipe, a fleece; so only by its code)
    (&[&[" z1 "], &[" p1 ", "water"]], "water"),
    // finishes
    (&[&["tapijt"], &["carpet"]], "carpet"),
    (&[&["linoleum"]], "linoleum"),
    (&[&["vinyl"]], "vinyl"),
    (&[&["laminaat"], &["laminate"]], "laminate"),
    (&[&["textiel"], &["textile"], &["linnen"], &["fabric"]], "textile"),
    (&[&["verf"], &["paint"], &[" ral "]], "paint"),
    (&[&["lucht"], &[" air "]], "air"),
];

/// The name as the words are looked for in it: lower case, what is no letter or digit a space,
/// a space at both ends.
fn spaced(name: &str) -> String {
    let spaced: String = name.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' }).collect();
    format!(" {} ", spaced.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// How the names of working materials start, as they are looked for in a name.
const WORKING: &[&str] = &[" analytical", " phase", " default", " poche", " render material", " system", " air openings", " air surfaces"];

/// The material a code alone stands for, where a name says no more than its code
/// ("f2_C20/25"): the plainest of its kind.
const PLAIN: &[(&str, &str)] = &[
    ("e1", "natural-stone"),
    ("f1", "sand-lime"),
    ("f2", "concrete"),
    ("f4", "aerated-concrete"),
    ("f7", "gypsum-board"),
    ("g2", "brick"),
    ("g3", "ceramic-tile"),
    ("h1", "cast-iron"),
    ("h2", "steel"),
    ("h3", "steel"),
    ("h4", "aluminium"),
    ("h5", "copper"),
    ("h7", "zinc"),
    ("h8", "lead"),
    ("i1", "bamboo"),
    ("i2", "timber"),
    ("i3", "hardwood"),
    ("i4", "plywood"),
    ("j1", "chipboard"),
    ("m1", "insulation"),
    ("n6", "plastic"),
    ("n7", "insulation"),
    ("o1", "glass"),
    ("o2", "frosted-glass"),
    ("o7", "mirror"),
    ("p1", "earth"),
    ("s1", "asphalt"),
    ("z1", "water"),
];

/// The NL-SfB material code a name is written with: a letter and a digit as its first word, or
/// as its second after a word of capitals ("NLRS_f2_...").
pub fn code_of(name: &str) -> Option<&'static str> {
    let mut words = name.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty());
    let first = words.next()?;
    let word = if first.len() > 2 && first.chars().all(|c| c.is_ascii_uppercase()) { words.next()? } else { first };
    PLAIN.iter().find(|(code, _)| *code == word).map(|(code, _)| *code)
}

/// The key of the suite's material that a material called `name` in another program is a kind
/// of: by the words in the name, else by the material code it is written with; none when
/// neither says it.
pub fn key_of(name: &str) -> Option<&'static str> {
    let spaced = spaced(name);
    // what a program keeps for its own working (a default, a phase, an analytical surface) is
    // of no kind, whatever word is in it
    if WORKING.iter().any(|start| spaced.starts_with(start)) {
        return None;
    }
    if spaced == " water " {
        return Some("water");
    }
    let by_words = WORDS.iter().find(|(alternatives, _)| alternatives.iter().any(|all| all.iter().all(|fragment| spaced.contains(fragment)))).map(|(_, key)| *key);
    by_words.or_else(|| code_of(name).and_then(|code| PLAIN.iter().find(|(of, _)| *of == code)).map(|(_, key)| *key))
}

impl Material {
    /// Its NL-SfB material code, where the convention has one for its kind.
    pub fn material_code(&self) -> Option<&'static str> {
        code(&self.key)
    }
}

impl Library {
    /// The material of this library that a material called `name` in another program is a
    /// kind of: one whose own name it is, else the one its words say, where the library has it.
    pub fn material_like(&self, name: &str) -> Option<&Material> {
        let wanted = name.trim();
        let own = self.materials.iter().find(|m| crate::Lang::ALL.iter().any(|lang| m.name(*lang).eq_ignore_ascii_case(wanted)) || m.key.eq_ignore_ascii_case(wanted));
        own.or_else(|| key_of(name).and_then(|key| self.material(key)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_material_has_its_code_by_its_key() {
        let library = Library::starter();
        let of = |key: &str| library.material(key).unwrap_or_else(|| panic!("{key}")).material_code();
        assert_eq!((of("concrete"), of("precast-concrete"), of("brick"), of("sand-lime"), of("timber"), of("hardwood")), (Some("f2"), Some("f2"), Some("g2"), Some("f1"), Some("i2"), Some("i3")));
        assert_eq!((of("steel"), of("aluminium"), of("glass"), of("insulation-eps"), of("mineral-wool"), of("earth")), (Some("h2"), Some("h4"), Some("o1"), Some("n7"), Some("m1"), Some("p1")));
        assert_eq!((of("generic"), of("paint")), (None, None));
        // a code is a letter and a digit, and most of the general library has one
        let coded = library.materials.iter().filter(|m| m.material_code().is_some()).count();
        assert!(coded * 10 >= library.materials.len() * 6, "{coded} of {}", library.materials.len());
        for m in &library.materials {
            if let Some(code) = m.material_code() {
                let mut chars = code.chars();
                assert!(code.len() == 2 && chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.next().is_some_and(|c| c.is_ascii_digit()), "{}: {code}", m.key);
            }
        }
        // in a file the material's node has it
        let node = crate::ifcx::material_node(library.material("brick").unwrap());
        assert_eq!(node["attributes"][MATERIAL_CODE]["code"], "g2");
        assert!(crate::ifcx::material_node(library.material("generic").unwrap())["attributes"].get(MATERIAL_CODE).is_none());
    }

    #[test]
    fn a_name_of_another_program_says_which_material_it_is_a_kind_of() {
        for (name, key) in [
            ("f2_beton_prefab_grijs", "precast-concrete"),
            ("f2_beton_ter_plaatse", "concrete"),
            ("Concrete, grey", "concrete"),
            ("h2_betonstaal_staaf", "steel"),
            ("f1_kalkzandsteen_blokken", "sand-lime"),
            ("f4_cellenbeton_blokken", "aerated-concrete"),
            ("f2_dekvloer_zandcement", "screed"),
            ("f7_dekvloer_anhydriet", "anhydrite-screed"),
            ("g2_baksteen_rood", "brick"),
            ("g2_dakpan_antraciet", "roof-tile"),
            ("g3_tegels_keramisch_wit", "ceramic-tile"),
            ("f2_tegels_beton_300", "concrete-paver"),
            ("e1_natuursteen_grijs", "natural-stone"),
            ("Stainless steel, brushed", "stainless-steel"),
            ("h2_staal_gegalvaniseerd", "galvanised-steel"),
            ("Steel, structural", "steel"),
            ("h4_aluminium_geanodiseerd_zwart", "anodised-aluminium"),
            ("h8_lood", "lead"),
            ("m1_isolatie_glaswol", "glass-wool"),
            ("m1_isolatie_steenwol", "mineral-wool"),
            ("n7_isolatie_EPS", "insulation-eps"),
            ("n7_isolatie_PIR", "insulation-pir"),
            ("Rigid insulation", "insulation"),
            ("i2_hout_vuren", "timber"),
            ("i3_hout_eiken", "hardwood"),
            ("Oak flooring", "parquet"),
            ("i4_beplating_multiplex", "plywood"),
            ("j1_spaanplaat", "chipboard"),
            ("o7_glas_spiegelend", "mirror"),
            ("o2_glas_mat", "frosted-glass"),
            ("Glass, clear glazing", "glass"),
            ("n6_epdm", "epdm"),
            ("s9_bitumen", "bitumen"),
            ("p1_ondergrond_klei", "clay"),
            ("p1_zand", "sand"),
            ("s1_asfalt_zwart", "asphalt"),
            ("z1_water", "water"),
            ("RAL 9010", "paint"),
            ("j9_lucht_spouw", "air"),
            // a name that says no more than its code is the plainest of its kind
            ("f2_C20/25", "concrete"),
            ("NLRS_i2_C24", "timber"),
            ("4_dpc_folie", "damp-proof-course"),
            ("Azobe", "hardwood"),
            ("Water", "water"),
            ("300_hout_loof_creme", "hardwood"),
        ] {
            assert_eq!(key_of(name), Some(key), "{name}");
        }
        // a working material of another program is of no kind
        for name in ["Default", "Default Wall", "Poche", "Phase - Exist", "Analytical Wall Surface", "Analytical Surface - Air Openings", "Hot water pipe", ""] {
            assert_eq!(key_of(name), None, "{name}");
        }
        // every key the words give is a material of the general library
        let library = Library::starter();
        for key in WORDS.iter().map(|(_, key)| key).chain(PLAIN.iter().map(|(_, key)| key)) {
            assert!(library.material(key).is_some(), "{key}");
        }
        // the code a name is written with
        assert_eq!((code_of("f2_beton"), code_of("NLRS_g2_baksteen"), code_of("Glass"), code_of("x9_iets"), code_of("")), (Some("f2"), Some("g2"), None, None, None));
        // the library's own name first, then the words
        assert_eq!(library.material_like("Brickwork").map(|m| m.key.as_str()), Some("brick"));
        assert_eq!(library.material_like("g2_baksteen_rood").map(|m| m.key.as_str()), Some("brick"));
        assert_eq!(library.material_like("Poche"), None);
    }
}
