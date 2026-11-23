//! What a Dutch delivery asks of a type beside its name and its material: the BIM basis ILS.
//!
//! The BIM basis ILS is the Dutch sector's short list of what every model that is handed over
//! holds. Of a type it asks the right IFC class, a name, an NL-SfB code of four digits, its
//! material, and three properties in IFC's own property sets: whether it bears (`LoadBearing`),
//! whether it is external (`IsExternal`) and its fire rating (`FireRating`).
//!
//! A build-up says the three itself ([`Delivery`], in [`BuildUp::delivery`]) or leaves them to
//! its code: an NL-SfB code of the groups 21 (external walls) and 27 (roofs) is external; a code
//! whose first digit after the point is 1 does not bear and one whose is 2 does (21.1, 22.2),
//! as table 1 of NL-SfB splits its groups. A fire rating follows from nothing: it is given.
//!
//! [`BuildUp::missing`] says what a build-up still lacks for a delivery.
//!
//! Written from the ILS as it is commonly cited, not checked against its text here.
//!
//! A linear type does the same ([`LinearType::delivery`]); its role says whether it bears where
//! its code does not: a column, a beam and a pile do, a mullion, a skirting board and the runs of
//! the services do not. Of a run of the services a delivery asks no fire rating here.
//!
//! Not yet: system and object types; the ILS's other points (storeys, placing, doubles),
//! which are of a model and not of a library.

use crate::buildup::{BuildUp, Role};
use crate::material::Material;
use crate::profile::{LinearRole, LinearType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// What a delivery asks of a build-up, where the build-up says it itself.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Delivery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_bearing: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_external: Option<bool>,
    /// As the delivery writes it ("60", "REI 60", "EW 30").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_rating: Option<String>,
}

impl Delivery {
    pub fn is_empty(&self) -> bool {
        *self == Delivery::default()
    }
}

/// What a build-up lacks for a delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Missing {
    /// Its NL-SfB code has no four digits.
    FourDigits,
    /// Neither it nor its code says whether it bears.
    LoadBearing,
    FireRating,
}

impl Missing {
    /// In Dutch and in English.
    pub fn names(self) -> (&'static str, &'static str) {
        match self {
            Missing::FourDigits => ("NL-SfB-code van vier cijfers", "NL-SfB code of four digits"),
            Missing::LoadBearing => ("dragend of niet", "loadbearing or not"),
            Missing::FireRating => ("brandwerendheid", "fire rating"),
        }
    }
}

/// Whether the code says its element bears: the first digit after the point, 1 (not) or 2.
fn bears(code: &str) -> Option<bool> {
    match code.split_once('.')?.1.chars().next()? {
        '1' => Some(false),
        '2' => Some(true),
        _ => None,
    }
}

impl BuildUp {
    /// Whether it is external: what it says itself, else what its code says (an external wall,
    /// a roof).
    pub fn is_external(&self, materials: &[Material]) -> bool {
        self.delivery.is_external.unwrap_or_else(|| {
            let code = self.nlsfb(materials);
            code.starts_with("21") || code.starts_with("27") || (self.code.is_none() && self.role == Role::Roof)
        })
    }

    /// Whether it bears: what it says itself, else what its code says; none when neither does.
    pub fn load_bearing(&self, materials: &[Material]) -> Option<bool> {
        self.delivery.load_bearing.or_else(|| bears(&self.nlsfb(materials)))
    }

    /// IFC's own properties of it for a delivery, by their name in `Pset_WallCommon`,
    /// `Pset_SlabCommon`, `Pset_RoofCommon` and `Pset_CoveringCommon`.
    pub fn delivery_properties(&self, materials: &[Material]) -> Vec<(&'static str, Value)> {
        let mut out = vec![("IsExternal", json!(self.is_external(materials)))];
        out.extend(self.load_bearing(materials).map(|bears| ("LoadBearing", json!(bears))));
        out.extend(self.delivery.fire_rating.as_deref().filter(|rating| !rating.trim().is_empty()).map(|rating| ("FireRating", json!(rating))));
        out
    }

    /// What it still lacks for a delivery.
    pub fn missing(&self, materials: &[Material]) -> Vec<Missing> {
        let mut out = Vec::new();
        if self.nlsfb(materials).chars().filter(char::is_ascii_digit).count() < 4 {
            out.push(Missing::FourDigits);
        }
        if self.load_bearing(materials).is_none() {
            out.push(Missing::LoadBearing);
        }
        if self.delivery.fire_rating.as_deref().is_none_or(|rating| rating.trim().is_empty()) {
            out.push(Missing::FireRating);
        }
        out
    }
}

