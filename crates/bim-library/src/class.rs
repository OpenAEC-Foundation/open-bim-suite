//! Strength classes of every family: what the grade a material names says of it, and where
//! each value is from.
//!
//! One place answers for a grade ([`class`]): concrete ("C20/25"), structural steel ("S355"),
//! reinforcing steel ("B500B"), bolts ("8.8") and timber ("C24", "GL28h", from
//! [`crate::grade`]). A class ([`StrengthClass`]) has its values by their symbol, the partial
//! factors that go with it, and its [`Source`]: the standard, its year and the table or clause.
//!
//! What is here is not a copy of a standard's table:
//! - A concrete class is its name: the two numbers are the cylinder and the cube strength. The
//!   other values follow from the formulae of EN 1992-1-1, table 3.1 (fcm = fck + 8, fctm =
//!   0.30 fck^(2/3) up to C50/60 and 2.12 ln(1 + fcm/10) above, Ecm = 22 (fcm/10)^0.3), rounded
//!   as that table rounds them.
//! - A bolt class is its name: "8.8" is a tensile strength of 800 N/mm² and a yield of 0.8 of it.
//! - Structural and reinforcing steel have the nominal values of their name and of their product
//!   standard.
//!
//! The partial factors are the values the Eurocodes recommend. A country's annex may set others;
//! none is checked here, and nothing here is validated by an engineer: the Normenatlas is to be
//! the source of validated values, and holds none yet.
//!
//! Not yet: exposure classes, prestressing steel, masonry, aluminium, welds, cross laminated
//! timber; steel thicker than 80 mm; the values a national annex changes.

use crate::grade::{self, TimberKind};
use serde::Serialize;
use serde_json::{json, Map, Value as Json};
use std::sync::OnceLock;

/// The attribute of a material's node with the class its grade names.
pub const STRENGTH_CLASS: &str = "openaec::strengthclass";

/// What a class is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Concrete,
    Steel,
    Reinforcement,
    Bolt,
    Timber,
}

impl Family {
    pub const ALL: [Family; 5] = [Family::Concrete, Family::Steel, Family::Reinforcement, Family::Bolt, Family::Timber];
}

/// The unit of a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Unit {
    /// N/mm².
    #[serde(rename = "N/mm2")]
    Stress,
    /// kg/m³.
    #[serde(rename = "kg/m3")]
    Density,
    /// A ratio or a factor.
    #[serde(rename = "-")]
    None,
    /// Percent.
    #[serde(rename = "%")]
    Percent,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Unit::Stress => "N/mm²",
            Unit::Density => "kg/m³",
            Unit::None => "",
            Unit::Percent => "%",
        }
    }
}

/// One value of a class, by its symbol as the standard writes it ("fck", "Ecm", "fy").
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Value {
    pub symbol: &'static str,
    pub value: f64,
    pub unit: Unit,
}

/// Where the values of a class are from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Source {
    /// "EN 1992-1-1".
    pub standard: &'static str,
    /// The year of the edition; none where the one the values were taken from is not known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    /// The table or clause ("table 3.1").
    pub place: &'static str,
    /// Where the factors are from ("EN 1992-1-1, 2.4.2.4, recommended").
    pub factors: &'static str,
}

impl Source {
    /// "EN 1992-1-1:2004, table 3.1".
    pub fn label(&self) -> String {
        match self.year {
            Some(year) => format!("{}:{year}, {}", self.standard, self.place),
            None => format!("{}, {}", self.standard, self.place),
        }
    }
}

/// A strength class: its values, its partial factors and its source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrengthClass {
    /// As its standard writes it: "C20/25", "S355", "B500B", "8.8", "C24".
    pub name: String,
    pub family: Family,
    pub values: Vec<Value>,
    /// The partial factors of the material, as the Eurocode recommends them.
    pub factors: Vec<Value>,
    pub source: Source,
}

impl StrengthClass {
    /// The value with `symbol`.
    pub fn get(&self, symbol: &str) -> Option<f64> {
        self.values.iter().find(|v| v.symbol == symbol).map(|v| v.value)
    }

