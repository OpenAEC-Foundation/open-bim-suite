//! Levels of detail: what of an item is shown, and which of its data is filled in, at LOD 200,
//! 350 and 500.
//!
//! [`crate::lod`] says how far a type is worked out. This says how much of it a view shows, or a
//! phase of a project asks for ([`Detail`]):
//!
//! - **200**, a design element: its outline and no more. Plain and black: no layers, no hatch.
//! - **350**, as it is built: its layers and its parts, each with the hatch of its material.
//! - **500**, everything there is: every part, and all that is known of every material.
//!
//! A lower level is made from a higher one where that can be done: a build-up at 200 is one band
//! as thick as its layers together ([`BuildUp::at`]). A material has, per level, what is drawn
//! of it ([`Detail::material`]) and which of its data is filled in ([`Material::data_at`]).
//!
//! The definitions are in `docs/library/levels-of-detail.md`.
//!
//! A library says the level its drawings show at ([`crate::Library::detail`]): at 200 what is cut
//! through one of its materials is an outline alone, without a hatch.
//!
//! Not yet: a view or a phase does not choose a level of its own; linear types, system types and
//! objects are the same at every level; views per level drawn by hand.

use crate::buildup::BuildUp;
use crate::lod::Lod;
use crate::material::Material;
use crate::Lang;
use serde::{Deserialize, Serialize};

/// A level of detail an item is shown at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum Detail {
    #[serde(rename = "200")]
    D200,
    #[default]
    #[serde(rename = "350")]
    D350,
    #[serde(rename = "500")]
    D500,
}

/// What a section through a material shows of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shown {
    /// The paper: an outline round it and no more.
    Outline,
    /// Its hatch.
    Hatch,
}

/// One band of a build-up as a level shows it: so thick, and of a material where the level
/// shows materials.
#[derive(Debug, Clone, PartialEq)]
pub struct Band<'a> {
    pub thickness: f64,
    pub material: Option<&'a str>,
}

/// One thing known of a material: what it is called in Dutch and in English, and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datum {
    pub nl: &'static str,
    pub en: &'static str,
    pub value: String,
    /// The lowest level it is filled in at.
    pub from: Detail,
}

impl Detail {
    pub const ALL: [Detail; 3] = [Detail::D200, Detail::D350, Detail::D500];

    /// Its number: 200, 350 or 500.
    pub fn number(self) -> u32 {
        match self {
            Detail::D200 => 200,
            Detail::D350 => 350,
            Detail::D500 => 500,
        }
    }

    /// As it is written: "LOD 350".
    pub fn name(self) -> &'static str {
        match self {
            Detail::D200 => "LOD 200",
            Detail::D350 => "LOD 350",
            Detail::D500 => "LOD 500",
        }
    }

    /// What it shows, in Dutch and in English.
    pub fn says(self) -> (&'static str, &'static str) {
        match self {
            Detail::D200 => ("ontwerpelement: alleen de omtrek", "design element: the outline alone"),
            Detail::D350 => ("zoals gebouwd wordt: lagen, onderdelen en arceringen", "as it is built: layers, parts and hatches"),
            Detail::D500 => ("alles wat er is: elk onderdeel en alle gegevens", "everything there is: every part and all the data"),
        }
    }

    /// The level with `number`.
    pub fn of(number: u32) -> Option<Detail> {
        Detail::ALL.into_iter().find(|d| d.number() == number)
    }

    /// The level a type worked out to `lod` can be shown at without making anything up: a
    /// generic type has an outline alone; one worked out to be made has everything.
    pub fn up_to(lod: Lod) -> Detail {
        match lod {
            Lod::L200 => Detail::D200,
            Lod::L300 | Lod::L350 => Detail::D350,
            Lod::L400 => Detail::D500,
        }
    }

    /// What a section shows of a material at this level.
    pub fn material(self) -> Shown {
        match self {
            Detail::D200 => Shown::Outline,
            Detail::D350 | Detail::D500 => Shown::Hatch,
        }
    }
}

impl BuildUp {
    /// The bands a section through it shows at `detail`, from its first layer on: at 200 one
    /// band of its whole thickness and of no material, above that its layers.
    pub fn at(&self, detail: Detail) -> Vec<Band<'_>> {
        match detail {
            Detail::D200 => vec![Band { thickness: self.thickness(), material: None }],
            Detail::D350 | Detail::D500 => self.layers.iter().map(|layer| Band { thickness: layer.thickness, material: Some(layer.material.as_str()) }).collect(),
        }
    }
}

impl Material {
    /// Everything known of it, each with the lowest level it is filled in at: its name at 200;
    /// its hatch and its grade at 350; its colour, its key and what its strength class says at
    /// 500.
    pub fn data(&self, lang: Lang) -> Vec<Datum> {
        let mut data = vec![Datum { nl: "Naam", en: "Name", value: self.name(lang).to_string(), from: Detail::D200 }];
        let hatch = self.catalogue_hatch().map_or_else(|| format!("{:?}", self.hatch).to_lowercase(), |h| h.name(lang).to_string());
        data.push(Datum { nl: "Arcering", en: "Hatch", value: hatch, from: Detail::D350 });
        if let Some(grade) = &self.grade {
            data.push(Datum { nl: "Kwaliteit", en: "Grade", value: grade.clone(), from: Detail::D350 });
        }
        let [r, g, b] = self.colour;
        data.push(Datum { nl: "Kleur", en: "Colour", value: format!("#{r:02x}{g:02x}{b:02x}"), from: Detail::D500 });
        data.push(Datum { nl: "Sleutel", en: "Key", value: self.key.clone(), from: Detail::D500 });
        if let Some(class) = self.class() {
            let mut lines = class.lines();
            let source = lines.pop().unwrap_or_default();
            data.push(Datum { nl: "Sterkteklasse", en: "Strength class", value: lines.join("; "), from: Detail::D500 });
            data.push(Datum { nl: "Bron", en: "Source", value: source, from: Detail::D500 });
        }
        data
    }

