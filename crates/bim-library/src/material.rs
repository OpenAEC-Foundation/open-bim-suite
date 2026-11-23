//! Materials: what a layer is made of. A material has a stable English key that layers and files
//! refer to, a name in four languages, a colour for the 3D view, and a hatch kind that says how a
//! cut through it is drawn. The pattern of a hatch kind depends on the drawing standard
//! ([`HatchKind::pattern`]), so the drawing settings of a country choose it.
//!
//! The materials, their colours and their hatches are Open BIM Studio's.

use crate::Lang;
use serde::{Deserialize, Serialize};

/// A name in each language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Names {
    pub nl: String,
    pub en: String,
    pub de: String,
    pub fr: String,
}

impl Names {
    pub fn new(nl: &str, en: &str, de: &str, fr: &str) -> Self {
        Self { nl: nl.into(), en: en.into(), de: de.into(), fr: fr.into() }
    }

    /// The same name in every language (a code such as "OSB", or a name read from a file).
    pub fn same(name: &str) -> Self {
        Self::new(name, name, name, name)
    }

    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::Nl => &self.nl,
            Lang::En => &self.en,
            Lang::De => &self.de,
            Lang::Fr => &self.fr,
        }
    }
}

/// How a cut through a material is hatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HatchKind {
    Brick,
    SandLime,
    Concrete,
    Timber,
    Steel,
    Insulation,
    Gypsum,
    /// A cavity: left empty.
    Air,
    Generic,
}

impl HatchKind {
    pub const ALL: [HatchKind; 9] = [
        HatchKind::Brick,
        HatchKind::SandLime,
        HatchKind::Concrete,
        HatchKind::Timber,
        HatchKind::Steel,
        HatchKind::Insulation,
        HatchKind::Gypsum,
        HatchKind::Air,
        HatchKind::Generic,
    ];

    /// The hatch of this kind under `standard`: NEN 47 in colour; DIN 1356, ISO 128 and ANSI in
    /// black on white, each with its own spacing and its own sign for masonry, concrete and timber.
    pub fn pattern(self, standard: DrawingStandard) -> HatchPattern {
        use DrawingStandard::*;
        use HatchKind::*;
        const WHITE: [u8; 3] = [255, 255, 255];
        const BLACK: [u8; 3] = [0, 0, 0];
        const GREY: [u8; 3] = [128, 128, 128];
        let lines = |fill, line, angle, spacing| HatchPattern { fill, line, angle, spacing, pair: 0.0, crossed: false, zigzag: false };
        let pairs = |fill, line, angle, spacing, pair| HatchPattern { pair, ..lines(fill, line, angle, spacing) };
        let crossed = |fill, line, angle, spacing| HatchPattern { crossed: true, ..lines(fill, line, angle, spacing) };
        let solid = |fill| lines(fill, fill, 45.0, 0.0);
        let zigzag = |fill| HatchPattern { zigzag: true, ..lines(fill, BLACK, 0.0, 0.0) };
        match (standard, self) {
            // brickwork is hatched with pairs of lines: the sign of NEN 47 as the hatch catalogue
            // has it, at 1:50 a pair every 2.25 mm with its lines 0.375 mm apart
            (Nen47, Brick) => pairs([212, 144, 143], BLACK, 45.0, 0.1125, 0.01875),
            (Nen47, Insulation) => zigzag([255, 253, 224]),
            (Nen47, Concrete) => solid([192, 192, 192]),
            (Nen47, SandLime) => lines([200, 192, 176], [168, 160, 144], 45.0, 0.08),
            (Nen47, Timber) => lines([240, 220, 185], BLACK, 45.0, 0.075),
            (Nen47, Steel) => solid(BLACK),
            (Nen47, Gypsum) => lines(WHITE, GREY, 45.0, 0.03),
            (Nen47, Air) => solid(WHITE),
            (Nen47, Generic) => lines(WHITE, GREY, 45.0, 0.06),
            (_, Insulation) => zigzag(WHITE),
            (_, Air) => solid(WHITE),
            (_, Steel) => solid(BLACK),
            (Din1356, Brick | SandLime) => lines(WHITE, BLACK, 45.0, 0.05),
            (Din1356, Concrete) => crossed(WHITE, BLACK, 45.0, 0.05),
            (Din1356, Timber) => lines(WHITE, BLACK, 0.0, 0.03),
            (Iso128, Brick | SandLime) => lines(WHITE, BLACK, 45.0, 0.04),
            (Iso128, Concrete) => crossed(WHITE, BLACK, 45.0, 0.08),
            (Iso128, Timber) => lines(WHITE, BLACK, 135.0, 0.04),
            (Ansi, Brick | SandLime) => lines(WHITE, BLACK, 45.0, 0.025),
            (Ansi, Concrete) => solid([210, 210, 210]),
            (Ansi, Timber) => lines(WHITE, BLACK, 45.0, 0.06),
            (_, Gypsum | Generic) => lines(WHITE, BLACK, 45.0, 0.06),
        }
    }
}

