//! The 2D components of Project Ocondat: the catalogue of what a plan, a section and a detail
//! can place, each categorised and on its layer.
//!
//! The catalogue (`data/ocondat-components.json`, see the file's `source` and `licence`) is
//! made from Ocondat's own index by `tools/ocondat-components/extract.py`, which is run again
//! when Ocondat changes; it is compiled in, so nothing is read at run time.
//!
//! - Only the components of the maker `generic` are taken: Ocondat gives those the LGPL-3.0.
//!   What a maker supplied (Halfen, ArcelorMittal) and the community's sets are counted
//!   ([`not_taken`]) and wait until their terms are checked
//!   (`docs/library/ocondat-inventory.md`).
//! - A component is of a series, which gives it its IFC class, its predefined type and its
//!   NL-SfB code ([`Series`]). Its kind ([`Kind`]) is a detail for the series of details, else
//!   a component.
//! - Its layer ([`Item::layer`]) follows the form Ocondat's own drawings use
//!   (`A-DETL-GENF`: discipline, `DETL`, a minor part), with the NL-SfB code as the minor
//!   part: `S-DETL-28` for steelwork, `A-DETL-31` for windows and doors, `P-DETL-52` for
//!   rainwater drainage. The suite has no layer standard of its own yet; this rule is the
//!   one place to change when it has.
//! - The source and the licence stand with every item ([`Item::source`], [`LICENCE`]).
//!
//! Not yet: the drawings themselves. The catalogue points at a component's DXF in Ocondat; a
//! view cannot place one until the lines are read into the library.

use crate::material::Names;
use crate::{nlsfb, uuid, Lang};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::OnceLock;

const CATALOGUE: &str = include_str!("../data/ocondat-components.json");

/// The attribute of a component's node with the component itself.
pub const COMPONENT: &str = "openaec::component";
/// The licence Ocondat gives its generic components.
pub const LICENCE: &str = "LGPL-3.0";
/// Who made the components of the catalogue: no maker.
pub const MAKER: &str = "generic";
/// Where a component's drawing is in Ocondat, before its series and its own path.
const ROOT: &str = "DXF Library/components/Generic";

/// What an item of a library is: the kinds a thing of Ocondat is sorted into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Material,
    Profile,
    BuildUp,
    /// A part drawn in 2D: an anchor, a frame, a floor slab's section.
    Component,
    /// A detail: several parts as they meet.
    Detail,
    Symbol,
}

/// A name in Dutch and English, as Ocondat's index gives it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Two {
    nl: String,
    en: String,
}

#[derive(Deserialize)]
struct RawSeries {
    name: String,
    names: Two,
    class: String,
    predefined: String,
    nlsfb: String,
    standard: String,
    components: Vec<Component>,
}

#[derive(Deserialize)]
struct File {
    not_taken: std::collections::BTreeMap<String, usize>,
    series: Vec<RawSeries>,
}

/// A series of components: what its components have in common.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    /// Its name in Ocondat, which is its folder there ("Ramen-en-deuren").
    pub name: String,
    /// What it holds, in the library's languages (Ocondat names it in Dutch and English).
    pub names: Names,
    /// The IFC class of its components ("IfcWindow") and their predefined type ("WINDOW").
    pub class: String,
    pub predefined: String,
    /// Their NL-SfB code, two digits ("31").
    pub nlsfb: String,
    /// The standard Ocondat names for them ("NEN-EN 14351-1"); empty: none.
    pub standard: String,
    pub components: Vec<Component>,
}

/// A component: one drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    /// Stable, as Ocondat's index has it ("gen-betonanker-m10").
    pub id: String,
    pub name: String,
    /// The width and the height of its drawing, millimetres.
    pub size: [f64; 2],
    /// Its DXF, under its series' folder in Ocondat.
    pub dxf: String,
}

/// A component with its series: what is known of one item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Item<'a> {
    pub series: &'a Series,
    pub component: &'a Component,
}