    /// The factor with `symbol` ("γC").
    pub fn factor(&self, symbol: &str) -> Option<f64> {
        self.factors.iter().find(|v| v.symbol == symbol).map(|v| v.value)
    }

    /// The symbol of its modulus of elasticity.
    fn modulus(&self) -> &'static str {
        match self.family {
            Family::Concrete => "Ecm",
            Family::Steel | Family::Bolt => "E",
            Family::Reinforcement => "Es",
            Family::Timber => "E0,mean",
        }
    }

    /// The modulus of elasticity a structural model reads, in N/m².
    pub fn e(&self) -> Option<f64> {
        self.get(self.modulus()).map(|e| e * 1e6)
    }

    /// The shear modulus, in N/m², where the class has one.
    pub fn g(&self) -> Option<f64> {
        self.get("G").or_else(|| self.get("Gmean")).map(|g| g * 1e6)
    }

    /// The density in kg/m³: of timber the characteristic one.
    pub fn density(&self) -> Option<f64> {
        self.get("ρ").or_else(|| self.get("ρk"))
    }

    /// The strength a class is known by, in N/mm²: fck, fy, fyk, fub or fm,k.
    pub fn strength(&self) -> Option<f64> {
        self.get(match self.family {
            Family::Concrete => "fck",
            Family::Steel => "fy",
            Family::Reinforcement => "fyk",
            Family::Bolt => "fub",
            Family::Timber => "fm,k",
        })
    }

    /// The class as lines to read: its values by unit, its factors, its source.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for unit in [Unit::Stress, Unit::Density, Unit::None, Unit::Percent] {
            let of: Vec<String> = self.values.iter().filter(|v| v.unit == unit).map(|v| format!("{} {}", v.symbol, v.value)).collect();
            if !of.is_empty() {
                lines.push(format!("{} {}", of.join(" · "), unit.label()).trim_end().to_string());
            }
        }
        if !self.factors.is_empty() {
            lines.push(self.factors.iter().map(|v| format!("{} {}", v.symbol, v.value)).collect::<Vec<_>>().join(" · "));
        }
        lines.push(self.source.label());
        lines
    }

    /// The properties IFC has for it, in SI units (Pa, kg/m³), by their name in the property
    /// sets of a material (`Pset_MaterialCommon`, `Pset_MaterialMechanical`,
    /// `Pset_MaterialConcrete`, `Pset_MaterialSteel`).
    pub fn ifc_properties(&self) -> Vec<(&'static str, f64)> {
        let mut out = Vec::new();
        let mut add = |name, value: Option<f64>| out.extend(value.map(|v| (name, v)));
        add("MassDensity", self.density());
        add("YoungModulus", self.e());
        add("ShearModulus", self.g());
        match self.family {
            Family::Concrete => add("CompressiveStrength", self.get("fck").map(|f| f * 1e6)),
            Family::Steel => {
                add("YieldStress", self.get("fy").map(|f| f * 1e6));
                add("UltimateStress", self.get("fu").map(|f| f * 1e6));
            }
            Family::Reinforcement => add("YieldStress", self.get("fyk").map(|f| f * 1e6)),
            Family::Bolt => {
                add("YieldStress", self.get("fyb").map(|f| f * 1e6));
                add("UltimateStress", self.get("fub").map(|f| f * 1e6));
            }
            Family::Timber => {}
        }
        out
    }

    /// What a material's node says of its class: the class itself under [`STRENGTH_CLASS`], and
    /// IFC's own properties.
    pub fn attributes(&self) -> Map<String, Json> {
        let mut map = Map::new();
        map.insert(STRENGTH_CLASS.into(), json!(self));
        for (name, value) in self.ifc_properties() {
            map.insert(format!("bsi::ifc::prop::{name}"), json!(value));
        }
        map
    }
}

fn stress(symbol: &'static str, value: f64) -> Value {
    Value { symbol, value, unit: Unit::Stress }
}

fn factor(symbol: &'static str, value: f64) -> Value {
    Value { symbol, value, unit: Unit::None }
}

fn round(value: f64, decimals: i32) -> f64 {
    let by = 10f64.powi(decimals);
    (value * by).round() / by
}