/// The drawing standards whose hatches are known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DrawingStandard {
    /// NEN 47 (the Netherlands), in colour.
    #[default]
    Nen47,
    /// DIN 1356 (Germany).
    Din1356,
    /// ISO 128 / BS 1192.
    Iso128,
    /// ANSI (the United States).
    Ansi,
}

impl DrawingStandard {
    pub const ALL: [DrawingStandard; 4] = [DrawingStandard::Nen47, DrawingStandard::Din1356, DrawingStandard::Iso128, DrawingStandard::Ansi];
}

/// A hatch: the fill behind it, its line colour, the angle of its lines (degrees on paper,
/// anticlockwise from the x axis) and their spacing (metres on the model, so a hatch scales with
/// the drawing; 0 for a solid fill without lines), crossed lines, or one zigzag across the layer.
/// Where its lines come in pairs, `pair` is how far the two lines of a pair are apart (metres on
/// the model; 0: single lines) and `spacing` how far the pairs are.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HatchPattern {
    pub fill: [u8; 3],
    pub line: [u8; 3],
    pub angle: f64,
    pub spacing: f64,
    #[serde(default)]
    pub pair: f64,
    pub crossed: bool,
    pub zigzag: bool,
}

/// A material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// Stable, English, lower case with hyphens ("sand-lime"): what layers and files refer to.
    pub key: String,
    /// Its names, as written in a type's name: lower case but for codes ("isolatie PIR").
    pub names: Names,
    pub hatch: HatchKind,
    /// Its colour in the 3D view, red, green and blue.
    pub colour: [u8; 3],
    /// The key of its own hatch in the hatch catalogue ("openaec-12-spruce"), where it has one:
    /// the sign of its kind of material, in its colour. Without one it is drawn by its hatch
    /// kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign: Option<String>,
    /// Its grade or strength class, as its standard writes it ("C24", "S355", "C20/25").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grade: Option<String>,
    /// Its hatch under a hatch standard, by the standard's key, where it names one of its own
    /// ([`crate::standard`]); under the others its sign gives it.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub hatches: std::collections::BTreeMap<String, crate::standard::MaterialHatch>,
}

impl Material {
    pub fn name(&self, lang: Lang) -> &str {
        self.names.get(lang)
    }

    /// A material drawn by its hatch kind.
    pub fn new(key: &str, names: Names, hatch: HatchKind, colour: [u8; 3]) -> Self {
        Self { key: key.into(), names, hatch, colour, sign: None, grade: None, hatches: Default::default() }
    }

    /// Its own hatch of the hatch catalogue, when it names one the catalogue has.
    pub fn catalogue_hatch(&self) -> Option<&'static crate::hatch::Hatch> {
        self.sign.as_deref().and_then(crate::hatch::hatch)
    }

    /// The strength class its grade names, of whatever family ([`crate::class`]): concrete,
    /// steel, reinforcing steel, a bolt or timber.
    pub fn class(&self) -> Option<&'static crate::class::StrengthClass> {
        self.grade.as_deref().and_then(crate::class::class)
    }

    /// The timber strength class its grade names, when it names one ([`crate::grade`]).
    pub fn timber_grade(&self) -> Option<&'static crate::grade::TimberGrade> {
        self.grade.as_deref().and_then(crate::grade::timber_grade)
    }

    /// Its colour as fractions of one, for IFCX and the renderer.
    pub fn colour_f64(&self) -> [f64; 3] {
        self.colour.map(|c| f64::from(c) / 255.0)
    }
}

