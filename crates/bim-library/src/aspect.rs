//! Aspects: what else is known of an item of the library, each a set of properties with a
//! source and a date.
//!
//! An aspect ([`Aspect`]) belongs to one item, named by the path of its node, and is of one
//! kind ([`Kind`]): costs, environmental data, physics, planning, maintenance, the maker's data.
//! An item has at most one aspect of a kind. Its properties are free (a name and a JSON value),
//! so a kind can be filled in without the library changing its shape; a kind with a shape of
//! its own reads them as that ([`crate::cost::Cost`]).
//!
//! In a file an aspect is an attribute on the item's node, `openaec::aspect::<kind>`, written as
//! a node of its own with the item's path: IFCX composes the attributes of nodes of one path, so
//! a supplier's prices can be a layer over a library.
//!
//! `docs/library/aspects.md` lists the kinds, what could supply each, and which come first.

use crate::LibraryError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// What the attribute of an aspect starts with; the kind's key follows.
pub const ASPECT: &str = "openaec::aspect::";

/// A kind of aspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A price per unit, split into what it is made up of ([`crate::cost`]).
    Cost,
    /// Environmental data: an EPD, a profile of the NMD.
    Environment,
    /// Thermal, moisture, sound and fire.
    Physics,
    /// Production rates, for a planning.
    Planning,
    /// Life span and condition.
    Maintenance,
    /// The maker's data: an article number, a GTIN, a link.
    Maker,
}

impl Kind {
    pub const ALL: [Kind; 6] = [Kind::Cost, Kind::Environment, Kind::Physics, Kind::Planning, Kind::Maintenance, Kind::Maker];

    /// Its key, as in its attribute: "cost".
    pub fn key(self) -> &'static str {
        match self {
            Kind::Cost => "cost",
            Kind::Environment => "environment",
            Kind::Physics => "physics",
            Kind::Planning => "planning",
            Kind::Maintenance => "maintenance",
            Kind::Maker => "maker",
        }
    }

    /// Its name in Dutch and in English.
    pub fn names(self) -> (&'static str, &'static str) {
        match self {
            Kind::Cost => ("Kosten", "Costs"),
            Kind::Environment => ("Milieu", "Environment"),
            Kind::Physics => ("Bouwfysica", "Physics"),
            Kind::Planning => ("Planning", "Planning"),
            Kind::Maintenance => ("Onderhoud", "Maintenance"),
            Kind::Maker => ("Fabrikant", "Maker"),
        }
    }

    /// The attribute an aspect of this kind is written as.
    pub fn attribute(self) -> String {
        format!("{ASPECT}{}", self.key())
    }

    /// The kind an attribute names.
    pub fn of_attribute(attribute: &str) -> Option<Kind> {
        let key = attribute.strip_prefix(ASPECT)?;
        Kind::ALL.into_iter().find(|kind| kind.key() == key)
    }
}

/// What is the same of every aspect: where it is from and of when.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Origin {
    /// Who or what says so ("an offer of 12 March", a price book, an EPD's number).
    pub source: String,
    /// The day it held, `YYYY-MM-DD`.
    pub date: String,
    /// Where it holds ("NL", "NL-ZH"); none: anywhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// One aspect of one item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aspect {
    /// The path of the item's node ([`crate::ifcx::material_path`] and its likes).
    pub item: String,
    pub kind: Kind,
    pub origin: Origin,
    pub properties: BTreeMap<String, Value>,
}

/// Whether `date` is a day as `YYYY-MM-DD`.
pub fn is_day(date: &str) -> bool {
    let parts: Vec<&str> = date.split('-').collect();
    let [y, m, d] = parts[..] else { return false };
    let number = |s: &str, len: usize| (s.len() == len && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse::<u32>().ok()).flatten();
    matches!((number(y, 4), number(m, 2), number(d, 2)), (Some(_), Some(1..=12), Some(1..=31)))
}

impl Aspect {
    /// Refused without an item, a source or a day.
    pub fn check(&self) -> Result<(), LibraryError> {
        if self.item.trim().is_empty() {
            return Err(LibraryError::NoKey);
        }
        if self.origin.source.trim().is_empty() {
            return Err(LibraryError::BadAspect("a source".into()));
        }
        if !is_day(&self.origin.date) {
            return Err(LibraryError::BadAspect("a date as YYYY-MM-DD".into()));
        }
        Ok(())
    }

    /// The aspect as the value of its attribute: its origin, then its properties.
    pub fn value(&self) -> Value {
        let mut map = match serde_json::to_value(&self.origin) {
            Ok(Value::Object(map)) => map,
            _ => Map::new(),
        };
        for (name, value) in &self.properties {
            map.entry(name.clone()).or_insert_with(|| value.clone());
        }
        Value::Object(map)
    }

    /// The aspect of `kind` of the item at `item`, read from the value of its attribute.
    pub fn read(item: &str, kind: Kind, value: &Value) -> Result<Aspect, LibraryError> {
        let unreadable = |what: String| LibraryError::Unreadable(format!("{} of {item}: {what}", kind.attribute()));
        let origin: Origin = serde_json::from_value(value.clone()).map_err(|e| unreadable(e.to_string()))?;
        let map = value.as_object().ok_or_else(|| unreadable("no object".into()))?;
        let properties = map.iter().filter(|(name, _)| !matches!(name.as_str(), "source" | "date" | "region")).map(|(name, value)| (name.clone(), value.clone())).collect();
        Ok(Aspect { item: item.into(), kind, origin, properties })
    }