struct Catalogue {
    series: Vec<Series>,
    not_taken: Vec<(String, usize)>,
}

fn catalogue() -> &'static Catalogue {
    static CATALOGUE_READ: OnceLock<Catalogue> = OnceLock::new();
    CATALOGUE_READ.get_or_init(|| {
        let file: File = serde_json::from_str(CATALOGUE).expect("the component catalogue is compiled in");
        let series = file
            .series
            .into_iter()
            .map(|s| Series { names: Names::new(&s.names.nl, &s.names.en, &s.names.en, &s.names.en), name: s.name, class: s.class, predefined: s.predefined, nlsfb: s.nlsfb, standard: s.standard, components: s.components })
            .collect();
        Catalogue { series, not_taken: file.not_taken.into_iter().collect() }
    })
}

/// The series of the catalogue, by their NL-SfB code.
pub fn series() -> &'static [Series] {
    &catalogue().series
}

/// Every component of the catalogue with its series.
pub fn items() -> impl Iterator<Item = Item<'static>> {
    series().iter().flat_map(|series| series.components.iter().map(move |component| Item { series, component }))
}

/// The component with `id`.
pub fn item(id: &str) -> Option<Item<'static>> {
    items().find(|item| item.component.id == id)
}

/// How many components of Ocondat's index are not taken, by who made them: their terms are
/// not checked yet.
pub fn not_taken() -> &'static [(String, usize)] {
    &catalogue().not_taken
}

impl Series {
    pub fn title(&self, lang: Lang) -> &str {
        self.names.get(lang)
    }

    /// What its components are: details, or components.
    pub fn kind(&self) -> Kind {
        if self.name.to_lowercase().contains("details") { Kind::Detail } else { Kind::Component }
    }

    /// The discipline whose layers its components are drawn on: `S` for what bears (members,
    /// piles, fasteners), `P` for pipes, `E` for cables, `M` for ducts, else `A`.
    pub fn discipline(&self) -> char {
        match self.class.as_str() {
            "IfcMember" if self.predefined == "MEMBER" => 'S',
            "IfcPile" | "IfcMechanicalFastener" | "IfcBeam" | "IfcColumn" | "IfcFooting" | "IfcReinforcingBar" => 'S',
            "IfcPipeSegment" | "IfcPipeFitting" | "IfcSanitaryTerminal" => 'P',
            "IfcCableCarrierSegment" | "IfcCableSegment" | "IfcElectricAppliance" => 'E',
            "IfcDuctSegment" | "IfcDuctFitting" => 'M',
            _ => 'A',
        }
    }

    /// The layer its components are drawn on: its discipline, `DETL` and its NL-SfB code.
    pub fn layer(&self) -> String {
        format!("{}-DETL-{}", self.discipline(), self.nlsfb)
    }
}