/// The hatch of the catalogue a starter material is drawn with: the suite's sign for its kind
/// of material. The materials NEN 47 has no sign for are drawn by their hatch kind.
fn starter_sign(key: &str) -> Option<&'static str> {
    Some(match key {
        "concrete" => "openaec-5",
        "steel" | "galvanised-steel" => "openaec-18",
        "aluminium" | "zinc" | "copper" => "openaec-19",
        "roofing" => "openaec-24",
        "hollow-core-concrete" => "openaec-6",
        "brick" => "openaec-1",
        "sand-lime" => "openaec-3",
        "insulation" | "mineral-wool" => "openaec-17",
        "insulation-pir" => "openaec-17-pir",
        "timber" | "timber-cladding" => "openaec-12",
        "osb" => "openaec-16-osb",
        "plywood" => "openaec-16-plywood",
        "glass" => "openaec-29",
        "screed" => "openaec-10",
        // the kinds a model taken over from another program usually has (see `common`)
        "precast-concrete" => "openaec-6",
        "concrete-block" | "aerated-concrete" => "openaec-3",
        "natural-stone" => "openaec-9",
        "ceramic-tile" | "plaster" | "carpet" | "linoleum" | "laminate" => "openaec-10",
        "hardwood" => "openaec-13",
        "parquet" => "openaec-12-oak",
        "chipboard" | "fibre-cement" => "openaec-16",
        "mdf" => "openaec-16-mdf",
        "stainless-steel" | "cast-iron" | "steel-deck" => "openaec-18",
        "brass" => "openaec-19",
        "lead" => "openaec-20",
        "insulation-eps" => "openaec-17-eps",
        "insulation-xps" => "openaec-17",
        "glass-wool" => "openaec-17-glass-wool",
        "epdm" | "bitumen" => "openaec-24",
        "vinyl" | "rubber" | "plastic" | "polycarbonate" => "openaec-22",
        "mirror" => "openaec-29",
        "sealant" => "openaec-23",
        "earth" => "openaec-25-soil",
        "asphalt" => "openaec-25-asphalt",
        "sand" => "openaec-26",
        "gravel" => "openaec-27",
        "water" => "openaec-28",
        "clay" => "openaec-25-clay",
        "peat" => "openaec-25-peat",
        "frosted-glass" => "openaec-29-frosted-glass",
        "toughened-glass" => "openaec-29-clear-glass",
        "bamboo" => "openaec-12-bamboo",
        "anhydrite-screed" => "openaec-10",
        "concrete-paver" => "openaec-7",
        "gypsum-block" => "openaec-4",
        "polyester" => "openaec-22",
        "anodised-aluminium" => "openaec-19",
        _ => return None,
    })
}

