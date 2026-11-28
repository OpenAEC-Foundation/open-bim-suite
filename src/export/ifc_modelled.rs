//! What the IFC writer knows of the elements the Modeller made, from the definitions they keep
//! (`OpenAEC_definition` among their properties): the openings doors and windows make in walls,
//! and the plain extrusions some elements are.
//!
//! Each element is taken back into a model of the kernel, so a door finds the wall it stands in
//! as the kernel finds it. Coordinates are the project's (metres, world), as the elements are
//! shown; the writer checks them against the triangles it was given before it trusts them.

use crate::model::Model;
use bim_core::element::PlanarForm;
use bim_core::{Element, ElementId, Shape, TileGrid};
use glam::{DVec2, DVec3};
use std::collections::HashMap;

/// The modelled elements of a model: the kernel's model of them, and which element of the model
/// shown each one is.
pub(super) struct Modelled {
    model: bim_core::Model,
    grid: TileGrid,
    /// The element of the model shown (its id there) of each modelled element.
    shown: HashMap<ElementId, u64>,
    /// The modelled element of each element of the model shown.
    of_shown: HashMap<u64, ElementId>,
}

/// An opening an element of the model shown has: in a wall (`host`), filled by a door or window
/// (`filler`) when one stands in it. `origin` is the middle of its bottom edge on the wall's
/// axis; it runs `width` along `along`, `depth` across the wall (through it, with some to
/// spare) and `height` up.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Void {
    pub host: u64,
    pub filler: Option<u64>,
    pub origin: DVec3,
    pub along: DVec2,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    /// The wall's thickness: how much of the wall the opening takes out.
    pub thickness: f64,
}

/// A body that is a profile extruded: the profile's outline and holes in the plane of `x` and
/// `y` of the frame at `origin`, extruded `depth` along that frame's `z`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Extrusion {
    pub origin: DVec3,
    pub x: DVec3,
    pub z: DVec3,
    pub outer: Vec<DVec2>,
    pub holes: Vec<Vec<DVec2>>,
    pub depth: f64,
}

impl Extrusion {
    /// Its volume.
    pub fn volume(&self) -> f64 {
        let area = |ring: &[DVec2]| {
            let n = ring.len();
            (0..n).map(|i| ring[i].perp_dot(ring[(i + 1) % n])).sum::<f64>().abs() / 2.0
        };
        (area(&self.outer) - self.holes.iter().map(|h| area(h)).sum::<f64>()) * self.depth
    }

    /// Its lowest and highest corner (world).
    pub fn bounds(&self) -> (DVec3, DVec3) {
        let y = self.z.cross(self.x);
        let mut lo = DVec3::splat(f64::MAX);
        let mut hi = DVec3::splat(f64::MIN);
        for p in &self.outer {
            for along in [0.0, self.depth] {
                let w = self.origin + self.x * p.x + y * p.y + self.z * along;
                lo = lo.min(w);
                hi = hi.max(w);
            }
        }
        (lo, hi)
    }
}

/// The modelled elements of `model`, or none when it has none.
pub(super) fn read(model: &Model) -> Option<Modelled> {
    let key = crate::export::ifcx::DEFINITION;
    let grid = TileGrid::default();
    let mut kernel = bim_core::Model::new(grid);
    let (mut shown, mut of_shown) = (HashMap::new(), HashMap::new());
    for (&id, props) in &model.props {
        if !props.contains(key) {
            continue;
        }
        let Some(definition) = model.element_props(id).into_iter().find(|(k, _)| k == key).map(|(_, v)| v) else { continue };
        let Ok(definitions) = serde_json::from_str::<Vec<Element>>(&definition) else { continue };
        // the element's own definition first, then the connections it is the primary member of
        let mut definitions = definitions.into_iter();
        let Some(element) = definitions.next() else { continue };
        shown.insert(element.id, id);
        of_shown.insert(id, element.id);
        kernel.insert(element);
        for joint in definitions {
            kernel.insert(joint);
        }
    }
    (!shown.is_empty()).then_some(Modelled { model: kernel, grid, shown, of_shown })
}

