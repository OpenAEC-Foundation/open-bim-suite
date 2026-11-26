//! Exact solids of the elements, in the springrep B-rep kernel (MIT): what is round in an
//! element's definition is round here, where the planar solids ([`crate::solid`]) draw it in
//! chords. Used for exact quantities and for exchange with CAD (STEP).
//!
//! The planar solids stay the source of display, cuts and joins. An exact solid is made from the
//! definition only where that is what the element evaluates to (a round member with its own square
//! ends, a level slab with bowed edges); every other part is its planar solid, sewn face by face,
//! which is exact for a solid with flat faces.
//!
//! springrep works in millimetres; the kernel in metres. Parts are built in the frame of their
//! element's tile, so the numbers stay small and its tolerances hold; a file places each element
//! at its tile's origin.

use crate::element::{arc_of, linear_axes, linear_solid, planar_parts, Element, LinearDef, PlanarDef, PlanarForm, Shape};
use crate::model::{Model, Part};
use crate::profile::ProfileDef;
use crate::{ElementId, Refusal};
use glam::{DVec2, DVec3};
use springrep::math::{Frame, Transform, Vec2, Vec3};
use springrep::profile::{NamedSegment, Profile, ProfileLoop};
use springrep::topo::Brep;

/// Millimetres per metre.
const MM: f64 = 1000.0;

/// How close two points of a planar solid must lie to be sewn into one vertex (mm): the kernel's
/// linear tolerance.
const SEW: f64 = crate::LINEAR_TOLERANCE * MM;

/// A part of an element as an exact solid, in the frame of its tile, in millimetres.
#[derive(Debug, Clone)]
pub struct ExactPart {
    /// The IFC class of the part.
    pub class: &'static str,
    /// The material key of its layer, if it has one.
    pub material: Option<String>,
    pub brep: Brep,
    /// Whether it was made from the definition with its curves (true), or sewn from its planar
    /// solid (false).
    pub curved: bool,
}

fn v3(p: DVec3) -> Vec3 {
    Vec3::new(p.x * MM, p.y * MM, p.z * MM)
}

fn v2(p: DVec2) -> Vec2 {
    Vec2::new(p.x * MM, p.y * MM)
}

fn failed(_: springrep::Error) -> Refusal {
    Refusal::NotClosed
}