/// The kinds of material a model usually has beside the suite's own: what the material library
/// of a modelling program such as Revit holds as its generic materials (masonry and stone,
/// timber and boards, metals, insulation, membranes, finishes, plastics, and the ground of a
/// site). The kinds are mirrored, not the content: the names, colours and hatches are the
/// suite's own, by its own rules, and nothing of another program's library is taken over. A
/// project starts with them, so a model taken over finds a material of its kind.
pub fn common() -> Vec<Material> {
    use HatchKind::*;
    let m = |key: &str, nl: &str, en: &str, de: &str, fr: &str, hatch, colour| Material { sign: starter_sign(key).map(str::to_string), ..Material::new(key, Names::new(nl, en, de, fr), hatch, colour) };
    vec![
        // masonry, stone and what is laid or spread on them
        m("precast-concrete", "prefab beton", "precast concrete", "Betonfertigteil", "béton préfabriqué", Concrete, [190, 190, 186]),
        m("concrete-block", "betonsteen", "concrete block", "Betonstein", "bloc de béton", SandLime, [170, 170, 166]),
        m("aerated-concrete", "cellenbeton", "aerated concrete", "Porenbeton", "béton cellulaire", SandLime, [240, 240, 235]),
        m("natural-stone", "natuursteen", "natural stone", "Naturstein", "pierre naturelle", Generic, [150, 150, 145]),
        m("ceramic-tile", "keramische tegels", "ceramic tiles", "keramische Fliesen", "carrelage céramique", Generic, [235, 232, 225]),
        m("plaster", "stucwerk", "plaster", "Putz", "enduit", Gypsum, [245, 243, 238]),
        // timber and boards
        m("hardwood", "hardhout", "hardwood", "Hartholz", "bois feuillu", Timber, [150, 100, 65]),
        m("parquet", "parket", "wood flooring", "Parkett", "parquet", Timber, [190, 150, 100]),
        m("chipboard", "spaanplaat", "chipboard", "Spanplatte", "panneau de particules", Timber, [200, 175, 130]),
        m("mdf", "MDF", "MDF", "MDF", "MDF", Timber, [190, 160, 120]),
        m("laminate", "laminaat", "laminate", "Laminat", "stratifié", Generic, [205, 185, 150]),
        m("fibre-cement", "vezelcement", "fibre cement", "Faserzement", "fibres-ciment", Generic, [200, 200, 195]),
        // metals
        m("stainless-steel", "roestvast staal", "stainless steel", "Edelstahl", "acier inoxydable", Steel, [200, 205, 210]),
        m("cast-iron", "gietijzer", "cast iron", "Gusseisen", "fonte", Steel, [90, 90, 95]),
        m("steel-deck", "staalplaat", "steel deck", "Stahltrapezblech", "bac acier", Steel, [170, 175, 180]),
        m("lead", "lood", "lead", "Blei", "plomb", Steel, [120, 125, 135]),
        m("brass", "messing", "brass", "Messing", "laiton", Steel, [200, 170, 90]),
        // insulation
        m("insulation-eps", "isolatie EPS", "EPS insulation", "EPS-Dämmung", "isolant PSE", Insulation, [240, 240, 240]),
        m("insulation-xps", "isolatie XPS", "XPS insulation", "XPS-Dämmung", "isolant XPS", Insulation, [200, 225, 240]),
        m("glass-wool", "glaswol", "glass wool", "Glaswolle", "laine de verre", Insulation, [255, 245, 160]),
        // membranes and what keeps water out
        m("epdm", "EPDM", "EPDM", "EPDM", "EPDM", Generic, [50, 50, 55]),
        m("bitumen", "bitumen", "bitumen", "Bitumen", "bitume", Generic, [40, 40, 40]),
        m("damp-proof-course", "waterkering", "damp-proof course", "Feuchtigkeitssperre", "barrière d'étanchéité", Generic, [30, 30, 30]),
        m("sealant", "kit", "sealant", "Dichtstoff", "mastic", Generic, [110, 110, 110]),
        // finishes
        m("carpet", "tapijt", "carpet", "Teppich", "moquette", Generic, [150, 140, 130]),
        m("vinyl", "vinyl", "vinyl flooring", "Vinylbelag", "revêtement vinyle", Generic, [190, 190, 185]),
        m("linoleum", "linoleum", "linoleum", "Linoleum", "linoléum", Generic, [170, 160, 140]),
        m("rubber", "rubber", "rubber", "Gummi", "caoutchouc", Generic, [60, 60, 60]),
        m("paint", "verf", "paint", "Anstrich", "peinture", Generic, [250, 250, 250]),
        m("textile", "textiel", "textile", "Textil", "textile", Generic, [200, 190, 175]),
        // plastics and glass
        m("plastic", "kunststof", "plastic", "Kunststoff", "plastique", Generic, [225, 225, 225]),
        m("polycarbonate", "polycarbonaat", "polycarbonate", "Polycarbonat", "polycarbonate", Generic, [215, 230, 240]),
        m("mirror", "spiegel", "mirror", "Spiegel", "miroir", Generic, [200, 215, 225]),
        // the ground of a site
        m("earth", "grond", "earth", "Erdreich", "terre", Generic, [150, 115, 80]),
        m("sand", "zand", "sand", "Sand", "sable", Generic, [225, 205, 155]),
        m("gravel", "grind", "gravel", "Kies", "gravier", Generic, [170, 165, 155]),
        m("asphalt", "asfalt", "asphalt", "Asphalt", "enrobé", Generic, [70, 70, 72]),
        m("grass", "gras", "grass", "Rasen", "gazon", Generic, [120, 160, 90]),
        m("water", "water", "water", "Wasser", "eau", Generic, [150, 195, 225]),
        m("clay", "klei", "clay", "Ton", "argile", Generic, [140, 120, 100]),
        m("peat", "veen", "peat", "Torf", "tourbe", Generic, [85, 65, 50]),
        m("planting", "beplanting", "planting", "Bepflanzung", "plantation", Generic, [80, 130, 70]),
        // a roof and what grows on it
        m("roof-tile", "dakpannen", "roof tiles", "Dachziegel", "tuiles", Brick, [165, 85, 60]),
        m("reed", "riet", "reed", "Reet", "chaume", Generic, [190, 165, 110]),
        m("sedum", "sedum", "sedum", "Sedum", "sedum", Generic, [110, 150, 90]),
        m("roof-substrate", "daktuinsubstraat", "roof garden substrate", "Dachsubstrat", "substrat de toiture", Generic, [120, 95, 75]),
        m("drainage-layer", "drainagelaag", "drainage layer", "Dränschicht", "couche drainante", Generic, [95, 100, 105]),
        // more of what a floor and a partition are made of
        m("anhydrite-screed", "anhydriet dekvloer", "anhydrite screed", "Anhydritestrich", "chape anhydrite", Concrete, [225, 222, 212]),
        m("concrete-paver", "betontegels", "concrete pavers", "Betonplatten", "dalles en béton", Concrete, [160, 160, 156]),
        m("gypsum-block", "gipsblokken", "gypsum blocks", "Gipsdielen", "carreaux de plâtre", Gypsum, [240, 238, 232]),
        m("polyester", "polyester", "polyester", "Polyester", "polyester", Generic, [235, 235, 230]),
        // variants a model tells apart
        m("toughened-glass", "gehard glas", "toughened glass", "Sicherheitsglas", "verre trempé", Generic, [185, 215, 230]),
        m("frosted-glass", "mat glas", "frosted glass", "Mattglas", "verre dépoli", Generic, [215, 228, 232]),
        m("anodised-aluminium", "geanodiseerd aluminium", "anodised aluminium", "eloxiertes Aluminium", "aluminium anodisé", Steel, [165, 168, 172]),
        m("bamboo", "bamboe", "bamboo", "Bambus", "bambou", Timber, [205, 180, 120]),
    ]
}

