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
//! as thick as its layers together ([`BuildUp::at`]); a linear type at 200 is the box its
//! profile fits in ([`LinearType::at`]); a system type at 200 is one band as deep as it reaches
//! across its face ([`SystemType::at`]). A material has, per level, what is drawn of it
//! ([`Detail::material`]). A material, a linear type and a system type each say which of their
//! data is filled in at a level (`data_at`).
//!
//! The definitions are in `docs/library/levels-of-detail.md`.
//!
//! A library says the level its drawings show at ([`crate::Library::detail`]): at 200 what is cut
//! through one of its materials is an outline alone, without a hatch.
//!
//! Not yet: a view or a phase does not choose a level of its own; objects are the same at every
//! level; views per level drawn by hand.

use crate::buildup::BuildUp;
use crate::lod::Lod;
use crate::material::Material;
use crate::profile::{LinearType, ProfileShape};
use crate::system::SystemType;
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

/// One band of a build-up or of a system as a level shows it: so thick, and of a material where
/// the level shows materials. Of a system above 200, a band of no material is a gap between two
/// of its layers.
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
        let exposed = self.exposures();
        if !exposed.is_empty() {
            data.push(Datum { nl: "Milieuklassen", en: "Exposure classes", value: exposed.iter().map(|c| c.name).collect::<Vec<_>>().join(", "), from: Detail::D350 });
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

fn datum(nl: &'static str, en: &'static str, value: String, from: Detail) -> Datum {
    Datum { nl, en, value, from }
}

impl LinearType {
    /// The profile a cut across it shows at `detail`: at 200 the box its profile fits in (a
    /// circle where it is round), above that its own profile.
    pub fn at(&self, detail: Detail) -> ProfileShape {
        let size = self.profile.size();
        match detail {
            Detail::D200 if self.profile.round() => ProfileShape::Circle { diameter: size.x },
            Detail::D200 => ProfileShape::Rectangle { width: size.x, depth: size.y },
            Detail::D350 | Detail::D500 => self.profile.clone(),
        }
    }

    /// Everything known of it, each with the lowest level it is filled in at: its name, what it
    /// is and its size at 200; its material, its profile and its code at 350; its key, the
    /// area and the second moments of its profile and its material's grade at 500.
    pub fn data(&self, materials: &[Material], lang: Lang) -> Vec<Datum> {
        let mm = |v: f64| crate::millimetres(v, lang);
        let size = self.profile.size();
        let material = materials.iter().find(|m| m.key == self.material);
        let mut data = vec![
            datum("Naam", "Name", self.rule_name(materials, lang), Detail::D200),
            datum("Soort", "Kind", self.role.name().to_string(), Detail::D200),
            datum("Maat", "Size", if self.profile.round() { format!("⌀ {} mm", mm(size.x)) } else { format!("{} × {} mm", mm(size.x), mm(size.y)) }, Detail::D200),
            datum("Materiaal", "Material", material.map_or(self.material.clone(), |m| m.name(lang).to_string()), Detail::D350),
            datum("Profiel", "Profile", self.designation.clone().unwrap_or_else(|| self.profile.ifc().0.to_string()), Detail::D350),
        ];
        if let Some(code) = &self.code {
            data.push(datum("NL-SfB", "NL-SfB", code.clone(), Detail::D350));
        }
        data.push(datum("Sleutel", "Key", self.id.clone(), Detail::D500));
        if let Ok(props) = self.profile.props() {
            data.push(datum("Oppervlakte", "Area", format!("{:.1} cm²", props.area * 1e4), Detail::D500));
            data.push(datum("Traagheidsmomenten", "Second moments", format!("Iy {:.0} · Iz {:.0} cm⁴", props.iu * 1e8, props.iv * 1e8), Detail::D500));
        }
        if let Some(grade) = material.and_then(|m| m.grade.as_ref()) {
            data.push(datum("Kwaliteit", "Grade", grade.clone(), Detail::D500));
        }
        data
    }

    /// What of its data is filled in at `detail`.
    pub fn data_at(&self, detail: Detail, materials: &[Material], lang: Lang) -> Vec<Datum> {
        self.data(materials, lang).into_iter().filter(|d| d.from <= detail).collect()
    }
}

impl SystemType {
    /// The bands a cut across its face shows at `detail`, from its exterior in: at 200 one band
    /// as deep as it reaches and of no material; above that its face and each of its layers, as
    /// deep as its deepest part and of its panels' material, with a band of no material where
    /// two of them leave a gap.
    pub fn at(&self, detail: Detail) -> Vec<Band<'_>> {
        let (interior, exterior) = self.extent();
        if detail == Detail::D200 {
            return vec![Band { thickness: exterior - interior, material: None }];
        }
        let mut faces: Vec<(f64, f64, &str)> = self
            .faces()
            .map(|(offset, _, members, panels)| {
                let half = members.map_or(0.0, |m| m.depth).max(panels.thickness) / 2.0;
                (offset + half, offset - half, panels.material.as_str())
            })
            .collect();
        faces.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut bands = Vec::new();
        let mut at = exterior;
        for (top, bottom, material) in faces {
            if at - top > 1e-6 {
                bands.push(Band { thickness: at - top, material: None });
            }
            let top = top.min(at);
            if top - bottom > 1e-6 {
                bands.push(Band { thickness: top - bottom, material: Some(material) });
                at = bottom;
            }
        }
        bands
    }

    /// Everything known of it, each with the lowest level it is filled in at: its name, its
    /// group and how deep it is at 200; its grid, its members, its panels and how many layers
    /// it has at 350; its key, its code and each layer at 500.
    pub fn data(&self, materials: &[Material], lang: Lang) -> Vec<Datum> {
        let mm = |v: f64| crate::millimetres(v, lang);
        let named = |key: &str| materials.iter().find(|m| m.key == key).map_or(key.to_string(), |m| m.name(lang).to_string());
        let (interior, exterior) = self.extent();
        let mut data = vec![
            datum("Naam", "Name", self.rule_name(materials, lang), Detail::D200),
            datum("Groep", "Group", self.group.name(lang).to_string(), Detail::D200),
            datum("Diepte", "Depth", format!("{} mm", mm(exterior - interior)), Detail::D200),
            datum("Raster", "Grid", format!("{} × {} mm", mm(self.grid.bay), mm(self.grid.row)), Detail::D350),
            datum("Stijlen en regels", "Members", format!("{} {} × {} mm", named(&self.members.material), mm(self.members.width), mm(self.members.depth)), Detail::D350),
            datum("Vulling", "Panels", format!("{} {} mm", named(&self.panels.material), mm(self.panels.thickness)), Detail::D350),
        ];
        if !self.layers.is_empty() {
            data.push(datum("Lagen", "Layers", self.layers.len().to_string(), Detail::D350));
        }
        data.push(datum("Sleutel", "Key", self.id.clone(), Detail::D500));
        if let Some(code) = &self.code {
            data.push(datum("NL-SfB", "NL-SfB", code.clone(), Detail::D500));
        }
        for layer in &self.layers {
            let side = if layer.offset > 0.0 { ("Laag buiten", "Layer outside") } else { ("Laag binnen", "Layer inside") };
            data.push(datum(side.0, side.1, format!("{} {} mm", named(&layer.panels.material), mm(layer.depth())), Detail::D500));
        }
        data
    }

    /// What of its data is filled in at `detail`.
    pub fn data_at(&self, detail: Detail, materials: &[Material], lang: Lang) -> Vec<Datum> {
        self.data(materials, lang).into_iter().filter(|d| d.from <= detail).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Library;

    #[test]
    fn a_linear_type_at_200_is_the_box_of_its_profile() {
        let library = Library::starter();
        let steel = LinearType::steel("test.hea200", crate::LinearRole::Beam, "HEA 200").expect("HEA 200");
        // an I profile: 200 wide and 190 high, and at 200 no more than that
        assert_eq!(steel.at(Detail::D200), ProfileShape::Rectangle { width: 0.2, depth: 0.19 });
        assert_eq!((steel.at(Detail::D350), steel.at(Detail::D500)), (steel.profile.clone(), steel.profile.clone()));
        // a round one stays round, a tube without its wall
        let tube = LinearType::new("test.tube", crate::LinearRole::Column, "steel", ProfileShape::Chs { d: 0.1143, t: 0.005 });
        assert_eq!((tube.at(Detail::D200), tube.at(Detail::D350)), (ProfileShape::Circle { diameter: 0.1143 }, tube.profile.clone()));
        // the box is as large as the profile at every level, and can be drawn
        for t in &library.linear_types {
            let outline = t.at(Detail::D200);
            assert!((outline.size() - t.profile.size()).length() < 1e-12 && outline.rings(1e-3).is_ok_and(|rings| rings.1.is_empty()), "{}", t.id);
        }
        // its data: more at a higher level, and never less
        let at = |d| steel.data_at(d, &library.materials, Lang::En).into_iter().map(|d| d.en).collect::<Vec<_>>();
        assert_eq!(at(Detail::D200), ["Name", "Kind", "Size"]);
        assert_eq!(at(Detail::D350), ["Name", "Kind", "Size", "Material", "Profile"]);
        assert_eq!(at(Detail::D500), ["Name", "Kind", "Size", "Material", "Profile", "Key", "Area", "Second moments"]);
        let all = steel.data(&library.materials, Lang::Nl);
        let of = |nl: &str| all.iter().find(|d| d.nl == nl).map(|d| d.value.as_str());
        assert_eq!((of("Naam"), of("Soort"), of("Maat"), of("Profiel"), of("Sleutel")), (Some("HEA 200"), Some("Beam"), Some("200 × 190 mm"), Some("HEA 200"), Some("test.hea200")));
        assert_eq!(tube.data(&library.materials, Lang::En).iter().find(|d| d.en == "Size").map(|d| d.value.as_str()), Some("⌀ 114.3 mm"));
        for t in &library.linear_types {
            let counts = Detail::ALL.map(|d| t.data_at(d, &library.materials, Lang::En).len());
            assert!(counts[0] == 3 && counts[0] < counts[1] && counts[1] < counts[2], "{}: {counts:?}", t.id);
        }
    }

    #[test]
    fn a_system_type_at_200_is_one_band_as_deep_as_it_reaches() {
        let library = Library::starter();
        let layered = library.system_types.iter().find(|t| t.layers.len() >= 3).expect("a system with layers");
        let (interior, exterior) = layered.extent();
        let outline = layered.at(Detail::D200);
        assert_eq!((outline.len(), outline[0].material), (1, None));
        assert!((outline[0].thickness - (exterior - interior)).abs() < 1e-12);
        for t in &library.system_types {
            let (interior, exterior) = t.extent();
            for detail in [Detail::D350, Detail::D500] {
                let bands = t.at(detail);
                // every level is as deep as the system, and shows a material
                assert!((bands.iter().map(|b| b.thickness).sum::<f64>() - (exterior - interior)).abs() < 1e-9, "{}", t.id);
                assert!(bands.iter().any(|b| b.material.is_some()) && bands.iter().all(|b| b.thickness > 0.0), "{}", t.id);
            }
            let counts = Detail::ALL.map(|d| t.data_at(d, &library.materials, Lang::En).len());
            assert!(counts[0] == 3 && counts[0] < counts[1] && counts[1] < counts[2], "{}: {counts:?}", t.id);
        }
        // above 200 a layered system shows a band per layer it has, or more where they leave gaps
        assert!(layered.at(Detail::D350).iter().filter(|b| b.material.is_some()).count() >= 3);
        let at = |d| layered.data_at(d, &library.materials, Lang::En).into_iter().map(|d| d.en).collect::<Vec<_>>();
        assert_eq!(at(Detail::D200), ["Name", "Group", "Depth"]);
        assert_eq!(at(Detail::D350), ["Name", "Group", "Depth", "Grid", "Members", "Panels", "Layers"]);
        assert!(at(Detail::D500).len() >= 8 + layered.layers.len() && at(Detail::D500).contains(&"Key"));
        // a system without layers is its face alone
        let plain = library.system_types.iter().find(|t| t.layers.is_empty()).expect("a system without layers");
        assert_eq!((plain.at(Detail::D350).len(), plain.at(Detail::D350)[0].material), (1, Some(plain.panels.material.as_str())));
        assert!(!plain.data(&library.materials, Lang::En).iter().any(|d| d.en == "Layers"));
    }

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