/// The concrete classes of EN 1992-1-1: cylinder and cube strength.
const CONCRETE: [(f64, f64); 14] = [(12.0, 15.0), (16.0, 20.0), (20.0, 25.0), (25.0, 30.0), (30.0, 37.0), (35.0, 45.0), (40.0, 50.0), (45.0, 55.0), (50.0, 60.0), (55.0, 67.0), (60.0, 75.0), (70.0, 85.0), (80.0, 95.0), (90.0, 105.0)];

fn concrete(fck: f64, cube: f64) -> StrengthClass {
    let fcm = fck + 8.0;
    let fctm = if fck <= 50.0 { 0.30 * fck.powf(2.0 / 3.0) } else { 2.12 * (1.0 + fcm / 10.0).ln() };
    StrengthClass {
        name: format!("C{fck}/{cube}"),
        family: Family::Concrete,
        values: vec![
            stress("fck", fck),
            stress("fck,cube", cube),
            stress("fcm", fcm),
            stress("fctm", round(fctm, 1)),
            stress("fctk,0.05", round(0.7 * fctm, 1)),
            stress("fctk,0.95", round(1.3 * fctm, 1)),
            stress("Ecm", round(22.0 * (fcm / 10.0).powf(0.3), 0) * 1000.0),
            // (plain concrete; with reinforcement EN 1991-1-1 adds 100 kg/m³)
            Value { symbol: "ρ", value: 2400.0, unit: Unit::Density },
        ],
        factors: vec![factor("γC", 1.5)],
        source: Source { standard: "EN 1992-1-1", year: Some(2004), place: "table 3.1", factors: "EN 1992-1-1, 2.4.2.4, recommended" },
    }
}

/// Structural steel: yield and tensile strength up to 40 mm, and from 40 to 80 mm.
const STEEL: [(&str, [f64; 4]); 5] = [("S235", [235.0, 360.0, 215.0, 360.0]), ("S275", [275.0, 430.0, 255.0, 410.0]), ("S355", [355.0, 490.0, 335.0, 470.0]), ("S420", [420.0, 520.0, 390.0, 520.0]), ("S460", [460.0, 540.0, 430.0, 540.0])];

fn steel(name: &str, [fy, fu, fy_thick, fu_thick]: [f64; 4]) -> StrengthClass {
    StrengthClass {
        name: name.into(),
        family: Family::Steel,
        values: vec![
            stress("fy", fy),
            stress("fu", fu),
            stress("fy,40-80", fy_thick),
            stress("fu,40-80", fu_thick),
            stress("E", 210_000.0),
            stress("G", 81_000.0),
            Value { symbol: "ρ", value: 7850.0, unit: Unit::Density },
            Value { symbol: "ν", value: 0.3, unit: Unit::None },
        ],
        factors: vec![factor("γM0", 1.0), factor("γM1", 1.0), factor("γM2", 1.25)],
        source: Source { standard: "EN 1993-1-1", year: Some(2005), place: "table 3.1 and 3.2.6", factors: "EN 1993-1-1, 6.1, recommended" },
    }
}

/// Reinforcing steel: its ductility class with the least k = (ft/fy)k and strain at the
/// greatest force, in percent.
const REINFORCEMENT: [(&str, Option<(f64, f64)>); 4] = [("B500", None), ("B500A", Some((1.05, 2.5))), ("B500B", Some((1.08, 5.0))), ("B500C", Some((1.15, 7.5)))];

fn reinforcement(name: &str, ductility: Option<(f64, f64)>) -> StrengthClass {
    let mut values = vec![stress("fyk", 500.0), stress("Es", 200_000.0), Value { symbol: "ρ", value: 7850.0, unit: Unit::Density }];
    if let Some((k, strain)) = ductility {
        values.push(Value { symbol: "k", value: k, unit: Unit::None });
        values.push(Value { symbol: "εuk", value: strain, unit: Unit::Percent });
    }
    StrengthClass {
        name: name.into(),
        family: Family::Reinforcement,
        values,
        factors: vec![factor("γS", 1.15)],
        source: Source { standard: "EN 1992-1-1", year: Some(2004), place: "3.2 and annex C", factors: "EN 1992-1-1, 2.4.2.4, recommended" },
    }
}