impl Modelled {
    /// The openings of the walls: each opening a wall's definition gives and each one a door or
    /// window standing in it makes, the latter filled by it.
    pub fn voids(&self) -> Vec<Void> {
        let mut out = Vec::new();
        for wall in self.model.iter() {
            let Some(&host) = self.shown.get(&wall.id) else { continue };
            let Shape::Planar(p) = wall.shape_in_project(&self.grid) else { continue };
            let PlanarForm::Wall { start, end, base, .. } = p.form else { continue };
            let Some(along) = (end - start).try_normalize() else { continue };
            let thickness = p.total_thickness();
            // the doors and windows that could stand in it, where they stand
            let fillers: Vec<(u64, DVec2, f64)> = self
                .model
                .iter()
                .filter_map(|e| match e.shape_in_project(&self.grid) {
                    Shape::Point(d) if d.opening.is_some() => Some((*self.shown.get(&e.id)?, d.at.truncate(), d.size.x)),
                    _ => None,
                })
                .collect();
            for opening in self.model.openings_of(wall.id) {
                let middle = start + along * (opening.along + opening.width / 2.0);
                // the one standing in it: its middle on the wall's axis where the opening's is
                let filler = fillers.iter().find(|(_, at, width)| {
                    let from = *at - start;
                    (from.dot(along) - (opening.along + opening.width / 2.0)).abs() < 1e-6 && (width - opening.width).abs() < 1e-6 && from.perp_dot(along).abs() <= thickness / 2.0 + 0.1
                });
                out.push(Void {
                    host,
                    filler: filler.map(|f| f.0),
                    origin: middle.extend(base + opening.sill),
                    along,
                    width: opening.width,
                    depth: thickness + 0.1,
                    height: opening.height,
                    thickness,
                });
            }
        }
        out.sort_by_key(|v| (v.host, v.filler));
        out
    }

    /// The extrusion element `id` of the model shown is, when its definition is one before any
    /// joint, cut or opening: a straight wall, a level slab with straight edges, or a member
    /// (column, beam, brace) of one profile. The writer uses it only when it is what is shown.
    pub fn extrusion(&self, id: u64) -> Option<Extrusion> {
        let element = self.model.get(*self.of_shown.get(&id)?)?;
        match element.shape_in_project(&self.grid) {
            Shape::Planar(p) => match &p.form {
                PlanarForm::Wall { start, end, base, height } => {
                    let length = start.distance(*end);
                    let along = (*end - *start).try_normalize()?;
                    let t = p.total_thickness();
                    let rect = vec![DVec2::new(0.0, -t / 2.0), DVec2::new(length, -t / 2.0), DVec2::new(length, t / 2.0), DVec2::new(0.0, t / 2.0)];
                    Some(Extrusion { origin: start.extend(*base), x: along.extend(0.0), z: DVec3::Z, outer: rect, holes: Vec::new(), depth: *height })
                }
                PlanarForm::Slab { outline, holes, top, bulges, slope, .. } => {
                    if slope.is_some() || bulges.iter().any(|b| b.abs() > 1e-12) || outline.len() < 3 {
                        return None;
                    }
                    let t = p.total_thickness();
                    Some(Extrusion { origin: DVec3::new(0.0, 0.0, top - t), x: DVec3::X, z: DVec3::Z, outer: outline.clone(), holes: holes.clone(), depth: t })
                }
                _ => None,
            },
            Shape::Linear(l) if element.hairpin_leg.is_none() => {
                let (axes, length) = bim_core::element::linear_axes(&l).ok()?;
                let profile = l.profile.profile().ok()?;
                Some(Extrusion { origin: axes.origin, x: axes.x, z: axes.z, outer: profile.outer().to_vec(), holes: profile.holes().to_vec(), depth: length })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_extruded_knows_its_volume_and_where_it_reaches() {
        let e = Extrusion {
            origin: DVec3::new(1.0, 2.0, 0.0),
            x: DVec3::Y,
            z: DVec3::Z,
            outer: vec![DVec2::new(0.0, -0.1), DVec2::new(4.0, -0.1), DVec2::new(4.0, 0.1), DVec2::new(0.0, 0.1)],
            holes: Vec::new(),
            depth: 3.0,
        };
        assert!((e.volume() - 4.0 * 0.2 * 3.0).abs() < 1e-12);
        // along y from (1, 2): x is across, from 0.9 to 1.1
        let (lo, hi) = e.bounds();
        assert!(lo.abs_diff_eq(DVec3::new(0.9, 2.0, 0.0), 1e-12) && hi.abs_diff_eq(DVec3::new(1.1, 6.0, 3.0), 1e-12), "{lo} {hi}");
    }
}
