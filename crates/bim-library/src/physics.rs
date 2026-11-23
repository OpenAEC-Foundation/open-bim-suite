//! The physics of a material: its density, how it conducts and holds heat, how it resists
//! vapour, and how it reacts to fire.
//!
//! [`Physics`] is the physics aspect of a material ([`crate::aspect`]): every value is given or
//! left open, and the aspect's source and date say where they are from. A build-up has the
//! thermal resistance of its layers together where every layer's material says how it conducts
//! ([`Library::resistance`]).
//!
//! No values are in the library: the design values of the standards are not free to pass on,
//! and a maker's declared value belongs with the maker's product. An office fills in its own.
//!
//! Not yet: sound, the surface resistances and the corrections of a real construction (an R of
//! layers alone is not an Rc of a standard), moisture content, and values that change with
//! temperature.

use crate::aspect::{Aspect, Kind, Origin};
use crate::buildup::BuildUp;
use crate::{Library, LibraryError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The physics of a material.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Physics {
    /// ρ in kg/m³.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<f64>,
    /// The thermal conductivity λ in W/(m·K).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductivity: Option<f64>,
    /// The specific heat capacity c in J/(kg·K).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specific_heat: Option<f64>,
    /// The water vapour diffusion resistance factor μ (air is 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vapour_resistance: Option<f64>,
    /// The reaction to fire class as EN 13501-1 writes it ("A1", "B-s1,d0").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_class: Option<String>,
}

impl Physics {
    /// Whether it says nothing.
    pub fn is_empty(&self) -> bool {
        *self == Physics::default()
    }

    /// The thermal resistance of a layer of it `thickness` metres thick, in m²K/W.
    pub fn resistance(&self, thickness: f64) -> Option<f64> {
        self.conductivity.map(|conductivity| thickness / conductivity)
    }

    /// The vapour resistance of a layer of it `thickness` metres thick as the thickness of air
    /// that resists as much (sd), in metres.
    pub fn air_layer(&self, thickness: f64) -> Option<f64> {
        self.vapour_resistance.map(|factor| factor * thickness)
    }

    /// Refused when a value is not above nothing (μ: not 1 or more) or the fire class is no
    /// class of EN 13501-1.
    pub fn check(&self) -> Result<(), LibraryError> {
        let positive = |value: Option<f64>| value.is_none_or(|v| v.is_finite() && v > 0.0);
        if !(positive(self.density) && positive(self.conductivity) && positive(self.specific_heat)) {
            return Err(LibraryError::BadAspect("values above 0".into()));
        }
        if !self.vapour_resistance.is_none_or(|v| v.is_finite() && v >= 1.0) {
            return Err(LibraryError::BadAspect("a vapour resistance factor of 1 or more".into()));
        }
        if let Some(class) = &self.fire_class {
            if !is_fire_class(class) {
                return Err(LibraryError::BadAspect("a fire class of EN 13501-1 (A1, A2, B to F, with s1 to s3 and d0 to d2)".into()));
            }
        }
        Ok(())
    }

    /// The physics as the physics aspect of the item at `item`.
    pub fn aspect(&self, item: &str, origin: Origin) -> Result<Aspect, LibraryError> {
        self.check()?;
        let properties = match serde_json::to_value(self) {
            Ok(Value::Object(map)) => map.into_iter().collect(),
            _ => Default::default(),
        };
        Ok(Aspect { item: item.into(), kind: Kind::Physics, origin, properties })
    }

    /// The physics a physics aspect holds.
    pub fn of(aspect: &Aspect) -> Result<Physics, LibraryError> {
        if aspect.kind != Kind::Physics {
            return Err(LibraryError::BadAspect("a physics aspect".into()));
        }
        let physics: Physics = serde_json::from_value(Value::Object(aspect.properties.clone().into_iter().collect())).map_err(|e| LibraryError::Unreadable(format!("physics of {}: {e}", aspect.item)))?;
        physics.check()?;
        Ok(physics)
    }

    /// IFC's own properties of it, by their name in the property sets of a material
    /// (`Pset_MaterialCommon`, `Pset_MaterialThermal`), in SI units.
    pub fn ifc_properties(&self) -> Vec<(&'static str, f64)> {
        [("MassDensity", self.density), ("ThermalConductivity", self.conductivity), ("SpecificHeatCapacity", self.specific_heat)].into_iter().filter_map(|(name, value)| value.map(|v| (name, v))).collect()
    }
}

/// Whether `class` is a reaction to fire class of EN 13501-1: A1, A2, B, C, D, E or F, with
/// smoke (s1 to s3) and droplets (d0 to d2) after a hyphen where the class has them
/// ("A2-s1,d0", "E-d2"), and "fl" for a flooring ("Bfl-s1").
pub fn is_fire_class(class: &str) -> bool {
    let (main, rest) = class.trim().split_once('-').map_or((class.trim(), None), |(main, rest)| (main, Some(rest)));
    let main = main.strip_suffix("fl").unwrap_or(main);
    if !["A1", "A2", "B", "C", "D", "E", "F"].contains(&main) {
        return false;
    }
    rest.is_none_or(|rest| rest.split(',').all(|part| ["s1", "s2", "s3", "d0", "d1", "d2"].contains(&part.trim())))
}

impl Library {
    /// The physics of material `key`, where it has an aspect that reads, with where it is from.
    pub fn physics(&self, key: &str) -> Option<(Physics, &Origin)> {
        let aspect = self.aspect(&crate::ifcx::material_path(key), Kind::Physics)?;
        Physics::of(aspect).ok().map(|physics| (physics, &aspect.origin))
    }

    /// Gives material `key` its physics; physics that say nothing take the aspect away.
    pub fn set_physics(&mut self, key: &str, physics: &Physics, origin: Origin) -> Result<(), LibraryError> {
        if self.material(key).is_none() {
            return Err(LibraryError::UnknownMaterial(key.into()));
        }
        let item = crate::ifcx::material_path(key);
        if physics.is_empty() {
            self.remove_aspect(&item, Kind::Physics);
            return Ok(());
        }
        self.set_aspect(physics.aspect(&item, origin)?)
    }

    /// The thermal resistance of the layers of a build-up together, in m²K/W: none while a
    /// layer's material does not say how it conducts. Without surface resistances.
    pub fn resistance(&self, buildup: &BuildUp) -> Option<f64> {
        buildup.layers.iter().map(|layer| self.physics(&layer.material).and_then(|(physics, _)| physics.resistance(layer.thickness))).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildup::Layer;

    fn origin() -> Origin {
        Origin { source: "the maker's declaration".into(), date: "2026-11-23".into(), region: None }
    }

    #[test]
    fn a_material_has_its_physics_with_a_source() {
        let mut library = Library::starter();
        let wool = Physics { density: Some(35.0), conductivity: Some(0.035), specific_heat: Some(1030.0), vapour_resistance: Some(1.0), fire_class: Some("A1".into()) };
        let key = library.materials.iter().find(|m| m.key.contains("insulation") || m.key.contains("wool")).map_or_else(|| library.materials[0].key.clone(), |m| m.key.clone());
        library.set_physics(&key, &wool, origin()).unwrap();
        assert_eq!(library.physics(&key), Some((wool.clone(), &origin())));
        assert_eq!((wool.resistance(0.14).map(|r| (r * 1000.0).round() / 1000.0), wool.air_layer(0.14)), (Some(4.0), Some(0.14)));
        // what is no value is refused and nothing changes
        for bad in [Physics { conductivity: Some(0.0), ..wool.clone() }, Physics { density: Some(-1.0), ..wool.clone() }, Physics { vapour_resistance: Some(0.5), ..wool.clone() }, Physics { fire_class: Some("G".into()), ..wool.clone() }, Physics { specific_heat: Some(f64::NAN), ..wool.clone() }] {
            assert!(matches!(library.set_physics(&key, &bad, origin()), Err(LibraryError::BadAspect(_))), "{bad:?}");
        }
        assert!(matches!(library.set_physics("no-such", &wool, origin()), Err(LibraryError::UnknownMaterial(_))));
        assert!(matches!(library.set_physics(&key, &wool, Origin { source: "".into(), ..origin() }), Err(LibraryError::BadAspect(_))));
        assert_eq!(library.physics(&key).map(|(p, _)| p), Some(wool.clone()));
        // through a file, with IFC's own properties on the material's node
        let file = crate::ifcx::file(&library, "test", "test", "2026-11-23T00:00:00Z");
        let path = crate::ifcx::material_path(&key);
        let own: Vec<&Value> = file["data"].as_array().unwrap().iter().filter(|n| n["path"] == path.as_str()).collect();
        assert_eq!(own[0]["attributes"]["bsi::ifc::prop::ThermalConductivity"].as_f64(), Some(0.035));
        assert_eq!((own[0]["attributes"]["bsi::ifc::prop::MassDensity"].as_f64(), own[0]["attributes"]["bsi::ifc::prop::SpecificHeatCapacity"].as_f64()), (Some(35.0), Some(1030.0)));
        assert_eq!(own[1]["attributes"]["openaec::aspect::physics"]["fire_class"], "A1");
        assert_eq!(crate::ifcx::read(&file).unwrap().physics(&key), Some((wool, &origin())));
        // physics that say nothing take the aspect away
        library.set_physics(&key, &Physics::default(), origin()).unwrap();
        assert_eq!((library.physics(&key), library.aspects.len()), (None, 0));
    }

    #[test]
    fn a_fire_class_is_one_of_en_13501() {
        for class in ["A1", "A2-s1,d0", "B-s2,d1", "E", "E-d2", "F", "Bfl-s1", "A1fl", " D-s3, d2 "] {
            assert!(is_fire_class(class), "{class}");
        }
        for class in ["", "G", "A3", "B-s4", "B-s1,d3", "A", "b", "B-"] {
            assert!(!is_fire_class(class), "{class}");
        }
    }

    #[test]
    fn a_build_up_has_the_resistance_of_its_layers_once_each_conducts() {
        let mut library = Library::starter();
        let keys: Vec<String> = library.materials.iter().take(3).map(|m| m.key.clone()).collect();
        let wall = BuildUp { layers: vec![Layer { material: keys[0].clone(), thickness: 0.1 }, Layer { material: keys[1].clone(), thickness: 0.14 }, Layer { material: keys[2].clone(), thickness: 0.1 }], ..library.buildups[0].clone() };
        assert_eq!(library.resistance(&wall), None);
        let conducts = |conductivity: f64| Physics { conductivity: Some(conductivity), ..Physics::default() };
        library.set_physics(&keys[0], &conducts(1.0), origin()).unwrap();
        library.set_physics(&keys[1], &conducts(0.035), origin()).unwrap();
        assert_eq!(library.resistance(&wall), None, "one layer does not say how it conducts");
        library.set_physics(&keys[2], &conducts(0.5), origin()).unwrap();
        let r = library.resistance(&wall).unwrap();
        assert!((r - (0.1 + 4.0 + 0.2)).abs() < 1e-9, "{r}");
    }
}
