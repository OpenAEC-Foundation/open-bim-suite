//! Costs: the price of an item of the library per unit, split into what it is made up of.
//!
//! A cost ([`Cost`]) is the costs aspect of an item ([`crate::aspect`]): a price per unit of
//! material, labour, equipment, subcontractor and other, in a currency, with the source, the
//! day and the region of its aspect, and the index it was at on that day so it can be brought
//! to today ([`Cost::indexed`]).
//!
//! The terms are Open Calc Studio's, so an estimate reads the library without conversion: its
//! unit codes ([`Unit`]: "st", "m", "m²", "m³", "uur"), its resource types ([`Resource`]:
//! "materiaal", "arbeid", "materieel", "onderaannemer", "overig") and its catalogue of articles
//! ([`catalogue`]: `{id, version, currency, resources}` with `defaultUnitPrice`,
//! `referenceUnitPrice`, `priceDate`, `source`). An article there is one resource of one type,
//! so a cost split in three is three articles. A price that is not marked verified goes out as
//! a reference price, which Open Calc Studio shows and does not copy into an estimate.
//!
//! Not yet: no prices are in the library (none is free to pass on), VAT is not recorded, and
//! labour is a price, not hours at a rate.

use crate::aspect::{Aspect, Kind, Origin};
use crate::{Lang, Library, LibraryError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// A unit a price is per, by Open Calc Studio's code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Unit {
    /// A piece.
    #[default]
    #[serde(rename = "st")]
    Piece,
    #[serde(rename = "m", alias = "m1")]
    Metre,
    #[serde(rename = "m²", alias = "m2")]
    SquareMetre,
    #[serde(rename = "m³", alias = "m3")]
    CubicMetre,
    #[serde(rename = "kg")]
    Kilogram,
    #[serde(rename = "ton")]
    Tonne,
    #[serde(rename = "uur")]
    Hour,
    #[serde(rename = "dgn", alias = "dag")]
    Day,
    /// A lump sum.
    #[serde(rename = "ls")]
    LumpSum,
}

impl Unit {
    pub const ALL: [Unit; 9] = [Unit::Piece, Unit::Metre, Unit::SquareMetre, Unit::CubicMetre, Unit::Kilogram, Unit::Tonne, Unit::Hour, Unit::Day, Unit::LumpSum];

    /// Its code: "m²".
    pub fn code(self) -> &'static str {
        match self {
            Unit::Piece => "st",
            Unit::Metre => "m",
            Unit::SquareMetre => "m²",
            Unit::CubicMetre => "m³",
            Unit::Kilogram => "kg",
            Unit::Tonne => "ton",
            Unit::Hour => "uur",
            Unit::Day => "dgn",
            Unit::LumpSum => "ls",
        }
    }
}

/// What a part of a price is of, by Open Calc Studio's resource type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    #[serde(rename = "materiaal")]
    Material,
    #[serde(rename = "arbeid")]
    Labour,
    #[serde(rename = "materieel")]
    Equipment,
    #[serde(rename = "onderaannemer")]
    Subcontractor,
    #[serde(rename = "overig")]
    Other,
}

impl Resource {
    pub const ALL: [Resource; 5] = [Resource::Material, Resource::Labour, Resource::Equipment, Resource::Subcontractor, Resource::Other];

    /// Its code: "materiaal".
    pub fn code(self) -> &'static str {
        match self {
            Resource::Material => "materiaal",
            Resource::Labour => "arbeid",
            Resource::Equipment => "materieel",
            Resource::Subcontractor => "onderaannemer",
            Resource::Other => "overig",
        }
    }

    /// Its name in Dutch and in English.
    pub fn names(self) -> (&'static str, &'static str) {
        match self {
            Resource::Material => ("materiaal", "material"),
            Resource::Labour => ("arbeid", "labour"),
            Resource::Equipment => ("materieel", "equipment"),
            Resource::Subcontractor => ("onderaannemer", "subcontractor"),
            Resource::Other => ("overig", "other"),
        }
    }
}