    /// What of its data is filled in at `detail`.
    pub fn data_at(&self, detail: Detail, lang: Lang) -> Vec<Datum> {
        self.data(lang).into_iter().filter(|d| d.from <= detail).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Library;

    #[test]
    fn the_levels_of_detail_are_three_in_order() {
        assert_eq!(Detail::ALL.map(Detail::number), [200, 350, 500]);
        assert_eq!(Detail::ALL.map(Detail::name), ["LOD 200", "LOD 350", "LOD 500"]);
        assert!(Detail::D200 < Detail::D350 && Detail::D350 < Detail::D500);
        assert_eq!((Detail::of(350), Detail::of(300), Detail::default()), (Some(Detail::D350), None, Detail::D350));
        assert_eq!((serde_json::to_string(&Detail::D500).unwrap(), serde_json::from_str::<Detail>("\"200\"").unwrap()), ("\"500\"".to_string(), Detail::D200));
        assert_eq!(Lod::ALL.map(Detail::up_to), [Detail::D200, Detail::D350, Detail::D350, Detail::D500]);
        assert_eq!(Detail::ALL.map(Detail::material), [Shown::Outline, Shown::Hatch, Shown::Hatch]);
        for d in Detail::ALL {
            assert!(!d.says().0.is_empty() && !d.says().1.is_empty());
        }
    }

    #[test]
    fn a_build_up_at_200_is_its_outline_made_from_its_layers() {
        let library = Library::starter();
        let layered = library.buildups.iter().find(|b| b.layers.len() >= 3).expect("a layered build-up");
        let outline = layered.at(Detail::D200);
        assert_eq!(outline.len(), 1);
        assert_eq!((outline[0].thickness, outline[0].material), (layered.thickness(), None));
        for detail in [Detail::D350, Detail::D500] {
            let bands = layered.at(detail);
            assert_eq!(bands.len(), layered.layers.len());
            assert!(bands.iter().zip(&layered.layers).all(|(band, layer)| band.thickness == layer.thickness && band.material == Some(layer.material.as_str())));
            // every level is as thick as the build-up
            assert!((bands.iter().map(|b| b.thickness).sum::<f64>() - layered.thickness()).abs() < 1e-12);
        }
    }

    #[test]
    fn a_material_has_more_data_at_a_higher_level() {
        let library = crate::ocondat::library();
        let steel = library.material("steel-s355").unwrap();
        let at = |d| steel.data_at(d, Lang::En).into_iter().map(|d| d.en).collect::<Vec<_>>();
        assert_eq!(at(Detail::D200), ["Name"]);
        assert_eq!(at(Detail::D350), ["Name", "Hatch", "Grade"]);
        assert_eq!(at(Detail::D500), ["Name", "Hatch", "Grade", "Colour", "Key", "Strength class", "Source"]);
        let all = steel.data(Lang::Nl);
        let of = |nl: &str| all.iter().find(|d| d.nl == nl).map(|d| d.value.as_str());
        assert_eq!((of("Naam"), of("Kwaliteit"), of("Bron")), (Some("staal S355"), Some("S355"), Some("EN 1993-1-1:2005, table 3.1 and 3.2.6")));
        assert!(of("Sterkteklasse").is_some_and(|v| v.starts_with("fy 355 · fu 490") && v.contains("γM0 1")));
        // a material without a grade has no class to tell of
        let plain = library.material("concrete").unwrap();
        assert_eq!(plain.data_at(Detail::D500, Lang::En).into_iter().map(|d| d.en).collect::<Vec<_>>(), ["Name", "Hatch", "Colour", "Key"]);
        // every material of the library has a name at 200 and no less at a higher level
        for m in &library.materials {
            let counts = Detail::ALL.map(|d| m.data_at(d, Lang::En).len());
            assert!(counts[0] == 1 && counts[0] < counts[1] && counts[1] < counts[2], "{}: {counts:?}", m.key);
        }
    }
    #[test]
    fn a_library_says_the_level_its_drawings_show_at() {
        let mut library = Library::starter();
        assert_eq!((library.detail, library.shown_detail(), library.hatches_cuts()), (None, Detail::D350, true));
        library.detail = Some(Detail::D200);
        assert!(!library.hatches_cuts());
        library.detail = Some(Detail::D500);
        assert!(library.hatches_cuts());
        // kept in the library's file where it is said
        let nodes = |library: &Library| crate::ifcx::nodes(library).len();
        assert_eq!(nodes(&library), nodes(&Library::starter()) + 1);
        let file = crate::ifcx::file(&library, "test", "test", "2026-11-23T00:00:00Z");
        assert_eq!(crate::ifcx::read(&file).unwrap().detail, Some(Detail::D500));
        let plain = crate::ifcx::file(&Library::starter(), "test", "test", "2026-11-23T00:00:00Z");
        assert_eq!(crate::ifcx::read(&plain).unwrap().detail, None);
    }

}