/// The bolt classes of EN 1993-1-8.
const BOLTS: [(f64, f64); 7] = [(4.0, 6.0), (4.0, 8.0), (5.0, 6.0), (5.0, 8.0), (6.0, 8.0), (8.0, 8.0), (10.0, 9.0)];

fn bolt(tensile: f64, ratio: f64) -> StrengthClass {
    let fub = tensile * 100.0;
    StrengthClass {
        name: format!("{tensile}.{ratio}"),
        family: Family::Bolt,
        values: vec![stress("fyb", fub * ratio / 10.0), stress("fub", fub), stress("E", 210_000.0)],
        factors: vec![factor("γM2", 1.25)],
        source: Source { standard: "EN 1993-1-8", year: Some(2005), place: "table 3.1", factors: "EN 1993-1-8, 2.2, recommended" },
    }
}

fn timber(g: &grade::TimberGrade) -> StrengthClass {
    let glulam = g.kind == TimberKind::Glulam;
    StrengthClass {
        name: g.grade.clone(),
        family: Family::Timber,
        values: vec![
            stress("fm,k", g.fm_k),
            stress("ft,0,k", g.ft_0_k),
            stress("ft,90,k", g.ft_90_k),
            stress("fc,0,k", g.fc_0_k),
            stress("fc,90,k", g.fc_90_k),
            stress("fv,k", g.fv_k),
            stress("E0,05", g.e_0_u_k),
            stress("E0,mean", g.e_0_mean),
            stress("E90,mean", g.e_90_mean),
            stress("Gmean", g.g_mean),
            Value { symbol: "ρk", value: g.density(), unit: Unit::Density },
        ],
        factors: vec![factor("γM", if glulam { 1.25 } else { 1.3 })],
        source: Source { standard: if glulam { "EN 14080" } else { "EN 338" }, year: None, place: "as Project Ocondat has it", factors: "EN 1995-1-1, table 2.3, recommended" },
    }
}

/// Every class: concrete, structural steel, reinforcing steel, bolts, then timber, each rising.
pub fn classes() -> &'static [StrengthClass] {
    static CLASSES: OnceLock<Vec<StrengthClass>> = OnceLock::new();
    CLASSES.get_or_init(|| {
        let mut all: Vec<StrengthClass> = CONCRETE.iter().map(|(fck, cube)| concrete(*fck, *cube)).collect();
        all.extend(STEEL.iter().map(|(name, values)| steel(name, *values)));
        all.extend(REINFORCEMENT.iter().map(|(name, ductility)| reinforcement(name, *ductility)));
        all.extend(BOLTS.iter().map(|(tensile, ratio)| bolt(*tensile, *ratio)));
        all.extend(grade::timber_grades().iter().map(timber));
        all
    })
}

/// The classes of one family.
pub fn of(family: Family) -> impl Iterator<Item = &'static StrengthClass> {
    classes().iter().filter(move |c| c.family == family)
}