/// The index a price was at on its day: which index, and its value then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Index {
    /// Which ("BDB bouwkosten", "CBS inputprijsindex").
    pub name: String,
    /// Its value on the day of the price.
    pub value: f64,
}

/// The price of an item per unit.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Cost {
    pub unit: Unit,
    /// "EUR".
    pub currency: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub material: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub labour: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub equipment: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub subcontractor: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub other: f64,
    /// Whether the price was checked against its source: only then an estimate may take it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub verified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Index>,
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

impl Cost {
    /// The part of the price that is of `resource`.
    pub fn part(&self, resource: Resource) -> f64 {
        match resource {
            Resource::Material => self.material,
            Resource::Labour => self.labour,
            Resource::Equipment => self.equipment,
            Resource::Subcontractor => self.subcontractor,
            Resource::Other => self.other,
        }
    }

    /// The parts it has: those that are not nothing.
    pub fn parts(&self) -> impl Iterator<Item = (Resource, f64)> + '_ {
        Resource::ALL.into_iter().map(|r| (r, self.part(r))).filter(|(_, price)| *price != 0.0)
    }

    /// The price per unit: its parts together.
    pub fn unit_price(&self) -> f64 {
        Resource::ALL.into_iter().map(|r| self.part(r)).sum()
    }

    /// The price per unit brought to a day on which its index stands at `now`; none without an
    /// index of its own.
    pub fn indexed(&self, now: f64) -> Option<f64> {
        let index = self.index.as_ref()?;
        (index.value > 0.0 && now > 0.0).then(|| self.unit_price() * now / index.value)
    }

    /// Refused without a currency, with a part below nothing or no number, with nothing at all,
    /// or with an index that is not above nothing.
    pub fn check(&self) -> Result<(), LibraryError> {
        if self.currency.trim().len() != 3 || !self.currency.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(LibraryError::BadAspect("a currency of three capitals (EUR)".into()));
        }
        if Resource::ALL.into_iter().any(|r| !(self.part(r).is_finite() && self.part(r) >= 0.0)) {
            return Err(LibraryError::BadAspect("prices of 0 or more".into()));
        }
        if self.unit_price() == 0.0 {
            return Err(LibraryError::BadAspect("a price".into()));
        }
        if let Some(index) = &self.index {
            if index.name.trim().is_empty() || !(index.value.is_finite() && index.value > 0.0) {
                return Err(LibraryError::BadAspect("an index with a name and a value above 0".into()));
            }
        }
        Ok(())
    }

    /// The cost as the costs aspect of the item at `item`.
    pub fn aspect(&self, item: &str, origin: Origin) -> Result<Aspect, LibraryError> {
        self.check()?;
        let properties = match serde_json::to_value(self) {
            Ok(Value::Object(map)) => map.into_iter().collect(),
            _ => Default::default(),
        };
        Ok(Aspect { item: item.into(), kind: Kind::Cost, origin, properties })
    }

    /// The cost a costs aspect holds.
    pub fn of(aspect: &Aspect) -> Result<Cost, LibraryError> {
        if aspect.kind != Kind::Cost {
            return Err(LibraryError::BadAspect("a costs aspect".into()));
        }
        let cost: Cost = serde_json::from_value(Value::Object(aspect.properties.clone().into_iter().collect())).map_err(|e| LibraryError::Unreadable(format!("cost of {}: {e}", aspect.item)))?;
        cost.check()?;
        Ok(cost)
    }

    /// As it is read: "12.50 EUR/m²".
    pub fn label(&self) -> String {
        format!("{:.2} {}/{}", self.unit_price(), self.currency, self.unit.code())
    }
}

impl Library {
    /// The cost of the item at `item`, where it has one that reads.
    pub fn cost(&self, item: &str) -> Option<(Cost, &Origin)> {
        let aspect = self.aspect(item, Kind::Cost)?;
        Cost::of(aspect).ok().map(|cost| (cost, &aspect.origin))
    }

    /// Gives the item at `item` a cost.
    pub fn set_cost(&mut self, item: &str, cost: &Cost, origin: Origin) -> Result<(), LibraryError> {
        self.set_aspect(cost.aspect(item, origin)?)
    }