/// What a library holds of one kind of type, and how much of it a delivery still lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stock {
    /// What it counts ("walls", "beams"), in English, as data.
    pub of: String,
    pub count: usize,
    /// How many of them are complete for a delivery.
    pub complete: usize,
    /// How many lack a code of four digits, whether they bear, and a fire rating.
    pub lacking: [usize; 3],
}

/// The stock of a library's build-ups by role and its linear types by role: how many, how many
/// complete for a delivery, and what the others lack. A role it has none of is listed with 0:
/// that is what a library still misses.
pub fn stock(library: &crate::Library) -> Vec<Stock> {
    let count = |of: &str, all: Vec<Vec<Missing>>| Stock {
        of: of.to_string(),
        count: all.len(),
        complete: all.iter().filter(|missing| missing.is_empty()).count(),
        lacking: [Missing::FourDigits, Missing::LoadBearing, Missing::FireRating].map(|what| all.iter().filter(|missing| missing.contains(&what)).count()),
    };
    let mut out: Vec<Stock> = Role::ALL.into_iter().map(|role| count(&format!("{} build-ups", role.name().to_lowercase()), library.buildups_of(role).map(|b| b.missing(&library.materials)).collect())).collect();
    for (role, name) in [
        (LinearRole::Column, "columns"),
        (LinearRole::Beam, "beams"),
        (LinearRole::Member, "members"),
        (LinearRole::Pile, "piles"),
        (LinearRole::Mullion, "mullions"),
        (LinearRole::Skirting, "skirting boards"),
        (LinearRole::Pipe, "pipes"),
        (LinearRole::Duct, "ducts"),
        (LinearRole::CableTray, "cable trays"),
    ] {
        out.push(count(name, library.linear_types.iter().filter(|t| t.role == role).map(LinearType::missing).collect()));
    }
    out
}

impl LinearType {
    /// Whether it is of the services: a pipe, a duct, a cable tray.
    fn of_services(&self) -> bool {
        matches!(self.role, LinearRole::Pipe | LinearRole::Duct | LinearRole::CableTray)
    }

    /// Whether it is external: what it says itself, else what its code says (the groups 21, 27,
    /// 31 and 41 of NL-SfB are of the outside).
    pub fn is_external(&self) -> bool {
        self.delivery.is_external.unwrap_or_else(|| ["21", "27", "31", "41"].iter().any(|group| self.nlsfb().starts_with(group)))
    }

    /// Whether it bears: what it says itself, else what its code says, else what its role says;
    /// none for a member whose code does not say.
    pub fn load_bearing(&self) -> Option<bool> {
        self.delivery.load_bearing.or_else(|| bears(&self.nlsfb())).or(match self.role {
            LinearRole::Column | LinearRole::Beam | LinearRole::Pile => Some(true),
            LinearRole::Member => None,
            LinearRole::Mullion | LinearRole::Skirting | LinearRole::Pipe | LinearRole::Duct | LinearRole::CableTray => Some(false),
        })
    }

    /// IFC's own properties of it for a delivery (`Pset_BeamCommon`, `Pset_ColumnCommon` and
    /// their likes).
    pub fn delivery_properties(&self) -> Vec<(&'static str, Value)> {
        let mut out = vec![("IsExternal", json!(self.is_external()))];
        out.extend(self.load_bearing().map(|bears| ("LoadBearing", json!(bears))));
        out.extend(self.delivery.fire_rating.as_deref().filter(|rating| !rating.trim().is_empty()).map(|rating| ("FireRating", json!(rating))));
        out
    }