    /// The aspect as a node of IFCX: the item's path, and the aspect's attribute.
    pub fn node(&self) -> Value {
        let mut attributes = Map::new();
        attributes.insert(self.kind.attribute(), self.value());
        json!({"path": self.item, "attributes": attributes})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Library;

    fn maker(item: &str) -> Aspect {
        Aspect {
            item: item.into(),
            kind: Kind::Maker,
            origin: Origin { source: "the maker's catalogue".into(), date: "2026-11-23".into(), region: None },
            properties: BTreeMap::from([("article".to_string(), json!("A-1001")), ("gtin".to_string(), json!("8712345678906"))]),
        }
    }

    #[test]
    fn a_kind_has_its_attribute() {
        assert_eq!(Kind::ALL.map(Kind::key), ["cost", "environment", "physics", "planning", "maintenance", "maker"]);
        assert_eq!(Kind::Cost.attribute(), "openaec::aspect::cost");
        for kind in Kind::ALL {
            assert_eq!(Kind::of_attribute(&kind.attribute()), Some(kind));
            assert!(!kind.names().0.is_empty() && !kind.names().1.is_empty());
        }
        assert_eq!((Kind::of_attribute("openaec::aspect::colour"), Kind::of_attribute("openaec::material")), (None, None));
        assert!(is_day("2026-11-23") && !is_day("2026-13-01") && !is_day("23-11-2026") && !is_day("2026-1-1") && !is_day(""));
    }

    #[test]
    fn an_item_has_one_aspect_of_a_kind_with_a_source_and_a_date() {
        let mut library = Library::starter();
        let steel = crate::ifcx::material_path("steel");
        library.set_aspect(maker(&steel)).unwrap();
        assert_eq!(library.aspect(&steel, Kind::Maker).map(|a| a.properties["article"].clone()), Some(json!("A-1001")));
        assert_eq!(library.aspects_of(&steel).count(), 1);
        // a second of the same kind takes the first's place
        let mut newer = maker(&steel);
        newer.properties.insert("article".into(), json!("A-2002"));
        library.set_aspect(newer).unwrap();
        assert_eq!((library.aspects.len(), library.aspect(&steel, Kind::Maker).unwrap().properties["article"].clone()), (1, json!("A-2002")));
        // without a source, a day, or an item the library has: refused, and nothing changes
        let mut bad = maker(&steel);
        bad.origin.source = " ".into();
        assert!(matches!(library.set_aspect(bad), Err(LibraryError::BadAspect(_))));
        let mut bad = maker(&steel);
        bad.origin.date = "today".into();
        assert!(matches!(library.set_aspect(bad), Err(LibraryError::BadAspect(_))));
        assert!(matches!(library.set_aspect(maker("no-such-path")), Err(LibraryError::BadAspect(_))));
        assert_eq!(library.aspects.len(), 1);
        // taken away
        assert!(library.remove_aspect(&steel, Kind::Maker) && !library.remove_aspect(&steel, Kind::Maker));
        assert!(library.aspects.is_empty());
    }

    #[test]
    fn an_aspect_is_an_attribute_on_its_items_node_in_a_file() {
        let mut library = Library::starter();
        let steel = crate::ifcx::material_path("steel");
        let wall = crate::ifcx::buildup_path(&library.buildups[0].id);
        library.set_aspect(maker(&steel)).unwrap();
        let mut planning = maker(&wall);
        planning.kind = Kind::Planning;
        planning.origin.region = Some("NL".into());
        planning.properties = BTreeMap::from([("rate".to_string(), json!(1.25))]);
        library.set_aspect(planning).unwrap();
        let file = crate::ifcx::file(&library, "test", "test", "2026-11-23T00:00:00Z");
        // a node of its own with the item's path
        let nodes = file["data"].as_array().unwrap();
        let own: Vec<&Value> = nodes.iter().filter(|n| n["path"] == steel.as_str()).collect();
        assert_eq!(own.len(), 2);
        assert_eq!(own[1]["attributes"]["openaec::aspect::maker"], json!({"source": "the maker's catalogue", "date": "2026-11-23", "article": "A-1001", "gtin": "8712345678906"}));
        // and back
        let read = crate::ifcx::read(&file).unwrap();
        assert_eq!(read.aspects, library.aspects);
        assert_eq!(read.aspect(&wall, Kind::Planning).map(|a| (a.origin.region.as_deref(), a.properties["rate"].as_f64())), Some((Some("NL"), Some(1.25))));
        assert_eq!(read.materials, library.materials);
        // merged into another library, an aspect comes with the item it is of
        let mut other = Library::starter();
        other.merge(&library);
        assert_eq!(other.aspects.len(), 2);
        let mut bare = Library::default();
        bare.merge(&Library { aspects: library.aspects.clone(), ..Library::default() });
        assert!(bare.aspects.is_empty(), "no item, no aspect");
    }
}