    /// The price of a build-up per square metre as its materials make it: each layer its
    /// thickness times the price of its material per cubic metre, part by part. None while a
    /// layer's material has no price per cubic metre, or the prices are in different currencies.
    /// It is verified only when every price it is made of is; it has no index of its own.
    pub fn buildup_cost(&self, buildup: &crate::BuildUp) -> Option<Cost> {
        let mut total = Cost { unit: Unit::SquareMetre, verified: true, ..Cost::default() };
        for layer in &buildup.layers {
            let (price, _) = self.cost(&crate::ifcx::material_path(&layer.material))?;
            if price.unit != Unit::CubicMetre || (!total.currency.is_empty() && total.currency != price.currency) {
                return None;
            }
            total.currency = price.currency.clone();
            total.material += price.material * layer.thickness;
            total.labour += price.labour * layer.thickness;
            total.equipment += price.equipment * layer.thickness;
            total.subcontractor += price.subcontractor * layer.thickness;
            total.other += price.other * layer.thickness;
            total.verified &= price.verified;
        }
        (!buildup.layers.is_empty()).then_some(total)
    }

    /// The English name of the item at `item`, as its node has it.
    fn item_name(&self, item: &str) -> Option<String> {
        crate::ifcx::nodes(self).iter().find(|n| n["path"] == item).and_then(|n| n["attributes"]["bsi::ifc::prop::Name"].as_str().map(str::to_string))
    }
}

/// The costs of a library as a catalogue of articles Open Calc Studio reads: one article per
/// part of a price, with the id of the item's node and the part's resource type. Costs in
/// another currency than `currency` are left out, as Open Calc Studio refuses them.
pub fn catalogue(library: &Library, id: &str, version: &str, currency: &str) -> Value {
    let mut resources = Vec::new();
    for aspect in library.aspects.iter().filter(|a| a.kind == Kind::Cost) {
        let Ok(cost) = Cost::of(aspect) else { continue };
        if cost.currency != currency {
            continue;
        }
        let name = library.item_name(&aspect.item).unwrap_or_default();
        for (resource, price) in cost.parts() {
            let (default, reference) = if cost.verified { (json!(price), Value::Null) } else { (Value::Null, json!(price)) };
            let mut article = json!({
                "id": format!("{}:{}", aspect.item, resource.code()),
                "code": format!("{}:{}", aspect.item, resource.code()),
                "description": format!("{name}, {}", resource.names().1),
                "unit": cost.unit.code(),
                "resourceType": resource.code(),
                "category": Kind::Cost.names().1,
                "defaultUnitPrice": default,
                "referenceUnitPrice": reference,
                "currency": cost.currency,
                "source": aspect.origin.source,
                "priceDate": aspect.origin.date,
            });
            if let Some(region) = &aspect.origin.region {
                article["region"] = json!(region);
            }
            resources.push(article);
        }
    }
    json!({"id": id, "version": version, "currency": currency, "resources": resources})
}

