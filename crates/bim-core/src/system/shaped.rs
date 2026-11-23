//! Standing systems that are not a rectangle: a wall with a corner off, a gable.
//!
//! A standing system with an `outline` has it in its own face: every point along its line from
//! its start and up from its base. The system is made as the rectangle its line and its height
//! make, with its openings framed as its type frames them, and then cut to the outline: a part
//! the outline crosses is cut on it, one outside it is left out. What stands past the ends of the
//! rectangle (a layer that runs on around a corner) is kept as it is. No plate is laid along a
//! sloping cut: the members end on it.

use crate::boolean::{boolean, BooleanOp};
use crate::element::{PartSolid, SystemDef};
use crate::profile::Profile;
use crate::solid::{Axes, Solid};
use crate::{Refusal, LINEAR_TOLERANCE};
use glam::DVec2;

/// How far either side of a standing system the solid it is cut with reaches (metres).
const THROUGH: f64 = 5.0;

/// Standing system `s` with an outline as the rectangle it is made as, and the outline in its
/// face. `None` for one without an outline, and for one whose outline is that rectangle.
pub(super) fn bounded(s: &SystemDef) -> Result<Option<(SystemDef, Vec<DVec2>)>, Refusal> {
    if s.outline.is_empty() || s.lies() {
        return Ok(None);
    }
    if s.outline.len() < 3 {
        return Err(Refusal::TooFewPoints);
    }
    let length = (s.end - s.start).length();
    let tol = LINEAR_TOLERANCE;
    // within the rectangle, and reaching all of it: the line and the height say how large it is
    let (lo, hi) = s.outline.iter().fold((DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    if lo.x < -tol || lo.y < -tol || hi.x > length + tol || hi.y > s.height + tol || hi.x - lo.x < length - tol || hi.y - lo.y < s.height - tol {
        return Err(Refusal::Degenerate);
    }
    let rectangle = SystemDef { outline: Vec::new(), ..s.clone() };
    let corner = |p: &DVec2| (p.x.abs() < tol || (p.x - length).abs() < tol) && (p.y.abs() < tol || (p.y - s.height).abs() < tol);
    if s.outline.len() == 4 && s.outline.iter().all(corner) {
        return Ok(Some((rectangle, Vec::new())));
    }
    Ok(Some((rectangle, s.outline.clone())))
}

/// The `parts` of standing system `s` (made as its rectangle) cut to `outline` in its face.
pub(super) fn cut(s: &SystemDef, outline: &[DVec2], parts: Vec<PartSolid>) -> Result<Vec<PartSolid>, Refusal> {
    if outline.is_empty() {
        return Ok(parts);
    }
    let dir = (s.end - s.start).try_normalize().ok_or(Refusal::ZeroLength)?;
    let length = (s.end - s.start).length();
    let left = dir.perp();
    // the outline through the whole of the system: along its line, up, and across it
    let axes = Axes::new((s.start + left * THROUGH).extend(s.base), dir.extend(0.0), (-left).extend(0.0))?;
    let ring = if area2(outline) < 0.0 { outline.iter().rev().copied().collect() } else { outline.to_vec() };
    let tool = Solid::extrude(&Profile::new(ring, Vec::new())?, &axes, 2.0 * THROUGH)?;
    let mut kept = Vec::with_capacity(parts.len());
    for part in parts {
        // where it lies in the face, as far as the rectangle goes
        let (mut lo, mut hi) = (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY));
        for q in &part.solid.positions {
            let at = DVec2::new((DVec2::new(q.x, q.y) - s.start).dot(dir), q.z - s.base);
            (lo, hi) = (lo.min(at), hi.max(at));
        }
        let (a, b) = (DVec2::new(lo.x.max(0.0), lo.y.max(0.0)), DVec2::new(hi.x.min(length), hi.y.min(s.height)));
        match lies(a, b, outline) {
            Lies::Inside => kept.push(part),
            Lies::Outside => {}
            Lies::Across => match boolean(&part.solid, &tool, BooleanOp::Intersection) {
                Ok(solid) => kept.push(PartSolid { solid, ..part }),
                Err(Refusal::Empty) => {}
                Err(refusal) => return Err(refusal),
            },
        }
    }
    Ok(kept)
}

/// Twice the area of `ring`, above zero when it runs counter-clockwise.
fn area2(ring: &[DVec2]) -> f64 {
    (0..ring.len()).map(|i| ring[i].perp_dot(ring[(i + 1) % ring.len()])).sum()
}

/// How a rectangle lies to an outline.
enum Lies {
    Inside,
    Outside,
    Across,
}

/// How the rectangle from `a` to `b` lies to `ring`: across it when an edge of the ring runs
/// through its inside, else in it or out of it as its middle is. A rectangle with no inside (a
/// part past the system's end) counts as in it.
fn lies(a: DVec2, b: DVec2, ring: &[DVec2]) -> Lies {
    let tol = 1e-7;
    let (lo, hi) = (a + DVec2::splat(tol), b - DVec2::splat(tol));
    if hi.x <= lo.x || hi.y <= lo.y {
        return Lies::Inside;
    }
    let n = ring.len();
    let through = |p: DVec2, q: DVec2| {
        // the part of the edge inside the rectangle, by its parameter along the edge
        let d = q - p;
        let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
        for (k, m) in [(-d.x, p.x - lo.x), (d.x, hi.x - p.x), (-d.y, p.y - lo.y), (d.y, hi.y - p.y)] {
            if k.abs() < 1e-15 {
                if m < 0.0 {
                    return false;
                }
            } else if k < 0.0 {
                t0 = t0.max(m / k);
            } else {
                t1 = t1.min(m / k);
            }
        }
        (t1 - t0) * d.length() > tol
    };
    if (0..n).any(|i| through(ring[i], ring[(i + 1) % n])) {
        return Lies::Across;
    }
    let c = (lo + hi) / 2.0;
    let mut inside = false;
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        if (p.y > c.y) != (q.y > c.y) && c.x < p.x + (c.y - p.y) / (q.y - p.y) * (q.x - p.x) {
            inside = !inside;
        }
    }
    if inside {
        Lies::Inside
    } else {
        Lies::Outside
    }
}