/// The class a grade names, whatever its case and with or without spaces ("c20/25", "S355",
/// "b500b", "8.8", "gl28H"). A steel grade names its class by its start: "S235JR", "S355J2+N".
pub fn class(grade: &str) -> Option<&'static StrengthClass> {
    let grade: String = grade.chars().filter(|c| !c.is_whitespace()).collect();
    if grade.is_empty() {
        return None;
    }
    let all = classes();
    all.iter().find(|c| c.name.eq_ignore_ascii_case(&grade)).or_else(|| {
        // a structural steel with its quality after its class
        let upper = grade.to_ascii_uppercase();
        all.iter().filter(|c| c.family == Family::Steel).find(|c| upper.strip_prefix(c.name.as_str()).is_some_and(|rest| rest.starts_with(|ch: char| ch.is_ascii_alphabetic())))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Library;

    #[test]
    fn a_concrete_class_follows_from_its_name() {
        assert_eq!(of(Family::Concrete).map(|c| c.name.as_str()).collect::<Vec<_>>(), ["C12/15", "C16/20", "C20/25", "C25/30", "C30/37", "C35/45", "C40/50", "C45/55", "C50/60", "C55/67", "C60/75", "C70/85", "C80/95", "C90/105"]);
        let values = |name: &str| {
            let c = class(name).unwrap();
            ["fck", "fck,cube", "fcm", "fctm", "fctk,0.05", "fctk,0.95", "Ecm"].map(|s| c.get(s).unwrap())
        };
        // the values EN 1992-1-1 tabulates for these classes
        assert_eq!(values("C12/15"), [12.0, 15.0, 20.0, 1.6, 1.1, 2.0, 27000.0]);
        assert_eq!(values("c20/25"), [20.0, 25.0, 28.0, 2.2, 1.5, 2.9, 30000.0]);
        assert_eq!(values("C30/37"), [30.0, 37.0, 38.0, 2.9, 2.0, 3.8, 33000.0]);
        assert_eq!(values("C50/60"), [50.0, 60.0, 58.0, 4.1, 2.9, 5.3, 37000.0]);
        assert_eq!(values("C90/105"), [90.0, 105.0, 98.0, 5.0, 3.5, 6.6, 44000.0]);
        let c30 = class(" C30 / 37 ").unwrap();
        assert_eq!((c30.e(), c30.g(), c30.density(), c30.strength(), c30.factor("γC")), (Some(33.0e9), None, Some(2400.0), Some(30.0), Some(1.5)));
        assert_eq!(c30.source.label(), "EN 1992-1-1:2004, table 3.1");
        // what is no class of the standard is none: Ocondat's "C25/35" ("C20" alone is timber)
        assert!(class("C25/35").is_none() && class("C20/30").is_none() && class("").is_none());
        // a stronger class is stiffer and bears more tension
        for pair in of(Family::Concrete).collect::<Vec<_>>().windows(2) {
            assert!(pair[1].get("Ecm") >= pair[0].get("Ecm") && pair[1].get("fctm") > pair[0].get("fctm"), "{} to {}", pair[0].name, pair[1].name);
        }
    }

    #[test]
    fn steel_reinforcement_and_bolts_have_their_nominal_values() {
        let s355 = class("s355").unwrap();
        assert_eq!((s355.get("fy"), s355.get("fu"), s355.get("fy,40-80"), s355.get("fu,40-80")), (Some(355.0), Some(490.0), Some(335.0), Some(470.0)));
        assert_eq!((s355.e(), s355.g(), s355.density(), s355.factor("γM2")), (Some(210.0e9), Some(81.0e9), Some(7850.0), Some(1.25)));
        // a grade with its quality is the class it starts with
        assert_eq!((class("S235JR").map(|c| c.name.as_str()), class("S355J2+N").map(|c| c.name.as_str()), class("S235JR2").map(|c| c.name.as_str())), (Some("S235"), Some("S355"), Some("S235")));
        assert!(class("S3550").is_none() && class("S500").is_none());
        for c in of(Family::Steel) {
            let named: f64 = c.name[1..].parse().unwrap();
            assert_eq!(c.get("fy"), Some(named), "{}", c.name);
            assert!(c.get("fu") > c.get("fy") && c.get("fy,40-80") < c.get("fy"), "{}", c.name);
        }
        let b500b = class("B500B").unwrap();
        assert_eq!((b500b.family, b500b.get("fyk"), b500b.get("k"), b500b.get("εuk"), b500b.e(), b500b.factor("γS")), (Family::Reinforcement, Some(500.0), Some(1.08), Some(5.0), Some(200.0e9), Some(1.15)));
        assert_eq!(class("B500").unwrap().get("k"), None, "without its ductility class, no ductility");
        assert_eq!(of(Family::Bolt).map(|c| c.name.as_str()).collect::<Vec<_>>(), ["4.6", "4.8", "5.6", "5.8", "6.8", "8.8", "10.9"]);
        assert_eq!(of(Family::Bolt).map(|c| (c.get("fyb").unwrap(), c.get("fub").unwrap())).collect::<Vec<_>>(), [(240.0, 400.0), (320.0, 400.0), (300.0, 500.0), (400.0, 500.0), (480.0, 600.0), (640.0, 800.0), (900.0, 1000.0)]);
        assert_eq!(class("8.8").unwrap().source.label(), "EN 1993-1-8:2005, table 3.1");
    }

    #[test]
    fn a_timber_class_is_one_of_them_and_every_name_is_one_class() {
        let c24 = class("c24").unwrap();
        assert_eq!((c24.family, c24.strength(), c24.e(), c24.g(), c24.density(), c24.factor("γM")), (Family::Timber, Some(24.0), Some(11.0e9), Some(0.69e9), Some(350.0), Some(1.3)));
        assert_eq!((class("GL28h").unwrap().factor("γM"), class("GL28h").unwrap().source.label().as_str()), (Some(1.25), "EN 14080, as Project Ocondat has it"));
        let all = classes();
        assert_eq!(Family::ALL.map(|f| of(f).count()), [14, 5, 4, 7, grade::timber_grades().len()]);
        for c in all {
            assert_eq!(all.iter().filter(|o| o.name.eq_ignore_ascii_case(&c.name)).count(), 1, "{}", c.name);
            assert!(std::ptr::eq(class(&c.name).unwrap(), c), "{}", c.name);
            assert!(c.strength().is_some() && c.e().is_some() && !c.factors.is_empty() && c.lines().len() >= 3, "{}", c.name);
        }
        assert_eq!(class("C20/25").unwrap().lines(), ["fck 20 · fck,cube 25 · fcm 28 · fctm 2.2 · fctk,0.05 1.5 · fctk,0.95 2.9 · Ecm 30000 N/mm²", "ρ 2400 kg/m³", "γC 1.5", "EN 1992-1-1:2004, table 3.1"]);
    }

    #[test]
    fn a_material_names_its_class_and_takes_it_to_a_file() {
        let library: Library = crate::ocondat::library();
        let of = |key: &str| library.material(key).unwrap_or_else(|| panic!("{key}"));
        assert_eq!(of("concrete-c20-25").class().map(|c| c.name.as_str()), Some("C20/25"));
        assert_eq!(of("steel-s235jr2").class().map(|c| c.name.as_str()), Some("S235"));
        assert_eq!(of("steel-8-8").class().map(|c| c.family), Some(Family::Bolt));
        assert_eq!(of("spruce-c18").class().map(|c| c.family), Some(Family::Timber));
        assert!(of("concrete").class().is_none());
        // which graded materials of Ocondat name no class of a standard
        let none: Vec<&str> = library.materials.iter().filter(|m| m.grade.is_some() && m.class().is_none()).map(|m| m.grade.as_deref().unwrap()).collect();
        assert!(none.contains(&"C25/35") && !none.contains(&"S355") && !none.contains(&"B500"), "{none:?}");
        // in a file the material's node has the class and IFC's own properties, in pascal
        let node = crate::ifcx::material_node(of("steel-s355"));
        let a = &node["attributes"];
        assert_eq!((a[STRENGTH_CLASS]["name"].as_str(), a[STRENGTH_CLASS]["family"].as_str(), a[STRENGTH_CLASS]["source"]["standard"].as_str(), a[STRENGTH_CLASS]["source"]["year"].as_u64()), (Some("S355"), Some("steel"), Some("EN 1993-1-1"), Some(2005)));
        assert_eq!((a["bsi::ifc::prop::YieldStress"].as_f64(), a["bsi::ifc::prop::UltimateStress"].as_f64(), a["bsi::ifc::prop::YoungModulus"].as_f64(), a["bsi::ifc::prop::MassDensity"].as_f64()), (Some(355.0e6), Some(490.0e6), Some(210.0e9), Some(7850.0)));
        assert_eq!(crate::ifcx::material_node(of("concrete-c20-25"))["attributes"]["bsi::ifc::prop::CompressiveStrength"].as_f64(), Some(20.0e6));
        assert!(crate::ifcx::material_node(of("concrete"))["attributes"].get(STRENGTH_CLASS).is_none());
        // and the file reads back as the library it was
        let file = crate::ifcx::file(&library, "test", "test", "2026-11-23T00:00:00Z");
        assert_eq!(crate::ifcx::read(&file).unwrap().materials, library.materials);
    }
}