/// The materials a library starts with: Open BIM Studio's.
pub fn starter() -> Vec<Material> {
    use HatchKind::*;
    let m = |key: &str, nl: &str, en: &str, de: &str, fr: &str, hatch, colour| Material { sign: starter_sign(key).map(str::to_string), ..Material::new(key, Names::new(nl, en, de, fr), hatch, colour) };
    const CONCRETE: [u8; 3] = [185, 185, 185];
    const INSULATION: [u8; 3] = [240, 225, 150];
    const TIMBER: [u8; 3] = [205, 165, 115];
    const GENERIC: [u8; 3] = [175, 182, 195];
    vec![
        m("concrete", "beton", "concrete", "Beton", "béton", Concrete, CONCRETE),
        m("hollow-core-concrete", "kanaalplaat", "hollow-core slab", "Spannbeton-Hohldiele", "dalle alvéolée", Concrete, CONCRETE),
        m("wide-slab", "breedplaat", "wide-slab plank", "Elementdecke", "prédalle", Concrete, CONCRETE),
        m("topping-concrete", "opstort", "structural topping", "Aufbeton", "béton de clavetage", Concrete, [196, 196, 192]),
        m("brick", "metselwerk", "brickwork", "Mauerwerk", "maçonnerie", Brick, [205, 140, 115]),
        m("sand-lime", "kalkzandsteen", "sand-lime brick", "Kalksandstein", "brique silico-calcaire", SandLime, [225, 220, 205]),
        m("insulation", "isolatie", "insulation", "Dämmung", "isolant", Insulation, INSULATION),
        m("insulation-pir", "isolatie PIR", "PIR insulation", "PIR-Dämmung", "isolant PIR", Insulation, INSULATION),
        m("mineral-wool", "minerale wol", "mineral wool", "Mineralwolle", "laine minérale", Insulation, [236, 214, 120]),
        m("timber", "hout", "timber", "Holz", "bois", Timber, TIMBER),
        m("clt", "kruislaaghout", "cross-laminated timber", "Brettsperrholz", "bois lamellé-croisé", Timber, [214, 178, 128]),
        m("glulam", "gelamineerd hout", "glulam", "Brettschichtholz", "bois lamellé-collé", Timber, [210, 172, 120]),
        m("timber-cladding", "houten gevelbekleding", "timber cladding", "Holzverschalung", "bardage bois", Timber, TIMBER),
        m("timber-frame-insulated", "houtskelet met isolatie", "timber frame with insulation", "Holzständerwerk mit Dämmung", "ossature bois isolée", Insulation, INSULATION),
        m("osb", "OSB", "OSB", "OSB", "OSB", Timber, [214, 182, 128]),
        m("plywood", "multiplex", "plywood", "Sperrholz", "contreplaqué", Timber, [222, 196, 150]),
        m("gypsum-board", "gipsplaat", "gypsum board", "Gipskartonplatte", "plaque de plâtre", Gypsum, [240, 240, 236]),
        m("breather-membrane", "dampopen folie", "breather membrane", "diffusionsoffene Bahn", "pare-pluie", Generic, [62, 66, 72]),
        m("vapour-control-membrane", "dampremmende folie", "vapour control layer", "Dampfbremse", "pare-vapeur", Generic, [196, 214, 226]),
        m("metal-stud-insulated", "metal stud met isolatie", "metal studs with insulation", "Metallständerwerk mit Dämmung", "ossature métallique isolée", Insulation, INSULATION),
        m("steel", "staal", "steel", "Stahl", "acier", Steel, [95, 100, 110]),
        m("aluminium", "aluminium", "aluminium", "Aluminium", "aluminium", Generic, [196, 200, 206]),
        m("zinc", "zink", "zinc", "Zink", "zinc", Generic, [150, 160, 168]),
        m("glass", "glas", "glass", "Glas", "verre", Generic, [160, 200, 225]),
        m("air", "spouw", "air cavity", "Luftschicht", "lame d'air", Air, GENERIC),
        m("mortar", "mortel", "mortar", "Mörtel", "mortier", Generic, [228, 224, 212]),
        m("glue", "lijm", "adhesive", "Klebstoff", "colle", Generic, [210, 208, 200]),
        m("grout", "voegmortel", "grout", "Fugenmörtel", "coulis", Generic, [210, 208, 200]),
        m("screed", "dekvloer", "screed", "Estrich", "chape", Generic, [200, 196, 188]),
        m("mineral-fibre", "mineraalvezel", "mineral fibre", "Mineralfaser", "fibre minérale", Generic, [238, 238, 232]),
        m("hpl", "HPL", "HPL", "HPL", "HPL", Generic, [72, 76, 82]),
        m("pvc", "PVC", "PVC", "PVC", "PVC", Generic, [244, 244, 240]),
        m("galvanised-steel", "verzinkt staal", "galvanised steel", "verzinkter Stahl", "acier galvanisé", Steel, [176, 182, 188]),
        m("copper", "koper", "copper", "Kupfer", "cuivre", Generic, [184, 115, 51]),
        m("polyethylene", "polyetheen", "polyethylene", "Polyethylen", "polyéthylène", Generic, [40, 44, 52]),
        m("elastomeric-foam", "elastomeerschuim", "elastomeric foam", "Elastomerschaum", "mousse élastomère", Insulation, [52, 54, 58]),
        m("roofing", "dakbedekking", "roofing", "Dachabdichtung", "étanchéité de toiture", Generic, [42, 46, 49]),
        m("generic", "algemeen", "generic", "allgemein", "générique", Generic, GENERIC),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn starter_materials_have_unique_keys_and_every_name() {
        let materials = starter();
        let keys: HashSet<&str> = materials.iter().map(|m| m.key.as_str()).collect();
        assert_eq!(keys.len(), materials.len(), "keys are unique");
        for m in &materials {
            assert!(m.key.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{}", m.key);
            for lang in Lang::ALL {
                assert!(!m.name(lang).is_empty(), "{} has a name in {lang:?}", m.key);
            }
        }
    }

    #[test]
    fn hatches_follow_open_bim_studio() {
        let brick = HatchKind::Brick.pattern(DrawingStandard::Nen47);
        assert_eq!((brick.fill, brick.angle, brick.spacing, brick.pair), ([212, 144, 143], 45.0, 0.1125, 0.01875));
        // brickwork's pairs of lines are those of the sign in the hatch catalogue, which is drawn
        // for 1:50: its tile is the pairs' spacing, its two lines are a pair
        let sign = crate::hatch::sign(crate::HatchSet::Nen47, 1).expect("the sign for brickwork");
        let crate::hatch::Mark::Line { points: first, .. } = &sign.marks[0] else { panic!("a line") };
        let crate::hatch::Mark::Line { points: second, .. } = &sign.marks[1] else { panic!("a line") };
        let on_model = |millimetres: f64| millimetres * sign.scale[0] * 50.0 / 1000.0;
        assert_eq!((on_model(sign.tile[0]), on_model(second[0][0] - first[0][0]), sign.angle), (brick.spacing, brick.pair, brick.angle));
        // only brickwork under NEN 47 has pairs
        assert!(DrawingStandard::ALL.into_iter().flat_map(|s| HatchKind::ALL.map(|k| (s, k))).all(|(s, k)| (k.pattern(s).pair > 0.0) == (s == DrawingStandard::Nen47 && k == HatchKind::Brick)));
        assert!(HatchKind::Insulation.pattern(DrawingStandard::Nen47).zigzag);
        assert!(HatchKind::Concrete.pattern(DrawingStandard::Din1356).crossed, "reinforced concrete is crossed in DIN 1356");
        assert_eq!(HatchKind::Timber.pattern(DrawingStandard::Iso128).angle, 135.0);
        assert_eq!(HatchKind::Concrete.pattern(DrawingStandard::Ansi).fill, [210, 210, 210], "a grey tone in ANSI");
        for standard in DrawingStandard::ALL {
            let steel = HatchKind::Steel.pattern(standard);
            assert_eq!((steel.fill, steel.spacing), ([0, 0, 0], 0.0), "steel is solid black in {standard:?}");
            let air = HatchKind::Air.pattern(standard);
            assert_eq!((air.fill, air.spacing), ([255, 255, 255], 0.0), "a cavity is left empty in {standard:?}");
            for kind in HatchKind::ALL {
                let p = kind.pattern(standard);
                assert!(p.spacing >= 0.0 && p.angle.is_finite());
            }
        }
    }

    #[test]
    fn a_material_has_its_colour_as_fractions() {
        let concrete = starter().into_iter().find(|m| m.key == "concrete").unwrap();
        assert_eq!(concrete.colour_f64(), [185.0 / 255.0; 3]);
        assert_eq!(concrete.name(Lang::De), "Beton");
    }
    #[test]
    fn the_common_kinds_of_material_are_in_the_general_library_each_once() {
        let common = common();
        assert_eq!(common.len(), 55);
        let library = crate::Library::starter();
        assert_eq!(library.materials.len(), starter().len() + common.len());
        let mut keys: Vec<&str> = library.materials.iter().map(|m| m.key.as_str()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), library.materials.len(), "no key twice");
        for m in &common {
            // named in every language, lower case but for names that are codes
            for lang in Lang::ALL {
                assert!(!m.name(lang).trim().is_empty(), "{} in {lang:?}", m.key);
            }
            assert!(m.name(Lang::Nl).chars().next().is_some_and(|c| c.is_lowercase()) || m.name(Lang::Nl).chars().all(|c| !c.is_lowercase()), "{}", m.key);
            // a sign it names is one the catalogue has
            assert!(m.sign.is_none() || m.catalogue_hatch().is_some(), "{}: {:?}", m.key, m.sign);
        }
        // what a model taken over usually has is there: masonry, boards, metals, finishes, a site
        for key in ["concrete-block", "natural-stone", "hardwood", "mdf", "stainless-steel", "insulation-eps", "epdm", "carpet", "paint", "plastic", "earth", "asphalt", "water"] {
            assert!(library.material(key).is_some(), "{key}");
        }
        assert_eq!(library.material("natural-stone").and_then(|m| m.catalogue_hatch()).map(|h| h.sign), Some(9));
        assert_eq!(library.material("paint").and_then(|m| m.sign.clone()), None);
    }

}