/// A planar solid as an exact one: each face a flat piece, sewn along the edges the faces share.
pub fn from_solid(solid: &crate::Solid) -> Result<Brep, Refusal> {
    use springrep::sew::{sew, Piece, Span};
    let pieces = solid
        .faces
        .iter()
        // a face of no area (a sliver a cut leaves, all its corners on one line) bounds nothing
        .filter(|f| {
            let ring = &f.rings[0];
            let n = ring.len();
            let at = |k: usize| solid.positions[ring[k % n] as usize];
            let twice: DVec3 = (0..n).map(|k| at(k).cross(at(k + 1))).sum();
            twice.length() / 2.0 > crate::LINEAR_TOLERANCE * crate::LINEAR_TOLERANCE
        })
        .map(|f| {
            let loops = f
                .rings
                .iter()
                .map(|ring| {
                    // a corner within the tolerance of the one before is that corner: an edge
                    // shorter than it has no direction
                    let mut points: Vec<Vec3> = Vec::with_capacity(ring.len());
                    for &i in ring {
                        let p = v3(solid.positions[i as usize]);
                        if points.last().is_none_or(|q: &Vec3| q.dist(p) > SEW) {
                            points.push(p);
                        }
                    }
                    while points.len() > 1 && points[0].dist(points[points.len() - 1]) <= SEW {
                        points.pop();
                    }
                    let n = points.len();
                    (0..n).map(|k| Span::line(points[k], points[(k + 1) % n]).map_err(failed)).collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            Piece::planar(loops, "face").map_err(failed)
        })
        .collect::<Result<Vec<_>, _>>()?;
    sew(&pieces, SEW, "solid").map_err(failed)
}

/// Whether an element has curves its planar solid draws in chords: a round member, or a slab
/// with a bowed edge.
pub fn has_curves(e: &Element) -> bool {
    match &e.shape {
        Shape::Linear(l) => round_section(&l.profile).is_some(),
        Shape::Planar(p) => matches!(&p.form, PlanarForm::Slab { bulges, .. } if bulges.iter().any(|b| arc_of(DVec2::ZERO, DVec2::X, *b).is_some())),
        _ => false,
    }
}

/// A round section's outer diameter and, for a tube, its inner one (metres).
fn round_section(profile: &ProfileDef) -> Option<(f64, Option<f64>)> {
    use bim_library::ProfileShape;
    match profile {
        ProfileDef::Circle { diameter } => Some((*diameter, None)),
        ProfileDef::Catalogue { shape: ProfileShape::Circle { diameter }, .. } => Some((*diameter, None)),
        ProfileDef::Catalogue { shape: ProfileShape::Chs { d, t }, .. } if *t * 2.0 < *d => Some((*d, Some(*d - 2.0 * t))),
        _ => None,
    }
}

/// The exact solid of a round member with its own square ends.
fn round_member(l: &LinearDef, outer: f64, inner: Option<f64>) -> Result<Brep, Refusal> {
    let (axes, length) = linear_axes(l)?;
    let frame = Frame::new(v3(axes.origin), Vec3::new(axes.z.x, axes.z.y, axes.z.z), Vec3::new(axes.x.x, axes.x.y, axes.x.z)).map_err(failed)?;
    let mut section = Profile::circle(Vec2::ZERO, outer * MM);
    if let Some(inner) = inner {
        let mut hole = Profile::named_circle(Vec2::ZERO, inner * MM, "bore").loops.remove(0);
        hole.reverse();
        section = section.with_holes([hole]);
    }
    springrep::sweep::extrude(&section, &frame, length * MM, "member").map_err(failed)
}

/// A slab's outline with its bowed edges as arcs, its holes as polygons: the loops a level slab is
/// extruded from (millimetres, in plan).
fn slab_profile(outline: &[DVec2], bulges: &[f64], holes: &[Vec<DVec2>]) -> Result<Profile, Refusal> {
    let n = outline.len();
    let mut segments = Vec::with_capacity(n);
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let name = format!("edge_{i}");
        match arc_of(a, b, bulges.get(i).copied().unwrap_or(0.0)) {
            // turning as `arc_points` draws it: from `start`, counter-clockwise where `turn > 0`
            Some((centre, radius, start, turn)) => segments.push(NamedSegment::arc(v2(centre), radius * MM, start, turn, &name)),
            None => segments.push(NamedSegment::line(v2(a), v2(b), &name)),
        }
    }
    let holes = holes.iter().enumerate().map(|(h, ring)| {
        let m = ring.len();
        ProfileLoop { segments: (0..m).map(|k| NamedSegment::line(v2(ring[k]), v2(ring[(k + 1) % m]), &format!("hole_{h}_{k}"))).collect() }
    });
    Profile::outline(segments).with_holes(holes).normalized().map_err(failed)
}

/// The exact solids of a level slab, one per layer as [`planar_parts`] gives them (top down).
fn level_slab(p: &PlanarDef) -> Result<Vec<(Option<String>, Brep)>, Refusal> {
    let PlanarForm::Slab { outline, holes, top, bulges, .. } = &p.form else { return Err(Refusal::Degenerate) };
    let profile = slab_profile(outline, bulges, holes)?;
    let thickness = p.total_thickness();
    let layers: Vec<(Option<String>, f64, f64)> = match &p.build_up {
        None => vec![(None, 0.0, thickness)],
        Some(b) => {
            let mut from = 0.0;
            let mut out = Vec::new();
            for layer in &b.layers {
                if !layer.cavity {
                    out.push((Some(layer.material.clone()), from, layer.thickness));
                }
                from += layer.thickness;
            }
            out
        }
    };
    layers
        .into_iter()
        .map(|(material, from, depth)| {
            let foot = top - from - depth;
            let frame = Frame::new(Vec3::new(0.0, 0.0, foot * MM), Vec3::Z, Vec3::X).map_err(failed)?;
            Ok((material, springrep::sweep::extrude(&profile, &frame, depth * MM, "slab").map_err(failed)?))
        })
        .collect()
}

/// Whether the planar `parts` an element evaluates to are its definition's own (`own`), uncut:
/// the same count, each of the same volume. Then its exact solids stand for them.
fn uncut(parts: &[&Part], own: &[f64]) -> bool {
    parts.len() == own.len() && parts.iter().zip(own).all(|(p, v)| (p.solid.volume() - v).abs() <= 1e-9 * v.abs().max(1.0))
}

/// The exact parts of element `id`, in its tile's frame: from its definition where it is round
/// and uncut, else each sewn from its planar solid.
pub fn parts(model: &Model, id: ElementId) -> Result<Vec<ExactPart>, Refusal> {
    let e = model.get(id).ok_or(Refusal::Missing)?;
    let planar = model.evaluate(id)?;
    let (body, rest): (Vec<&Part>, Vec<&Part>) = planar.iter().partition(|p| p.class != crate::reinforcement::BAR_CLASS);
    let sewn = |p: &Part| Ok(ExactPart { class: p.class, material: p.material.clone(), brep: from_solid(&p.solid)?, curved: false });
    let mut out = Vec::new();
    let made: Option<Vec<(Option<String>, Brep)>> = match &e.shape {
        Shape::Linear(l) => match round_section(&l.profile) {
            Some((outer, inner)) if uncut(&body, &[linear_solid(l, None, None)?.volume()]) => Some(vec![(None, round_member(l, outer, inner)?)]),
            _ => None,
        },
        Shape::Planar(p) if has_curves(e) && matches!(&p.form, PlanarForm::Slab { slope: None, .. }) => {
            let own: Vec<f64> = planar_parts(p)?.iter().map(|(_, s)| s.volume()).collect();
            if uncut(&body, &own) { Some(level_slab(p)?) } else { None }
        }
        _ => None,
    };
    match made {
        Some(exact) => {
            for ((material, brep), p) in exact.into_iter().zip(&body) {
                out.push(ExactPart { class: p.class, material: material.or_else(|| p.material.clone()), brep, curved: true });
            }
        }
        None => {
            for p in &body {
                out.push(sewn(p)?);
            }
        }
    }
    for p in rest {
        out.push(sewn(p)?);
    }
    Ok(out)
}

/// The exact volume of `parts`, in cubic metres.
pub fn volume(parts: &[ExactPart]) -> f64 {
    parts.iter().map(|p| springrep::measure::volume(&p.brep)).sum::<f64>() / (MM * MM * MM)
}

/// The model as a STEP AP214 file named `name`: each element a product of its exact parts,
/// placed at its tile's origin, in millimetres. Also the ids of the elements left out because
/// they have no solid or it could not be made exact.
pub fn step(model: &Model, name: &str) -> Result<(String, Vec<ElementId>), Refusal> {
    use springrep::step::{Instance, Part as StepPart};
    let mut bodies = Vec::new();
    let mut left_out = Vec::new();
    for e in model.iter() {
        if matches!(e.shape, Shape::Connection(_)) && model.evaluate(e.id).map_or(true, |p| p.is_empty()) {
            continue;
        }
        match parts(model, e.id) {
            Ok(parts) if !parts.is_empty() => {
                let mut brep = Brep::new();
                for p in &parts {
                    brep.append(&p.brep);
                }
                let label = if e.name.is_empty() { format!("#{}", e.id.0) } else { e.name.clone() };
                bodies.push((label, brep, model.grid.origin(e.tile)));
            }
            _ => left_out.push(e.id),
        }
    }
    let step_parts: Vec<StepPart> = bodies.iter().map(|(label, brep, _)| StepPart::new(label, brep, None)).collect();
    let instances: Vec<Instance> = bodies.iter().enumerate().map(|(i, (label, _, origin))| Instance::new(label, i, Transform::translation(v3(*origin)))).collect();
    let text = springrep::step::write_assembly(name, &step_parts, &instances).map_err(failed)?;
    Ok((text, left_out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{PlanarForm, Slope};
    use crate::{Element, LinearDef, PlanarDef, TileGrid};

    fn model() -> Model {
        Model::new(TileGrid::new(1000).unwrap())
    }

    fn add(m: &mut Model, shape: Shape) -> ElementId {
        let e = Element::in_project(&m.grid, "e", None, shape);
        m.add(e).unwrap()
    }

    fn column(profile: ProfileDef) -> Shape {
        Shape::Linear(LinearDef { start: DVec3::new(2.0, 3.0, 0.0), end: DVec3::new(2.0, 3.0, 3.0), profile, roll: 0.0, role: Default::default() })
    }

    #[test]
    fn a_round_column_is_a_true_cylinder() {
        let mut m = model();
        let id = add(&mut m, column(ProfileDef::Circle { diameter: 0.4 }));
        let parts = parts(&m, id).unwrap();
        assert!(parts[0].curved);
        let exact = std::f64::consts::PI * 0.2 * 0.2 * 3.0;
        assert!((volume(&parts) - exact).abs() < 1e-9 * exact, "{} against {exact}", volume(&parts));
        // the chords the planar solid draws it in lie inside the circle
        let chorded: f64 = m.evaluate(id).unwrap().iter().map(|p| p.solid.volume()).sum();
        assert!(chorded < exact && chorded > exact * 0.999);
    }

    #[test]
    fn a_tube_keeps_its_bore() {
        let mut m = model();
        let shape = bim_library::ProfileShape::Chs { d: 0.2, t: 0.01 };
        let id = add(&mut m, column(ProfileDef::Catalogue { designation: "CHS 200x10".into(), shape }));
        let exact = std::f64::consts::PI * (0.1 * 0.1 - 0.09 * 0.09) * 3.0;
        assert!((volume(&parts(&m, id).unwrap()) - exact).abs() < 1e-9 * exact);
    }

    #[test]
    fn a_slab_with_a_bowed_edge_has_its_arc() {
        let mut m = model();
        // a 4 x 2 m slab whose far edge, from (4, 2) to (0, 2), bows out into a half circle
        // (bulge 1) of radius 2
        let outline = vec![DVec2::ZERO, DVec2::new(4.0, 0.0), DVec2::new(4.0, 2.0), DVec2::new(0.0, 2.0)];
        let bulges = vec![0.0, 0.0, 1.0, 0.0];
        let slab = PlanarDef { form: PlanarForm::Slab { outline, holes: Vec::new(), top: 3.0, bulges, role: Default::default(), slope: None }, thickness: 0.25, openings: Vec::new(), build_up: None };
        let id = add(&mut m, Shape::Planar(slab));
        let e = m.get(id).unwrap();
        assert!(has_curves(e));
        let parts = parts(&m, id).unwrap();
        assert!(parts[0].curved);
        let exact = (4.0 * 2.0 + std::f64::consts::PI * 2.0 * 2.0 / 2.0) * 0.25;
        assert!((volume(&parts) - exact).abs() < 1e-9 * exact, "{} against {exact}", volume(&parts));
    }

    #[test]
    fn a_flat_element_is_sewn_from_its_planar_solid() {
        let mut m = model();
        let outline = vec![DVec2::ZERO, DVec2::new(5.0, 0.0), DVec2::new(5.0, 4.0), DVec2::new(0.0, 4.0)];
        let roof = PlanarDef { form: PlanarForm::Slab { outline, holes: vec![vec![DVec2::new(1.0, 1.0), DVec2::new(1.0, 2.0), DVec2::new(2.0, 2.0), DVec2::new(2.0, 1.0)]], top: 3.0, bulges: Vec::new(), role: Default::default(), slope: Some(Slope { edge: 0, angle: 0.5 }) }, thickness: 0.2, openings: Vec::new(), build_up: None };
        let id = add(&mut m, Shape::Planar(roof));
        let parts = parts(&m, id).unwrap();
        assert!(!parts[0].curved);
        let planar: f64 = m.evaluate(id).unwrap().iter().map(|p| p.solid.volume()).sum();
        assert!((volume(&parts) - planar).abs() < 1e-9 * planar);
    }

    #[test]
    fn the_model_is_written_as_one_step_file_with_a_product_per_element() {
        let mut m = model();
        add(&mut m, column(ProfileDef::Circle { diameter: 0.3 }));
        add(&mut m, column(ProfileDef::Rectangle { width: 0.3, height: 0.3 }));
        let (text, left_out) = step(&m, "project").unwrap();
        assert!(left_out.is_empty());
        assert!(text.starts_with("ISO-10303-21;"));
        assert_eq!(text.matches("MANIFOLD_SOLID_BREP").count(), 2);
        assert!(text.contains("CYLINDRICAL_SURFACE"), "the round column is a cylinder");
        let back = springrep::step::read(text.as_bytes(), "back").unwrap();
        let v = springrep::measure::volume(&back.brep) / 1e9;
        let exact = std::f64::consts::PI * 0.15 * 0.15 * 3.0 + 0.3 * 0.3 * 3.0;
        assert!((v - exact).abs() < 1e-9 * exact, "{v} against {exact}");
    }
}
