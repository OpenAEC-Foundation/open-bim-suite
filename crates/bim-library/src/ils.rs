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
//! Not yet: linear, system and object types; the ILS's other points (storeys, placing, doubles),
//! which are of a model and not of a library.

use crate::buildup::{BuildUp, Role};
use crate::material::Material;
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
}