#[cfg(test)]
mod tests {
    use super::super::parts;
    use crate::element::{Opening, SystemDef};
    use crate::Refusal;
    use bim_library::Library;
    use glam::DVec2;

    fn frame(outline: Vec<DVec2>, openings: Vec<Opening>) -> SystemDef {
        let generic = SystemDef { start: DVec2::new(2.0, 1.0), end: DVec2::new(2.0, 5.8), base: 0.5, height: 2.7, spacing: DVec2::new(1.5, 1.5), member: DVec2::new(0.05, 0.15), panel: 0.02, system_type: None, openings, breaks: Vec::new(), outline, division: None };
        generic.typed_from(&Library::starter(), "system.timber-frame-140").unwrap()
    }

    fn volume(s: &SystemDef) -> f64 {
        parts(s).unwrap().iter().map(|p| p.solid.volume()).sum()
    }

    #[test]
    fn a_frame_with_a_corner_off_is_cut_on_its_outline() {
        let p = DVec2::new;
        let whole = frame(Vec::new(), Vec::new());
        // the rectangle as an outline is the rectangle
        let same = frame(vec![p(0.0, 0.0), p(4.8, 0.0), p(4.8, 2.7), p(0.0, 2.7)], Vec::new());
        assert_eq!(parts(&same).unwrap().len(), parts(&whole).unwrap().len());
        // a corner of 1.2 by 0.9 off its top at its end: nothing reaches into it, and what is
        // left weighs less by about the corner's share of the wall
        let cut = frame(vec![p(0.0, 0.0), p(4.8, 0.0), p(4.8, 1.8), p(3.6, 2.7), p(0.0, 2.7)], Vec::new());
        let made = parts(&cut).unwrap();
        for part in &made {
            for q in &part.solid.positions {
                let (u, v) = (q.y - 1.0, q.z - 0.5);
                // above the line from (4.8, 1.8) to (3.6, 2.7): v > 1.8 + (4.8 - u) * 0.75
                assert!(u > 4.8 + 1e-6 || v <= 1.8 + (4.8 - u) * 0.75 + 1e-6, "{u} {v}");
            }
        }
        let share = volume(&cut) / volume(&whole);
        let corner = 1.0 - 0.5 * 1.2 * 0.9 / (4.8 * 2.7);
        assert!((share - corner).abs() < 0.03, "{share} {corner}");
        // an opening in it is framed as in any frame
        let window = Opening { along: 1.2, sill: 0.9, width: 1.2, height: 1.2, framing: None };
        let with = frame(vec![p(0.0, 0.0), p(4.8, 0.0), p(4.8, 1.8), p(3.6, 2.7), p(0.0, 2.7)], vec![window]);
        assert!(parts(&with).unwrap().len() > made.len());
        // an outline that does not fill its rectangle, or leaves it, is refused
        assert_eq!(parts(&frame(vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 2.7), p(0.0, 2.7)], Vec::new())).map(|p| p.len()), Err(Refusal::Degenerate));
        assert_eq!(parts(&frame(vec![p(0.0, 0.0), p(4.8, 0.0)], Vec::new())).map(|p| p.len()), Err(Refusal::TooFewPoints));
    }
}