/// The name of a unit to read, where the code alone does not say it.
pub fn unit_name(unit: Unit, lang: Lang) -> &'static str {
    match (unit, lang) {
        (Unit::Piece, Lang::Nl) => "stuk",
        (Unit::Piece, _) => "piece",
        (Unit::Hour, Lang::Nl) => "uur",
        (Unit::Hour, _) => "hour",
        (Unit::Day, Lang::Nl) => "dag",
        (Unit::Day, _) => "day",
        (Unit::LumpSum, Lang::Nl) => "post",
        (Unit::LumpSum, _) => "lump sum",
        (unit, _) => unit.code(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall() -> Cost {
        Cost { unit: Unit::SquareMetre, currency: "EUR".into(), material: 48.0, labour: 31.5, equipment: 4.0, index: Some(Index { name: "BDB".into(), value: 120.0 }), ..Cost::default() }
    }

    fn origin() -> Origin {
        Origin { source: "an offer".into(), date: "2026-03-12".into(), region: Some("NL".into()) }
    }

    #[test]
    fn a_price_is_its_parts_together_and_is_brought_to_today_by_its_index() {
        let cost = wall();
        assert_eq!(cost.unit_price(), 83.5);
        assert_eq!(cost.parts().collect::<Vec<_>>(), [(Resource::Material, 48.0), (Resource::Labour, 31.5), (Resource::Equipment, 4.0)]);
        assert_eq!(cost.label(), "83.50 EUR/m²");
        assert_eq!((cost.indexed(132.0), cost.indexed(120.0), cost.indexed(0.0)), (Some(83.5 * 132.0 / 120.0), Some(83.5), None));
        assert_eq!(Cost { index: None, ..wall() }.indexed(132.0), None);
        // the units and the resource types are Open Calc Studio's codes
        assert_eq!(Unit::ALL.map(Unit::code), ["st", "m", "m²", "m³", "kg", "ton", "uur", "dgn", "ls"]);
        assert_eq!(Resource::ALL.map(Resource::code), ["materiaal", "arbeid", "materieel", "onderaannemer", "overig"]);
        for unit in Unit::ALL {
            assert_eq!(serde_json::to_value(unit).unwrap(), json!(unit.code()));
        }
        assert_eq!((serde_json::from_str::<Unit>("\"m2\"").unwrap(), serde_json::from_str::<Unit>("\"m1\"").unwrap(), serde_json::from_str::<Unit>("\"dag\"").unwrap()), (Unit::SquareMetre, Unit::Metre, Unit::Day));
        assert_eq!((unit_name(Unit::Piece, Lang::Nl), unit_name(Unit::SquareMetre, Lang::En)), ("stuk", "m²"));
        // what is no price is refused
        for bad in [Cost { currency: "euro".into(), ..wall() }, Cost { material: -1.0, ..wall() }, Cost { material: f64::NAN, ..wall() }, Cost { unit: Unit::Metre, currency: "EUR".into(), ..Cost::default() }, Cost { index: Some(Index { name: "BDB".into(), value: 0.0 }), ..wall() }] {
            assert!(matches!(bad.check(), Err(LibraryError::BadAspect(_))), "{bad:?}");
        }
    }

    #[test]
    fn a_cost_is_the_costs_aspect_of_an_item_and_goes_through_a_file() {
        let mut library = Library::starter();
        let item = crate::ifcx::buildup_path(&library.buildups[0].id);
        library.set_cost(&item, &wall(), origin()).unwrap();
        assert_eq!(library.cost(&item), Some((wall(), &origin())));
        assert_eq!(library.cost(&crate::ifcx::material_path("steel")), None);
        assert!(matches!(library.set_cost(&item, &Cost { currency: "".into(), ..wall() }, origin()), Err(LibraryError::BadAspect(_))));
        let file = crate::ifcx::file(&library, "test", "test", "2026-11-23T00:00:00Z");
        let node = file["data"].as_array().unwrap().iter().find(|n| n["attributes"].get("openaec::aspect::cost").is_some()).expect("the aspect's node");
        assert_eq!(node["path"], item.as_str());
        assert_eq!(node["attributes"]["openaec::aspect::cost"], json!({"source": "an offer", "date": "2026-03-12", "region": "NL", "unit": "m²", "currency": "EUR", "material": 48.0, "labour": 31.5, "equipment": 4.0, "index": {"name": "BDB", "value": 120.0}}));
        let read = crate::ifcx::read(&file).unwrap();
        assert_eq!(read.cost(&item), Some((wall(), &origin())));
        // an aspect of another kind is no cost
        let mut other = wall().aspect(&item, origin()).unwrap();
        other.kind = Kind::Maker;
        assert!(Cost::of(&other).is_err());
    }

    #[test]
    fn the_costs_are_a_catalogue_open_calc_studio_reads() {
        let mut library = Library::starter();
        let wall_item = crate::ifcx::buildup_path(&library.buildups[0].id);
        let steel = crate::ifcx::material_path("steel");
        library.set_cost(&wall_item, &wall(), origin()).unwrap();
        library.set_cost(&steel, &Cost { unit: Unit::Kilogram, currency: "EUR".into(), material: 1.4, verified: true, ..Cost::default() }, Origin { region: None, ..origin() }).unwrap();
        library.set_cost(&crate::ifcx::material_path("timber"), &Cost { unit: Unit::CubicMetre, currency: "USD".into(), material: 600.0, ..Cost::default() }, origin()).unwrap();
        let catalogue = catalogue(&library, "open-bim-library", "2026-11-23", "EUR");
        assert_eq!((catalogue["id"].as_str(), catalogue["version"].as_str(), catalogue["currency"].as_str()), (Some("open-bim-library"), Some("2026-11-23"), Some("EUR")));
        let resources = catalogue["resources"].as_array().unwrap();
        // three parts of the wall and one of steel; the price in dollars is left out
        assert_eq!(resources.len(), 4);
        let ids: std::collections::BTreeSet<&str> = resources.iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), 4, "every article has an id of its own");
        let labour = resources.iter().find(|r| r["resourceType"] == "arbeid").unwrap();
        assert_eq!((labour["unit"].as_str(), labour["referenceUnitPrice"].as_f64(), labour["defaultUnitPrice"].is_null(), labour["priceDate"].as_str(), labour["source"].as_str(), labour["region"].as_str()), (Some("m²"), Some(31.5), true, Some("2026-03-12"), Some("an offer"), Some("NL")));
        assert!(labour["description"].as_str().unwrap().ends_with(", labour") && labour["description"].as_str().unwrap().len() > ", labour".len());
        // a verified price is the price an estimate takes
        let kg = resources.iter().find(|r| r["unit"] == "kg").unwrap();
        assert_eq!((kg["defaultUnitPrice"].as_f64(), kg["referenceUnitPrice"].is_null(), kg["resourceType"].as_str(), kg.get("region")), (Some(1.4), true, Some("materiaal"), None));
    }
    #[test]
    fn a_build_up_costs_what_its_layers_cost() {
        let mut library = Library::starter();
        let keys: Vec<String> = library.materials.iter().take(3).map(|m| m.key.clone()).collect();
        let wall = crate::BuildUp { layers: vec![crate::Layer { material: keys[0].clone(), thickness: 0.1 }, crate::Layer { material: keys[1].clone(), thickness: 0.2 }], ..library.buildups[0].clone() };
        let per_m3 = |material: f64, labour: f64, verified: bool| Cost { unit: Unit::CubicMetre, currency: "EUR".into(), material, labour, verified, ..Cost::default() };
        let price = |library: &mut Library, key: &str, cost: &Cost| library.set_cost(&crate::ifcx::material_path(key), cost, origin()).unwrap();
        assert_eq!(library.buildup_cost(&wall), None);
        price(&mut library, &keys[0], &per_m3(300.0, 100.0, true));
        assert_eq!(library.buildup_cost(&wall), None, "one layer has no price");
        price(&mut library, &keys[1], &per_m3(150.0, 50.0, true));
        let cost = library.buildup_cost(&wall).unwrap();
        assert_eq!((cost.unit, cost.currency.as_str(), cost.material, cost.labour, cost.unit_price(), cost.verified), (Unit::SquareMetre, "EUR", 60.0, 20.0, 80.0, true));
        assert_eq!(cost.label(), "80.00 EUR/m²");
        // one price that is not verified, and the whole is not
        price(&mut library, &keys[1], &per_m3(150.0, 50.0, false));
        assert!(!library.buildup_cost(&wall).unwrap().verified);
        // a price per kilogram, or in another currency, makes no price per square metre
        price(&mut library, &keys[1], &Cost { unit: Unit::Kilogram, ..per_m3(1.0, 0.0, true) });
        assert_eq!(library.buildup_cost(&wall), None);
        price(&mut library, &keys[1], &Cost { currency: "USD".into(), ..per_m3(150.0, 50.0, true) });
        assert_eq!(library.buildup_cost(&wall), None);
        assert_eq!(library.buildup_cost(&crate::BuildUp { layers: Vec::new(), ..wall }), None);
    }

}