    /// What it still lacks for a delivery.
    pub fn missing(&self) -> Vec<Missing> {
        let mut out = Vec::new();
        if self.nlsfb().chars().filter(char::is_ascii_digit).count() < 4 {
            out.push(Missing::FourDigits);
        }
        if self.load_bearing().is_none() {
            out.push(Missing::LoadBearing);
        }
        if !self.of_services() && self.delivery.fire_rating.as_deref().is_none_or(|rating| rating.trim().is_empty()) {
            out.push(Missing::FireRating);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Library;

    #[test]
    fn a_build_up_says_what_a_delivery_asks_or_leaves_it_to_its_code() {
        let library = Library::starter();
        let m = &library.materials;
        let wall = |code: Option<&str>| BuildUp { code: code.map(str::to_string), ..library.buildups_of(Role::Wall).next().unwrap().clone() };
        // by its code: an external wall is external; the digit after the point says whether it bears
        assert_eq!((wall(Some("21.22")).is_external(m), wall(Some("22.11")).is_external(m), wall(Some("27.2")).is_external(m)), (true, false, true));
        assert_eq!((wall(Some("21.22")).load_bearing(m), wall(Some("22.11")).load_bearing(m), wall(Some("22")).load_bearing(m), wall(Some("22.3")).load_bearing(m)), (Some(true), Some(false), None, None));
        // a roof without a code of its own is external
        let roof = library.buildups_of(Role::Roof).next().unwrap().clone();
        assert!(BuildUp { code: None, ..roof }.is_external(m));
        // what it says itself wins
        let own = BuildUp { delivery: Delivery { load_bearing: Some(true), is_external: Some(true), fire_rating: Some("REI 60".into()) }, ..wall(Some("22.11")) };
        assert_eq!((own.is_external(m), own.load_bearing(m)), (true, Some(true)));
        assert_eq!(own.delivery_properties(m), [("IsExternal", json!(true)), ("LoadBearing", json!(true)), ("FireRating", json!("REI 60"))]);
        assert_eq!(wall(Some("22")).delivery_properties(m), [("IsExternal", json!(false))]);
    }

    #[test]
    fn a_build_up_says_what_it_lacks_for_a_delivery() {
        let library = Library::starter();
        let m = &library.materials;
        let wall = library.buildups_of(Role::Wall).next().unwrap().clone();
        assert_eq!(BuildUp { code: Some("22".into()), ..wall.clone() }.missing(m), [Missing::FourDigits, Missing::LoadBearing, Missing::FireRating]);
        assert_eq!(BuildUp { code: Some("22.1".into()), ..wall.clone() }.missing(m), [Missing::FourDigits, Missing::FireRating]);
        let done = BuildUp { code: Some("22.21".into()), delivery: Delivery { fire_rating: Some("60".into()), ..Delivery::default() }, ..wall.clone() };
        assert_eq!(done.missing(m), []);
        assert_eq!(BuildUp { delivery: Delivery { fire_rating: Some(" ".into()), ..Delivery::default() }, ..done }.missing(m), [Missing::FireRating]);
        for missing in [Missing::FourDigits, Missing::LoadBearing, Missing::FireRating] {
            assert!(!missing.names().0.is_empty() && !missing.names().1.is_empty());
        }
    }

    #[test]
    fn what_a_delivery_asks_goes_to_a_file_and_back() {
        let mut library = Library::starter();
        let id = library.buildups[0].id.clone();
        let before = serde_json::to_value(&library.buildups[0]).unwrap();
        assert!(before.get("delivery").is_none(), "a build-up that says nothing writes nothing");
        library.buildups[0].delivery = Delivery { load_bearing: Some(true), is_external: None, fire_rating: Some("REI 60".into()) };
        let file = crate::ifcx::file(&library, "test", "test", "2026-11-23T00:00:00Z");
        let path = crate::ifcx::buildup_path(&id);
        let node = file["data"].as_array().unwrap().iter().find(|n| n["path"] == path.as_str()).unwrap();
        assert_eq!((node["attributes"]["bsi::ifc::prop::LoadBearing"].as_bool(), node["attributes"]["bsi::ifc::prop::FireRating"].as_str(), node["attributes"]["bsi::ifc::prop::IsExternal"].is_boolean()), (Some(true), Some("REI 60"), true));
        let read = crate::ifcx::read(&file).unwrap();
        assert_eq!(read.buildup(&id).map(|b| &b.delivery), Some(&library.buildups[0].delivery));
        // a build-up written before this reads as saying nothing
        assert_eq!(serde_json::from_value::<BuildUp>(before).unwrap().delivery, Delivery::default());
    }
    #[test]
    fn a_linear_type_says_what_a_delivery_asks_or_leaves_it_to_its_role_and_code() {
        let library = Library::starter();
        let of = |role: LinearRole| library.linear_types.iter().find(|t| t.role == role).cloned();
        let beam = of(LinearRole::Beam).expect("a beam");
        // a beam bears by its role, and is of the inside by its code (28)
        assert_eq!((beam.load_bearing(), beam.is_external(), beam.missing()), (Some(true), false, vec![Missing::FourDigits, Missing::FireRating]));
        // its code says more than its role; what it says itself more than both
        let coded = LinearType { code: Some("28.11".into()), ..beam.clone() };
        assert_eq!((coded.load_bearing(), coded.missing()), (Some(false), vec![Missing::FireRating]));
        let own = LinearType { delivery: Delivery { load_bearing: Some(true), is_external: Some(true), fire_rating: Some("R 60".into()) }, ..coded };
        assert_eq!((own.load_bearing(), own.is_external(), own.missing()), (Some(true), true, vec![]));
        assert_eq!(own.delivery_properties(), [("IsExternal", json!(true)), ("LoadBearing", json!(true)), ("FireRating", json!("R 60"))]);
        // a member's code has to say whether it bears; a mullion is of the outside and bears nothing
        let member = LinearType { role: LinearRole::Member, ..beam.clone() };
        assert_eq!((member.load_bearing(), member.missing().contains(&Missing::LoadBearing)), (None, true));
        let mullion = LinearType { role: LinearRole::Mullion, ..beam.clone() };
        assert_eq!((mullion.load_bearing(), mullion.is_external()), (Some(false), true));
        // of a run of the services no fire rating is asked
        let pipe = LinearType { role: LinearRole::Pipe, ..beam.clone() };
        assert_eq!((pipe.load_bearing(), pipe.missing()), (Some(false), vec![Missing::FourDigits]));
        // to a file and back; a type that says nothing writes nothing
        let mut library = library;
        let id = beam.id.clone();
        assert!(serde_json::to_value(&beam).unwrap().get("delivery").is_none());
        library.linear_types.iter_mut().find(|t| t.id == id).unwrap().delivery.fire_rating = Some("R 30".into());
        let file = crate::ifcx::file(&library, "test", "test", "2026-11-23T00:00:00Z");
        let path = crate::ifcx::linear_type_path(&id);
        let node = file["data"].as_array().unwrap().iter().find(|n| n["path"] == path.as_str()).unwrap();
        assert_eq!((node["attributes"]["bsi::ifc::prop::FireRating"].as_str(), node["attributes"]["bsi::ifc::prop::LoadBearing"].as_bool()), (Some("R 30"), Some(true)));
        assert_eq!(crate::ifcx::read(&file).unwrap().linear_type(&id).map(|t| t.delivery.fire_rating.clone()), Some(Some("R 30".to_string())));
    }

    #[test]
    fn the_stock_of_a_library_says_what_it_holds_and_what_a_delivery_lacks() {
        let library = Library::starter();
        let all = stock(&library);
        assert_eq!(all.len(), 13);
        assert_eq!(all.iter().filter(|s| s.of.ends_with("build-ups")).map(|s| s.count).sum::<usize>(), library.buildups.len());
        assert_eq!(all.iter().filter(|s| !s.of.ends_with("build-ups")).map(|s| s.count).sum::<usize>(), library.linear_types.len());
        for s in &all {
            assert!(s.complete <= s.count && s.lacking.iter().all(|n| *n <= s.count), "{}", s.of);
        }
        // nothing of the starter library has a fire rating yet, so nothing is complete
        assert!(all.iter().all(|s| s.complete == 0));
        let walls = all.iter().find(|s| s.of == "wall build-ups").unwrap();
        assert_eq!(walls.lacking[2], walls.count);
        // one made complete is counted as such
        let mut library = library;
        let wall = library.buildups.iter_mut().find(|b| b.role == Role::Wall).unwrap();
        wall.code = Some("22.21".into());
        wall.delivery.fire_rating = Some("60".into());
        assert_eq!(stock(&library).iter().find(|s| s.of == "wall build-ups").map(|s| s.complete), Some(1));
    }

    #[test]
    #[ignore = "prints the stock of the libraries, for docs/library/dutch-library-stock.md"]
    fn print_the_stock() {
        for (name, library) in [("starter", Library::starter()), ("dutch", crate::dutch::library()), ("ocondat", crate::ocondat::library()), ("inb", crate::inb::library())] {
            println!("== {name}: materials {}, build-ups {}, linear {}, systems {}, objects {}, connections {}, services {}, covers {}", library.materials.len(), library.buildups.len(), library.linear_types.len(), library.system_types.len(), library.object_types.len(), library.connection_types.len(), library.service_types.len(), library.cover_types.len());
            for s in stock(&library) {
                println!("{} | {} | {} | {} | {} | {}", s.of, s.count, s.complete, s.lacking[0], s.lacking[1], s.lacking[2]);
            }
        }
    }

}