impl Item<'_> {
    pub fn kind(&self) -> Kind {
        self.series.kind()
    }

    pub fn layer(&self) -> String {
        self.series.layer()
    }

    /// Where its drawing is in Ocondat.
    pub fn source(&self) -> String {
        format!("{ROOT}/{}/{}", self.series.name, self.component.dxf)
    }

    /// Its node's path.
    pub fn path(&self) -> String {
        uuid(&format!("openaec-library/component/{}", self.component.id))
    }

    /// The component as a node of IFCX: its class, its predefined type, its NL-SfB code, and
    /// under [`COMPONENT`] its kind, its maker, its layer, its size, its source and its
    /// licence.
    pub fn node(&self) -> Value {
        json!({
            "path": self.path(),
            "attributes": {
                "bsi::ifc::class": {"code": self.series.class},
                "bsi::ifc::prop::Name": self.component.name,
                "bsi::ifc::prop::PredefinedType": self.series.predefined,
                nlsfb::CLASS: nlsfb::class_value(&self.series.nlsfb),
                COMPONENT: {
                    "id": self.component.id,
                    "kind": self.kind(),
                    "maker": MAKER,
                    "series": self.series.name,
                    "layer": self.layer(),
                    "size": self.component.size,
                    "source": self.source(),
                    "licence": LICENCE,
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_has_the_generic_components_by_series() {
        assert_eq!((series().len(), items().count()), (25, 1307));
        // every component has an id of its own, a name and a drawing; a series a class and a code
        let mut ids: Vec<&str> = items().map(|i| i.component.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 1307);
        for s in series() {
            assert!(s.class.starts_with("Ifc") && s.nlsfb.len() == 2 && s.nlsfb.chars().all(|c| c.is_ascii_digit()) && !s.components.is_empty(), "{}", s.name);
            assert!(Lang::ALL.iter().all(|lang| !s.title(*lang).is_empty()), "{}", s.name);
        }
        assert!(items().all(|i| i.component.id.starts_with("gen-") && !i.component.name.is_empty() && i.component.dxf.ends_with(".dxf") && i.component.size.iter().all(|v| v.is_finite() && *v >= 0.0)));
        // the series stand by their code
        assert!(series().windows(2).all(|pair| pair[0].nlsfb <= pair[1].nlsfb));
        let steel = series().iter().find(|s| s.name == "Staalconstructie").expect("steelwork");
        assert_eq!((steel.components.len(), steel.class.as_str(), steel.nlsfb.as_str(), steel.standard.as_str(), steel.title(Lang::En)), (490, "IfcMember", "28", "NEN-EN 1993", "Steel Construction"));
        // what the makers and the community supplied is counted, not taken
        assert_eq!(not_taken().iter().map(|(maker, count)| (maker.as_str(), *count)).collect::<Vec<_>>(), [("ArcelorMittal", 359), ("Community/DXF-library", 2256), ("Community/dxfBlocks", 305), ("Halfen", 18606)]);
    }

    #[test]
    fn a_component_is_categorised_and_on_its_layer() {
        let anchor = item("gen-betonanker-m10").expect("a concrete anchor");
        assert_eq!((anchor.series.name.as_str(), anchor.kind(), anchor.layer().as_str(), anchor.component.size), ("Basismaterialen", Kind::Component, "S-DETL-00", [145.0, 10.0]));
        assert_eq!(anchor.source(), "DXF Library/components/Generic/Basismaterialen/betonanker-M10/betonanker-m10.dxf");
        let layer = |name: &str| series().iter().find(|s| s.name == name).map(Series::layer);
        assert_eq!((layer("Staalconstructie").as_deref(), layer("Ramen-en-deuren").as_deref(), layer("Hemelwaterafvoer").as_deref(), layer("Afwerkprofielen").as_deref()), (Some("S-DETL-28"), Some("A-DETL-31"), Some("P-DETL-52"), Some("A-DETL-42")));
        // the roof details are details, the rest components
        assert_eq!(series().iter().filter(|s| s.kind() == Kind::Detail).map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Dakdetails"]);
        assert!(item("gen-nothing").is_none());
    }

    #[test]
    fn a_component_is_a_node_with_its_source_and_its_licence() {
        let anchor = item("gen-betonanker-m10").unwrap();
        let node = anchor.node();
        assert_eq!(node["path"], anchor.path());
        let attributes = &node["attributes"];
        assert_eq!((attributes["bsi::ifc::class"]["code"].as_str(), attributes["bsi::ifc::prop::PredefinedType"].as_str(), attributes[nlsfb::CLASS]["code"].as_str()), (Some("IfcMechanicalFastener"), Some("USERDEFINED"), Some("00")));
        let own = &attributes[COMPONENT];
        assert_eq!((own["kind"].as_str(), own["maker"].as_str(), own["layer"].as_str(), own["licence"].as_str()), (Some("component"), Some("generic"), Some("S-DETL-00"), Some("LGPL-3.0")));
        assert_eq!(own["source"].as_str(), Some(anchor.source().as_str()));
        // paths are stable and each its own
        let mut paths: Vec<String> = items().map(|i| i.path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), 1307);
    }
}
