//! The reinforcement of a concrete member with a rectangular section: bars along its top and its
//! bottom and closed stirrups round them, given by their numbers, diameters and cover. From these
//! and the member's sizes come the bars to cut, with their lengths and weights: the bar schedule.
//!
//! A first measure of it, for quantities and drawings: the bars are straight and as long as the
//! member less its cover at both ends, without laps, hooks or bends; a stirrup is the section's
//! outline inside the cover, with two hooks. Editable detailing defaults, not a checked design.

use crate::profile::Profile;
use crate::Refusal;
use serde::{Deserialize, Serialize};

/// The density of reinforcing steel (kg/m³).
pub const STEEL_DENSITY: f64 = 7850.0;
/// The IFC class of a bar or a stirrup as a part of its member, and what it is made of (a
/// material key of the project's library).
pub const BAR_CLASS: &str = "IfcReinforcingBar";
pub const BAR_MATERIAL: &str = "steel";
/// The length of each of a stirrup's two hooks, in bar diameters.
pub const HOOK_DIAMETERS: f64 = 10.0;
/// The least clear distance between two bars beside each other (metres), where their own
/// diameter is not more.
pub const CLEAR: f64 = 0.02;

/// A row of equal bars: how many, and their diameter (metres).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bars {
    pub count: u32,
    pub diameter: f64,
}

/// A member's reinforcement (metres).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reinforcement {
    /// The concrete over the stirrups, on every side and at both ends.
    pub cover: f64,
    /// The bars along the top and along the bottom of the section.
    pub top: Bars,
    pub bottom: Bars,
    /// The stirrups' diameter, and how far apart they stand.
    pub stirrup: f64,
    pub spacing: f64,
}

impl Default for Reinforcement {
    /// 30 mm cover, 2 Ø12 on top, 3 Ø16 below, stirrups Ø8 every 150 mm.
    fn default() -> Self {
        Self { cover: 0.03, top: Bars { count: 2, diameter: 0.012 }, bottom: Bars { count: 3, diameter: 0.016 }, stirrup: 0.008, spacing: 0.15 }
    }
}

/// What a bar is in its member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarKind {
    Top,
    Bottom,
    Stirrup,
    /// A bar of the bottom net or of the top net of a slab or a wall.
    NetBottom,
    NetTop,
    /// A bar drawn besides the usual ones: see [`Extra`].
    Extra,
}

/// How a bar is bent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BarShape {
    Straight,
    /// A closed stirrup: its width and its height on its centre line.
    Stirrup(f64, f64),
    /// A hairpin, a U: the length of each leg, and how far they lie apart.
    Hairpin(f64, f64),
}

/// How the bars along a line in a slab are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineForm {
    /// U-bars round the slab's edge: a leg in its top and one in its bottom, towards its inside.
    Hairpin,
    /// Straight bars in its top or in its bottom, half to each side of the line.
    Top,
    Bottom,
}

/// Reinforcement drawn besides an element's usual one (metres).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Extra {
    /// In a member: stirrups over the stretch `from` to `to` along it from its start, which
    /// stand there instead of its usual ones.
    Stirrups { from: f64, to: f64, diameter: f64, spacing: f64 },
    /// In a member: `count` more bars along its top or its bottom over the stretch, a row
    /// inside its usual bars.
    Bars { from: f64, to: f64, count: u32, diameter: f64, top: bool },
    /// In a slab: bars square to the line from `a` to `b`, `spacing` apart along it (in the
    /// middle of it, the cover inside its ends), each
    /// `length` long (a hairpin: each leg). The line is given from the first corner of the
    /// slab's outline, along its first edge and square to it, so it moves and turns with the slab.
    Line { a: glam::DVec2, b: glam::DVec2, diameter: f64, spacing: f64, length: f64, form: LineForm },
}

/// Equal bars of a member, as its bar schedule lists them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarRow {
    pub kind: BarKind,
    pub diameter: f64,
    pub count: u32,
    /// The length one bar is cut to (metres).
    pub length: f64,
    pub shape: BarShape,
}

impl BarRow {
    /// The weight of all bars of the row (kg).
    pub fn weight(&self) -> f64 {
        std::f64::consts::PI * self.diameter * self.diameter / 4.0 * self.length * f64::from(self.count) * STEEL_DENSITY
    }
}

impl Reinforcement {
    /// The bars of a member `length` long with section `profile`: its top bars, its bottom bars
    /// and its stirrups (a row with no bars is left out). Refused when the section is no
    /// rectangle, a size is not positive, or the bars do not fit beside each other in it.
    pub fn bars(&self, profile: &Profile, length: f64) -> Result<Vec<BarRow>, Refusal> {
        let positive = |v: f64| v.is_finite() && v > crate::LINEAR_TOLERANCE;
        if ![self.cover, self.stirrup, self.spacing, length].into_iter().all(positive) || [self.top, self.bottom].iter().any(|b| b.count > 0 && !positive(b.diameter)) {
            return Err(Refusal::NotPositive);
        }
        let outer = profile.outer();
        let lo = outer.iter().fold(glam::DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
        let hi = outer.iter().fold(glam::DVec2::splat(f64::NEG_INFINITY), |a, p| a.max(*p));
        let (width, height) = (hi.x - lo.x, hi.y - lo.y);
        if outer.len() != 4 || !profile.holes().is_empty() || (profile.area() - width * height).abs() > 1e-9 {
            return Err(Refusal::NotRectangular);
        }
        // inside the stirrups
        let (inner, tall) = (width - 2.0 * (self.cover + self.stirrup), height - 2.0 * (self.cover + self.stirrup));
        let straight = length - 2.0 * self.cover;
        if !positive(straight) || !positive(inner) || !positive(tall) {
            return Err(Refusal::BarsDoNotFit);
        }
        let mut rows = Vec::new();
        for (kind, bars) in [(BarKind::Top, self.top), (BarKind::Bottom, self.bottom)] {
            if bars.count == 0 {
                continue;
            }
            let needed = f64::from(bars.count) * bars.diameter + f64::from(bars.count - 1) * bars.diameter.max(CLEAR);
            if needed > inner + 1e-9 || bars.diameter > tall {
                return Err(Refusal::BarsDoNotFit);
            }
            rows.push(BarRow { kind, diameter: bars.diameter, count: bars.count, length: straight, shape: BarShape::Straight });
        }
        // a stirrup: round the section on its centre line, and its two hooks
        let (w, h) = (width - 2.0 * self.cover - self.stirrup, height - 2.0 * self.cover - self.stirrup);
        let count = (straight / self.spacing + 1e-9).floor() as u32 + 1;
        rows.push(BarRow { kind: BarKind::Stirrup, diameter: self.stirrup, count, length: 2.0 * (w + h) + 2.0 * HOOK_DIAMETERS * self.stirrup, shape: BarShape::Stirrup(w, h) });
        Ok(rows)
    }

    /// Where the bars along the member lie in its section (the profile's own axes): the centre
    /// and the diameter of each, the top row first, each row spread evenly between the
    /// stirrup's legs (one bar alone in the middle). Refused as [`Reinforcement::bars`].
    pub fn centres(&self, profile: &Profile) -> Result<Vec<(glam::DVec2, f64)>, Refusal> {
        self.bars(profile, 1.0 + 2.0 * self.cover)?;
        let outer = profile.outer();
        let lo = outer.iter().fold(glam::DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
        let hi = outer.iter().fold(glam::DVec2::splat(f64::NEG_INFINITY), |a, p| a.max(*p));
        let inset = self.cover + self.stirrup;
        let mut centres = Vec::new();
        for (bars, top) in [(self.top, true), (self.bottom, false)] {
            let half = bars.diameter / 2.0;
            let y = if top { hi.y - inset - half } else { lo.y + inset + half };
            let (first, last) = (lo.x + inset + half, hi.x - inset - half);
            for i in 0..bars.count {
                let x = if bars.count == 1 { (first + last) / 2.0 } else { first + (last - first) * f64::from(i) / f64::from(bars.count - 1) };
                centres.push((glam::DVec2::new(x, y), bars.diameter));
            }
        }
        Ok(centres)
    }

    /// Where the stirrups stand along a member `length` long with `extras`: its usual ones (see
    /// [`Reinforcement::stirrups`]) outside every stretch of stirrups drawn, and in each stretch
    /// its own, their spacing apart in the middle of the stretch. Each with its diameter and
    /// the extra it is of (`None`: a usual one).
    pub fn stirrups_in(&self, length: f64, extras: &[Extra]) -> Vec<(f64, f64, Option<usize>)> {
        let ends = (self.cover, length - self.cover);
        let stretches: Vec<(usize, f64, f64, f64, f64)> = extras
            .iter()
            .enumerate()
            .filter_map(|(k, e)| match *e {
                Extra::Stirrups { from, to, diameter, spacing } => Some((k, from.min(to).max(ends.0), from.max(to).min(ends.1), diameter, spacing)),
                _ => None,
            })
            .filter(|(_, lo, hi, _, spacing)| hi > lo && *spacing > crate::LINEAR_TOLERANCE)
            .collect();
        let mut all: Vec<(f64, f64, Option<usize>)> = self.stirrups(length).into_iter().filter(|s| !stretches.iter().any(|(_, lo, hi, _, _)| *s >= lo - 1e-9 && *s <= hi + 1e-9)).map(|s| (s, self.stirrup, None)).collect();
        for (k, lo, hi, diameter, spacing) in stretches {
            let count = ((hi - lo) / spacing + 1e-9).floor() as usize + 1;
            let first = lo + (hi - lo - (count - 1) as f64 * spacing) / 2.0;
            all.extend((0..count).map(|i| (first + i as f64 * spacing, diameter, Some(k))));
        }
        all
    }

    /// The bars drawn besides the usual ones along a member `length` long with section
    /// `profile`: of every stretch of bars of `extras`, each bar's place in the section (a row
    /// inside the usual bars, spread as they are), its diameter, where it starts and ends along
    /// the member (inside the cover), and the extra it is of.
    pub fn extra_bars(&self, profile: &Profile, length: f64, extras: &[Extra]) -> Vec<(usize, glam::DVec2, f64, f64, f64)> {
        let outer = profile.outer();
        let lo = outer.iter().fold(glam::DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
        let hi = outer.iter().fold(glam::DVec2::splat(f64::NEG_INFINITY), |a, p| a.max(*p));
        let inset = self.cover + self.stirrup;
        let mut bars = Vec::new();
        for (k, e) in extras.iter().enumerate() {
            let Extra::Bars { from, to, count, diameter, top } = *e else { continue };
            let (start, end) = (from.min(to).max(self.cover), from.max(to).min(length - self.cover));
            if end <= start {
                continue;
            }
            let usual = if top { self.top } else { self.bottom };
            let inside = inset + if usual.count > 0 { usual.diameter + diameter.max(CLEAR) } else { 0.0 } + diameter / 2.0;
            let y = if top { hi.y - inside } else { lo.y + inside };
            let (first, last) = (lo.x + inset + diameter / 2.0, hi.x - inset - diameter / 2.0);
            for i in 0..count {
                let x = if count == 1 { (first + last) / 2.0 } else { first + (last - first) * f64::from(i) / f64::from(count - 1) };
                bars.push((k, glam::DVec2::new(x, y), diameter, start, end));
            }
        }
        bars
    }

    /// The bars of a member with `extras`, as its bar schedule lists them: those of
    /// [`Reinforcement::bars`], its usual stirrups only where no stretch of stirrups stands
    /// instead, then a row for every stretch of stirrups and of bars. Refused as
    /// [`Reinforcement::bars`], and when an extra has a size that is not positive, lies outside
    /// the member, or its bars do not fit beside each other.
    pub fn rows(&self, profile: &Profile, length: f64, extras: &[Extra]) -> Result<Vec<BarRow>, Refusal> {
        let mut rows = self.bars(profile, length)?;
        let positive = |v: f64| v.is_finite() && v > crate::LINEAR_TOLERANCE;
        let stirrups = self.stirrups_in(length, extras);
        let usual = stirrups.iter().filter(|s| s.2.is_none()).count() as u32;
        match rows.last_mut() {
            Some(row) if usual > 0 => row.count = usual,
            _ => {
                rows.pop();
            }
        }
        let outer = profile.outer();
        let lo = outer.iter().fold(glam::DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
        let hi = outer.iter().fold(glam::DVec2::splat(f64::NEG_INFINITY), |a, p| a.max(*p));
        let extra_bars = self.extra_bars(profile, length, extras);
        for (k, e) in extras.iter().enumerate() {
            match *e {
                Extra::Stirrups { diameter, spacing, .. } => {
                    let count = stirrups.iter().filter(|s| s.2 == Some(k)).count() as u32;
                    let (w, h) = (hi.x - lo.x - 2.0 * self.cover - diameter, hi.y - lo.y - 2.0 * self.cover - diameter);
                    if !positive(diameter) || !positive(spacing) || count == 0 {
                        return Err(Refusal::NotPositive);
                    }
                    if !positive(w) || !positive(h) {
                        return Err(Refusal::BarsDoNotFit);
                    }
                    rows.push(BarRow { kind: BarKind::Stirrup, diameter, count, length: 2.0 * (w + h) + 2.0 * HOOK_DIAMETERS * diameter, shape: BarShape::Stirrup(w, h) });
                }
                Extra::Bars { count, diameter, .. } => {
                    let Some(&(_, _, _, start, end)) = extra_bars.iter().find(|b| b.0 == k) else { return Err(Refusal::NotPositive) };
                    if !positive(diameter) {
                        return Err(Refusal::NotPositive);
                    }
                    let inner = hi.x - lo.x - 2.0 * (self.cover + self.stirrup);
                    if f64::from(count) * diameter + f64::from(count - 1) * diameter.max(CLEAR) > inner + 1e-9 {
                        return Err(Refusal::BarsDoNotFit);
                    }
                    rows.push(BarRow { kind: BarKind::Extra, diameter, count, length: end - start, shape: BarShape::Straight });
                }
                Extra::Line { .. } => {}
            }
        }
        Ok(rows)
    }

    /// The bars as solids in a member's own frame, with its `extras`: every bar along it a
    /// round rod inside the cover at both ends, every stirrup a closed ring round the section,
    /// square in section and as thick as its diameter (its hooks and bends are not made).
    /// Refused as [`Reinforcement::rows`].
    pub fn solids(&self, member: &crate::element::LinearDef, extras: &[Extra]) -> Result<Vec<(BarKind, crate::Solid)>, Refusal> {
        use crate::solid::{Axes, Solid};
        let profile = member.profile.profile()?;
        let (axes, length) = crate::element::linear_axes(member)?;
        let centres = self.centres(&profile)?;
        self.rows(&profile, length, extras)?;
        let outer = profile.outer();
        let lo = outer.iter().fold(glam::DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
        let hi = outer.iter().fold(glam::DVec2::splat(f64::NEG_INFINITY), |a, p| a.max(*p));
        let mut solids = Vec::new();
        for (_, centre, diameter, start, end) in self.extra_bars(&profile, length, extras) {
            let at = Axes { origin: axes.point(centre) + axes.z * start, ..axes };
            solids.push((BarKind::Extra, Solid::extrude(&Profile::circle(diameter)?, &at, end - start)?));
        }
        for (i, (centre, diameter)) in centres.iter().enumerate() {
            let at = Axes { origin: axes.point(*centre) + axes.z * self.cover, ..axes };
            let kind = if i < self.top.count as usize { BarKind::Top } else { BarKind::Bottom };
            solids.push((kind, Solid::extrude(&Profile::circle(*diameter)?, &at, length - 2.0 * self.cover)?));
        }
        let rectangle = |inset: f64| vec![glam::DVec2::new(lo.x + inset, lo.y + inset), glam::DVec2::new(hi.x - inset, lo.y + inset), glam::DVec2::new(hi.x - inset, hi.y - inset), glam::DVec2::new(lo.x + inset, hi.y - inset)];
        for (s, diameter, _) in self.stirrups_in(length, extras) {
            let ring = Profile::new(rectangle(self.cover), vec![rectangle(self.cover + diameter)])?;
            let at = Axes { origin: axes.origin + axes.z * (s - diameter / 2.0), ..axes };
            solids.push((BarKind::Stirrup, Solid::extrude(&ring, &at, diameter)?));
        }
        Ok(solids)
    }

    /// Where the stirrups stand along a member `length` long, from its start: as many as
    /// [`Reinforcement::bars`] counts, their spacing apart and in the middle of the length
    /// inside the cover. Empty where that length is not positive.
    pub fn stirrups(&self, length: f64) -> Vec<f64> {
        let straight = length - 2.0 * self.cover;
        if !(straight > 0.0 && self.spacing > crate::LINEAR_TOLERANCE) {
            return Vec::new();
        }
        let count = (straight / self.spacing + 1e-9).floor() as u32 + 1;
        let first = self.cover + (straight - f64::from(count - 1) * self.spacing) / 2.0;
        (0..count).map(|i| first + f64::from(i) * self.spacing).collect()
    }
}

/// A net: bars of one diameter, `spacing` apart, both ways (metres).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Net {
    pub diameter: f64,
    pub spacing: f64,
}

/// The reinforcement of a slab or a wall: a net in its bottom and, when given, one in its top
/// (a wall: in its face on the left walking from its start to its end, and in the one on the
/// right), each `cover` inside the concrete (metres).
///
/// A first measure of it, as a member's: the bars are straight, run along the longest side and
/// square to it, and end the cover from every edge and every opening, without laps, hooks or
/// bends; an edge that bows is taken by its chords.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Nets {
    pub cover: f64,
    pub bottom: Net,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<Net>,
}

impl Default for Nets {
    /// 30 mm cover, Ø10 every 150 mm below and Ø8 every 150 mm on top.
    fn default() -> Self {
        Self { cover: 0.03, bottom: Net { diameter: 0.01, spacing: 0.15 }, top: Some(Net { diameter: 0.008, spacing: 0.15 }) }
    }
}

/// A bar of a net, in the plane of its slab (in plan) or its wall (along it and up from its
/// base).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetBar {
    /// Of the top net, else of the bottom one.
    pub top: bool,
    /// Square to the longest side, else along it.
    pub cross: bool,
    pub a: glam::DVec2,
    pub b: glam::DVec2,
    /// How far its centre lies under the top face (a wall: inside the face on its right).
    pub depth: f64,
    pub diameter: f64,
    /// The line it was drawn with (an entry of the element's extras); `None`: a bar of a net.
    pub extra: Option<usize>,
    /// A hairpin: how far under this leg its other leg lies. 0: a straight bar.
    pub drop: f64,
}

impl NetBar {
    /// The length it is cut to: a hairpin's two legs and what joins them.
    pub fn length(&self) -> f64 {
        if self.drop > 0.0 { 2.0 * self.a.distance(self.b) + self.drop } else { self.a.distance(self.b) }
    }
}

/// Where the line through `origin` along `u` (a unit way) lies inside `rings` (an outline and
/// its holes): from and to, as distances along it, between every odd and even crossing.
fn spans(rings: &[&[glam::DVec2]], origin: glam::DVec2, u: glam::DVec2) -> Vec<(f64, f64)> {
    let v = u.perp();
    let at = origin.dot(v);
    let mut crossings: Vec<f64> = rings
        .iter()
        .flat_map(|ring| (0..ring.len()).map(move |k| (ring[k], ring[(k + 1) % ring.len()])))
        .filter(|(p, q)| (p.dot(v) <= at) != (q.dot(v) <= at))
        .map(|(p, q)| p.dot(u) + (q.dot(u) - p.dot(u)) * (at - p.dot(v)) / (q.dot(v) - p.dot(v)) - origin.dot(u))
        .collect();
    crossings.sort_by(f64::total_cmp);
    crossings.chunks_exact(2).map(|pair| (pair[0], pair[1])).collect()
}

/// A bar shorter than this is not made (metres).
const SHORTEST: f64 = 0.1;

impl Nets {
    /// The bars of the nets in a plate of `outline` with `holes`, `thickness` thick: of each net
    /// the bars along the longest side of the outline, nearest the face, and those square to
    /// them inside these, every row spread in the middle of the room it has. Refused when a size
    /// is not positive or the nets do not fit in the thickness.
    pub fn bars(&self, outline: &[glam::DVec2], holes: &[Vec<glam::DVec2>], thickness: f64) -> Result<Vec<NetBar>, Refusal> {
        use glam::DVec2;
        let positive = |v: f64| v.is_finite() && v > crate::LINEAR_TOLERANCE;
        let nets = [(false, Some(self.bottom)), (true, self.top)];
        if !positive(self.cover) || !positive(thickness) || nets.iter().any(|(_, n)| n.is_some_and(|n| !positive(n.diameter) || !positive(n.spacing))) {
            return Err(Refusal::NotPositive);
        }
        if 2.0 * self.cover + 2.0 * self.bottom.diameter + self.top.map_or(0.0, |n| 2.0 * n.diameter) > thickness + 1e-9 {
            return Err(Refusal::BarsDoNotFit);
        }
        if outline.len() < 3 {
            return Err(Refusal::Degenerate);
        }
        // the longest side gives the way the first bars run
        let n = outline.len();
        let along = (0..n).map(|i| outline[(i + 1) % n] - outline[i]).max_by(|a, b| a.length_squared().total_cmp(&b.length_squared())).and_then(|d| d.try_normalize()).ok_or(Refusal::Degenerate)?;
        let rings: Vec<&[DVec2]> = std::iter::once(outline).chain(holes.iter().map(Vec::as_slice)).collect();
        let mut bars = Vec::new();
        for (top, net) in nets {
            let Some(net) = net else { continue };
            for cross in [false, true] {
                // `u` the way these bars run, `v` the way they are spread
                let (u, v) = if cross { (along.perp(), along) } else { (along, along.perp()) };
                let (lo, hi) = outline.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p.dot(v)), hi.max(p.dot(v))));
                let room = hi - lo - 2.0 * self.cover - net.diameter;
                if !positive(room) {
                    continue;
                }
                let count = (room / net.spacing + 1e-9).floor() as usize + 1;
                let first = lo + self.cover + net.diameter / 2.0 + (room - (count - 1) as f64 * net.spacing) / 2.0;
                let inside = self.cover + net.diameter * if cross { 1.5 } else { 0.5 };
                let depth = if top { inside } else { thickness - inside };
                for i in 0..count {
                    let at = first + i as f64 * net.spacing;
                    for (from, to) in spans(&rings, v * at, u) {
                        let (from, to) = (from + self.cover, to - self.cover);
                        if to - from >= SHORTEST {
                            bars.push(NetBar { top, cross, a: u * from + v * at, b: u * to + v * at, depth, diameter: net.diameter, extra: None, drop: 0.0 });
                        }
                    }
                }
            }
        }
        Ok(bars)
    }

    /// The plate of a slab or a wall as its nets lie in it: its outline and its holes (a wall:
    /// along it and up from its base, with `openings` as its holes), and how thick it is.
    /// Refused for a slab that slopes.
    fn plate(def: &crate::element::PlanarDef, openings: &[crate::element::Opening]) -> Result<(Vec<glam::DVec2>, Vec<Vec<glam::DVec2>>, f64), Refusal> {
        use crate::element::PlanarForm;
        use glam::DVec2;
        let rectangle = |lo: DVec2, hi: DVec2| vec![lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)];
        match &def.form {
            PlanarForm::Slab { slope: Some(_), .. } => Err(Refusal::NotRectangular),
            PlanarForm::Slab { outline, holes, bulges, .. } => Ok((crate::element::bowed(outline, bulges), holes.clone(), def.total_thickness())),
            PlanarForm::Wall { start, end, height, .. } => {
                let holes = openings.iter().map(|o| rectangle(DVec2::new(o.along, o.sill), DVec2::new(o.along + o.width, o.sill + o.height))).collect();
                Ok((rectangle(DVec2::ZERO, DVec2::new(start.distance(*end), *height)), holes, def.total_thickness()))
            }
        }
    }

    /// The bars of the nets in slab or wall `def` (a wall with `openings`, its own and those of
    /// the doors and windows standing in it): see [`Nets::bars`]. A slab's lie in plan, a wall's
    /// along it and up from its base. Then the bars of every line of `extras` drawn in a slab,
    /// inside both nets: straight ones half to each side of the line and no further than the
    /// slab reaches, hairpins from the line to the slab's inside. Refused as [`Nets::bars`],
    /// and when a line has a size that is not positive or no bar of it lies in the slab.
    pub fn laid(&self, def: &crate::element::PlanarDef, openings: &[crate::element::Opening], extras: &[Extra]) -> Result<Vec<NetBar>, Refusal> {
        use crate::element::PlanarForm;
        let (outline, holes, thickness) = Self::plate(def, openings)?;
        let mut bars = self.bars(&outline, &holes, thickness)?;
        let PlanarForm::Slab { outline: corners, .. } = &def.form else { return Ok(bars) };
        let positive = |v: f64| v.is_finite() && v > crate::LINEAR_TOLERANCE;
        let rings: Vec<&[glam::DVec2]> = std::iter::once(outline.as_slice()).chain(holes.iter().map(Vec::as_slice)).collect();
        // the slab's own frame: from its first corner, along its first edge
        let x = (corners[1] - corners[0]).try_normalize().ok_or(Refusal::Degenerate)?;
        let plan = |p: glam::DVec2| corners[0] + x * p.x + x.perp() * p.y;
        let (above, below) = (self.cover + self.top.map_or(0.0, |n| 2.0 * n.diameter), self.cover + 2.0 * self.bottom.diameter);
        for (k, e) in extras.iter().enumerate() {
            let Extra::Line { a, b, diameter, spacing, length, form } = *e else { continue };
            let (a, b) = (plan(a), plan(b));
            let Some(way) = (b - a).try_normalize() else { return Err(Refusal::ZeroLength) };
            if !positive(diameter) || !positive(spacing) || !positive(length) {
                return Err(Refusal::NotPositive);
            }
            let (top, bottom) = (above + diameter / 2.0, thickness - below - diameter / 2.0);
            if bottom - top < diameter {
                return Err(Refusal::BarsDoNotFit);
            }
            // along the line, the cover inside its ends
            let reach = a.distance(b) - 2.0 * self.cover;
            if !positive(reach) {
                return Err(Refusal::ZeroLength);
            }
            let count = (reach / spacing + 1e-9).floor() as usize + 1;
            let first = self.cover + (reach - (count - 1) as f64 * spacing) / 2.0;
            // a hairpin's legs point to the side of the line the slab lies on
            let within = |p: glam::DVec2| spans(&rings, p, way).iter().any(|(from, to)| *from <= 0.0 && *to >= 0.0);
            let side = if within((a + b) / 2.0 - way.perp() * 0.05) && !within((a + b) / 2.0 + way.perp() * 0.05) { -way.perp() } else { way.perp() };
            let made = bars.len();
            for i in 0..count {
                let at = a + way * (first + i as f64 * spacing);
                let (from, to, depth, drop) = match form {
                    LineForm::Hairpin => (self.cover + diameter / 2.0, self.cover + diameter / 2.0 + length, top, bottom - top),
                    LineForm::Top => (-length / 2.0, length / 2.0, top, 0.0),
                    LineForm::Bottom => (-length / 2.0, length / 2.0, bottom, 0.0),
                };
                for (lo, hi) in spans(&rings, at, side) {
                    let (from, to) = (from.max(lo + self.cover), to.min(hi - self.cover));
                    if to - from >= SHORTEST {
                        bars.push(NetBar { top: form != LineForm::Bottom, cross: false, a: at + side * from, b: at + side * to, depth, diameter, extra: Some(k), drop });
                    }
                }
            }
            if bars.len() == made {
                return Err(Refusal::BarsDoNotFit);
            }
        }
        Ok(bars)
    }

    /// The bars of [`Nets::laid`] as round rods in the element's own frame; a hairpin as its
    /// two legs and a rod between them at its end (its bends are not made).
    pub fn solids(&self, def: &crate::element::PlanarDef, openings: &[crate::element::Opening], extras: &[Extra]) -> Result<Vec<(BarKind, crate::Solid)>, Refusal> {
        use crate::element::PlanarForm;
        use crate::solid::{Axes, Solid};
        use glam::DVec3;
        let thickness = def.total_thickness();
        let mut solids = Vec::new();
        for bar in self.laid(def, openings, extras)? {
            let place = |p: glam::DVec2, depth: f64| match &def.form {
                PlanarForm::Slab { top, .. } => p.extend(top - depth),
                PlanarForm::Wall { start, end, base, .. } => {
                    let along = (*end - *start).normalize_or_zero();
                    (*start + along * p.x - along.perp() * (thickness / 2.0 - depth)).extend(base + p.y)
                }
            };
            let kind = match (bar.extra, bar.top) {
                (Some(_), _) => BarKind::Extra,
                (None, true) => BarKind::NetTop,
                (None, false) => BarKind::NetBottom,
            };
            let mut rod = |a: DVec3, b: DVec3| -> Result<(), Refusal> {
                let way = b - a;
                let side = if way.cross(DVec3::Z).length_squared() > 1e-12 { DVec3::Z } else { DVec3::X };
                solids.push((kind, Solid::extrude(&Profile::circle(bar.diameter)?, &Axes::new(a, side, way)?, way.length())?));
                Ok(())
            };
            rod(place(bar.a, bar.depth), place(bar.b, bar.depth))?;
            if bar.drop > 0.0 {
                rod(place(bar.a, bar.depth + bar.drop), place(bar.b, bar.depth + bar.drop))?;
                rod(place(bar.a, bar.depth + bar.drop), place(bar.a, bar.depth))?;
            }
        }
        Ok(solids)
    }
}

/// `bars` in the rows a bar schedule lists them in: equal bars of a net and a way, or of a line
/// drawn, together (lengths to the millimetre), in the order they come.
pub fn net_groups(bars: &[NetBar]) -> Vec<Vec<NetBar>> {
    let mm = |v: f64| (v * 1000.0).round() as i64;
    let mut rows: Vec<Vec<NetBar>> = Vec::new();
    for bar in bars {
        let same = |row: &&mut Vec<NetBar>| row[0].extra == bar.extra && row[0].top == bar.top && row[0].cross == bar.cross && row[0].diameter == bar.diameter && mm(row[0].length()) == mm(bar.length());
        match rows.iter_mut().find(same) {
            Some(row) => row.push(*bar),
            None => rows.push(vec![*bar]),
        }
    }
    rows
}

/// `bars` as a bar schedule lists them: a row for each of [`net_groups`].
pub fn net_rows(bars: &[NetBar]) -> Vec<BarRow> {
    net_groups(bars)
        .iter()
        .map(|row| {
            let bar = row[0];
            let kind = match (bar.extra, bar.top) {
                (Some(_), _) => BarKind::Extra,
                (None, true) => BarKind::NetTop,
                (None, false) => BarKind::NetBottom,
            };
            let shape = if bar.drop > 0.0 { BarShape::Hairpin(bar.a.distance(bar.b), bar.drop) } else { BarShape::Straight };
            BarRow { kind, diameter: bar.diameter, count: row.len() as u32, length: (bar.length() * 1000.0).round() / 1000.0, shape }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beam_lists_its_bars_with_lengths_and_weights() {
        // 300 × 500, 6 m long
        let section = Profile::rectangle(0.3, 0.5).unwrap();
        let rows = Reinforcement::default().bars(&section, 6.0).unwrap();
        assert_eq!(rows.iter().map(|r| (r.kind, r.count)).collect::<Vec<_>>(), [(BarKind::Top, 2), (BarKind::Bottom, 3), (BarKind::Stirrup, 40)]);
        // straight bars: the length less the cover at both ends
        assert!((rows[0].length - 5.94).abs() < 1e-12 && (rows[1].length - 5.94).abs() < 1e-12);
        // a stirrup: 2 × (232 + 432) on its centre line and two hooks of 80
        assert!((rows[2].length - (2.0 * (0.232 + 0.432) + 0.16)).abs() < 1e-12, "{}", rows[2].length);
        // Ø16 weighs 1.578 kg/m, Ø12 0.888 and Ø8 0.395
        assert!((rows[1].weight() - 3.0 * 5.94 * 1.578).abs() < 0.02, "{}", rows[1].weight());
        assert!((rows[0].weight() - 2.0 * 5.94 * 0.888).abs() < 0.02, "{}", rows[0].weight());
        assert!((rows[2].weight() - 40.0 * 1.488 * 0.395).abs() < 0.05, "{}", rows[2].weight());
    }

    #[test]
    fn the_bars_lie_inside_the_stirrups_and_the_stirrups_in_the_middle_of_the_length() {
        let section = Profile::rectangle(0.3, 0.5).unwrap();
        let r = Reinforcement::default();
        let centres = r.centres(&section).unwrap();
        let lo = section.outer().iter().fold(glam::DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
        // two on top, 30 + 8 + 6 from the top and the sides; three below with one in the middle
        let at: Vec<(i64, i64, i64)> = centres.iter().map(|(c, d)| (((c.x - lo.x) * 1000.0).round() as i64, ((c.y - lo.y) * 1000.0).round() as i64, (d * 1000.0).round() as i64)).collect();
        assert_eq!(at, [(44, 456, 12), (256, 456, 12), (46, 46, 16), (150, 46, 16), (254, 46, 16)]);
        // 40 stirrups at 150 take 5.85 of the 5.94 inside the cover: 45 left at each end
        let stirrups = r.stirrups(6.0);
        assert_eq!(stirrups.len(), 40);
        assert!((stirrups[0] - 0.075).abs() < 1e-9 && (stirrups[39] - 5.925).abs() < 1e-9 && (stirrups[1] - stirrups[0] - 0.15).abs() < 1e-9, "{stirrups:?}");
        assert!(r.stirrups(0.05).is_empty());
        assert_eq!(r.centres(&Profile::circle(0.4).unwrap()), Err(Refusal::NotRectangular));
    }

    #[test]
    fn the_bars_of_a_beam_are_solids_inside_its_concrete() {
        use crate::element::LinearDef;
        use crate::profile::ProfileDef;
        let beam = LinearDef { start: glam::DVec3::new(1.0, 2.0, 3.0), end: glam::DVec3::new(7.0, 2.0, 3.0), profile: ProfileDef::Rectangle { width: 0.3, height: 0.5 }, roll: 0.0 };
        let r = Reinforcement::default();
        let solids = r.solids(&beam, &[]).unwrap();
        assert_eq!(solids.iter().filter(|(k, _)| *k == BarKind::Top).count(), 2);
        assert_eq!(solids.iter().filter(|(k, _)| *k == BarKind::Bottom).count(), 3);
        assert_eq!(solids.iter().filter(|(k, _)| *k == BarKind::Stirrup).count(), 40);
        // all of it lies inside the concrete, the cover from every face
        let (lo, hi) = solids.iter().fold((glam::DVec3::splat(f64::INFINITY), glam::DVec3::splat(f64::NEG_INFINITY)), |(lo, hi), (_, s)| (lo.min(s.bounds().0), hi.max(s.bounds().1)));
        assert!((lo - glam::DVec3::new(1.03, 1.88, 2.78)).abs().max_element() < 1e-6 && (hi - glam::DVec3::new(6.97, 2.12, 3.22)).abs().max_element() < 1e-6, "{lo} {hi}");
        // a bottom bar: as long as the beam less its cover, 16 thick
        let (_, bar) = solids.iter().find(|(k, _)| *k == BarKind::Bottom).unwrap();
        let size = bar.bounds().1 - bar.bounds().0;
        assert!((size.x - 5.94).abs() < 1e-9 && (size.z - 0.016).abs() < 1e-3, "{size}");
        // a stirrup: a ring 8 thick round the section inside the cover
        let (_, stirrup) = solids.iter().find(|(k, _)| *k == BarKind::Stirrup).unwrap();
        let size = stirrup.bounds().1 - stirrup.bounds().0;
        assert!((size - glam::DVec3::new(0.008, 0.24, 0.44)).abs().max_element() < 1e-9, "{size}");
    }

    #[test]
    fn bars_that_do_not_fit_and_other_sections_are_refused() {
        let section = Profile::rectangle(0.2, 0.4).unwrap();
        let fits = Reinforcement { bottom: Bars { count: 3, diameter: 0.02 }, ..Default::default() };
        assert!(fits.bars(&section, 4.0).is_ok(), "3 Ø20 need 100 of the 124 inside the stirrups");
        let crowded = Reinforcement { bottom: Bars { count: 4, diameter: 0.02 }, ..Default::default() };
        assert_eq!(crowded.bars(&section, 4.0), Err(Refusal::BarsDoNotFit));
        assert_eq!(Reinforcement { cover: 0.2, ..Default::default() }.bars(&section, 4.0), Err(Refusal::BarsDoNotFit));
        assert_eq!(Reinforcement { spacing: 0.0, ..Default::default() }.bars(&section, 4.0), Err(Refusal::NotPositive));
        assert_eq!(Reinforcement::default().bars(&Profile::circle(0.4).unwrap(), 4.0), Err(Refusal::NotRectangular));
        // no top bars: only the bottom ones and the stirrups
        let rows = Reinforcement { top: Bars { count: 0, diameter: 0.0 }, ..Default::default() }.bars(&section, 4.0).unwrap();
        assert_eq!(rows.iter().map(|r| r.kind).collect::<Vec<_>>(), [BarKind::Bottom, BarKind::Stirrup]);
    }

    #[test]
    fn a_floor_has_a_net_below_and_one_on_top_inside_its_cover() {
        use glam::DVec2;
        // 6 × 4, 250 thick
        let outline = [DVec2::new(0.0, 0.0), DVec2::new(6.0, 0.0), DVec2::new(6.0, 4.0), DVec2::new(0.0, 4.0)];
        let nets = Nets::default();
        let bars = nets.bars(&outline, &[], 0.25).unwrap();
        let of = |top: bool, cross: bool| bars.iter().filter(|b| b.top == top && b.cross == cross).collect::<Vec<_>>();
        // below, along the 6 m: 3.93 of room takes 27 bars of 5.94; across: 5.93 takes 40 of 3.94
        let (main, cross) = (of(false, false), of(false, true));
        assert_eq!((main.len(), cross.len()), (27, 40));
        assert!((main[0].length() - 5.94).abs() < 1e-9 && (cross[0].length() - 3.94).abs() < 1e-9);
        // the main bars lie lowest, the cross bars on them; on top the other way round
        assert!((main[0].depth - 0.215).abs() < 1e-9 && (cross[0].depth - 0.205).abs() < 1e-9, "{} {}", main[0].depth, cross[0].depth);
        assert!((of(true, false)[0].depth - 0.034).abs() < 1e-9 && (of(true, true)[0].depth - 0.042).abs() < 1e-9);
        // the rows stand in the middle: as far from one edge as from the other
        assert!((main[0].a.y - (4.0 - main[26].a.y)).abs() < 1e-9 && main[0].a.y >= 0.035 - 1e-9);
        // the schedule: four rows, one per net and way
        let rows = net_rows(&bars);
        assert_eq!(rows.iter().map(|r| (r.kind, r.count)).collect::<Vec<_>>(), [(BarKind::NetBottom, 27), (BarKind::NetBottom, 40), (BarKind::NetTop, 27), (BarKind::NetTop, 40)]);
        // no top net, and what does not fit
        assert!(Nets { top: None, ..nets }.bars(&outline, &[], 0.25).unwrap().iter().all(|b| !b.top));
        assert_eq!(nets.bars(&outline, &[], 0.08), Err(Refusal::BarsDoNotFit));
        assert_eq!(Nets { cover: 0.0, ..nets }.bars(&outline, &[], 0.25), Err(Refusal::NotPositive));
    }

    #[test]
    fn the_bars_of_a_net_stop_at_an_opening_and_follow_an_l_shape() {
        use glam::DVec2;
        let p = DVec2::new;
        let nets = Nets { top: None, ..Default::default() };
        // a hole of 1 × 1 in the middle of 6 × 4: the bars through it are two short ones
        let outline = [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)];
        let hole = vec![p(2.5, 1.5), p(3.5, 1.5), p(3.5, 2.5), p(2.5, 2.5)];
        let bars = nets.bars(&outline, &[hole], 0.25).unwrap();
        let through: Vec<_> = bars.iter().filter(|b| !b.cross && (b.a.y - 2.0).abs() < 0.2).collect();
        assert!(!through.is_empty() && through.iter().all(|b| (b.length() - 2.44).abs() < 1e-9), "{through:?}");
        assert!(bars.iter().all(|b| !(b.a.x.min(b.b.x) < 3.0 && b.a.x.max(b.b.x) > 3.0 && (b.a.y - 2.0).abs() < 0.5 && !b.cross)));
        // an L: the bars along the long side are long in the wide part and short in the narrow one
        let l = [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 2.0), p(3.0, 2.0), p(3.0, 4.0), p(0.0, 4.0)];
        let bars = nets.bars(&l, &[], 0.25).unwrap();
        let lengths: std::collections::BTreeSet<i64> = bars.iter().filter(|b| !b.cross).map(|b| (b.length() * 1000.0).round() as i64).collect();
        assert_eq!(lengths.into_iter().collect::<Vec<_>>(), [2940, 5940]);
    }

    #[test]
    fn the_nets_of_a_slab_and_of_a_wall_are_rods_inside_them() {
        use crate::element::{Opening, PlanarDef, PlanarForm};
        use glam::{DVec2, DVec3};
        let bounds = |solids: &[(BarKind, crate::Solid)]| solids.iter().fold((DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)), |(lo, hi), (_, s)| (lo.min(s.bounds().0), hi.max(s.bounds().1)));
        let outline = vec![DVec2::new(1.0, 1.0), DVec2::new(7.0, 1.0), DVec2::new(7.0, 5.0), DVec2::new(1.0, 5.0)];
        let slab = PlanarDef { form: PlanarForm::Slab { outline, holes: Vec::new(), top: 3.0, bulges: Vec::new(), role: Default::default(), slope: None }, thickness: 0.25, openings: Vec::new(), build_up: None };
        let solids = Nets::default().solids(&slab, &[], &[]).unwrap();
        assert_eq!(solids.len(), 2 * (27 + 40));
        let (lo, hi) = bounds(&solids);
        assert!((lo - DVec3::new(1.03, 1.03, 2.78)).abs().max_element() < 1e-3 && (hi - DVec3::new(6.97, 4.97, 2.97)).abs().max_element() < 1e-3, "{lo} {hi}");
        // a wall of 5 × 3, 200 thick, along y, with a door: the rods stay in it and out of the door
        let door = Opening { along: 2.0, sill: 0.0, width: 1.0, height: 2.1, framing: None };
        let wall = PlanarDef { form: PlanarForm::Wall { start: DVec2::new(2.0, 0.0), end: DVec2::new(2.0, 5.0), base: 0.0, height: 3.0 }, thickness: 0.2, openings: Vec::new(), build_up: None };
        let solids = Nets::default().solids(&wall, &[door], &[]).unwrap();
        let (lo, hi) = bounds(&solids);
        assert!((lo - DVec3::new(1.93, 0.03, 0.03)).abs().max_element() < 1e-3 && (hi - DVec3::new(2.07, 4.97, 2.97)).abs().max_element() < 1e-3, "{lo} {hi}");
        assert!(solids.iter().all(|(_, s)| { let (lo, hi) = s.bounds(); !(lo.y < 2.5 && hi.y > 2.5 && lo.z < 1.0) }), "no rod through the door");
    }

    #[test]
    fn stirrups_and_bars_drawn_over_a_stretch_of_a_beam_come_besides_its_usual_ones() {
        use crate::element::LinearDef;
        use crate::profile::ProfileDef;
        let section = Profile::rectangle(0.3, 0.5).unwrap();
        let r = Reinforcement::default();
        // stirrups Ø10 every 100 over the first 1.5 m, and 2 Ø20 below from 2 to 4 m
        let extras = [Extra::Stirrups { from: 0.0, to: 1.5, diameter: 0.01, spacing: 0.1 }, Extra::Bars { from: 2.0, to: 4.0, count: 2, diameter: 0.02, top: false }];
        let stirrups = r.stirrups_in(6.0, &extras);
        // from the cover to 1.5: 1.47 takes 15 at 100; the usual ones stand only past 1.5
        assert_eq!(stirrups.iter().filter(|s| s.2 == Some(0)).count(), 15);
        assert!(stirrups.iter().filter(|s| s.2.is_none()).all(|s| s.0 > 1.5) && stirrups.iter().filter(|s| s.2.is_none()).count() == 30);
        let rows = r.rows(&section, 6.0, &extras).unwrap();
        assert_eq!(rows.iter().map(|r| (r.kind, r.count, (r.diameter * 1000.0).round() as i64)).collect::<Vec<_>>(), [(BarKind::Top, 2, 12), (BarKind::Bottom, 3, 16), (BarKind::Stirrup, 30, 8), (BarKind::Stirrup, 15, 10), (BarKind::Extra, 2, 20)]);
        assert!((rows[4].length - 2.0).abs() < 1e-12 && matches!(rows[3].shape, BarShape::Stirrup(w, h) if (w - 0.23).abs() < 1e-9 && (h - 0.43).abs() < 1e-9));
        // the bars lie a row inside the bottom ones: 30 + 8 + 16 + 20 + 10 above the underside
        let bars = r.extra_bars(&section, 6.0, &extras);
        assert_eq!(bars.len(), 2);
        assert!((bars[0].1.y + 0.25 - 0.084).abs() < 1e-9 && (bars[0].3, bars[0].4) == (2.0, 4.0), "{bars:?}");
        // as solids: 5 usual bars, 45 stirrups and the 2 drawn
        let beam = LinearDef { start: glam::DVec3::ZERO, end: glam::DVec3::new(6.0, 0.0, 0.0), profile: ProfileDef::Rectangle { width: 0.3, height: 0.5 }, roll: 0.0 };
        let solids = r.solids(&beam, &extras).unwrap();
        assert_eq!((solids.iter().filter(|s| s.0 == BarKind::Stirrup).count(), solids.iter().filter(|s| s.0 == BarKind::Extra).count()), (45, 2));
        // with no extras the rows are those of `bars`
        assert_eq!(r.rows(&section, 6.0, &[]).unwrap(), r.bars(&section, 6.0).unwrap());
        // too many bars beside each other, and a stretch outside the beam
        assert_eq!(r.rows(&section, 6.0, &[Extra::Bars { from: 2.0, to: 4.0, count: 9, diameter: 0.02, top: false }]), Err(Refusal::BarsDoNotFit));
        assert_eq!(r.rows(&section, 6.0, &[Extra::Bars { from: 7.0, to: 8.0, count: 2, diameter: 0.02, top: false }]), Err(Refusal::NotPositive));
    }

    #[test]
    fn a_line_drawn_in_a_slab_carries_hairpins_or_straight_bars_along_it() {
        use crate::element::{PlanarDef, PlanarForm};
        use glam::DVec2;
        let p = DVec2::new;
        // a slab of 6 × 4 from (1, 1), 250 thick
        let slab = PlanarDef { form: PlanarForm::Slab { outline: vec![p(1.0, 1.0), p(7.0, 1.0), p(7.0, 5.0), p(1.0, 5.0)], holes: Vec::new(), top: 3.0, bulges: Vec::new(), role: Default::default(), slope: None }, thickness: 0.25, openings: Vec::new(), build_up: None };
        let nets = Nets::default();
        // hairpins Ø8 every 150 along its first edge, legs of 600
        let hairpins = Extra::Line { a: p(0.0, 0.0), b: p(6.0, 0.0), diameter: 0.008, spacing: 0.15, length: 0.6, form: LineForm::Hairpin };
        let bars = nets.laid(&slab, &[], &[hairpins]).unwrap();
        let own: Vec<&NetBar> = bars.iter().filter(|b| b.extra == Some(0)).collect();
        assert_eq!(own.len(), 40);
        // each from 34 inside the edge 600 into the slab, a leg inside the top net and one inside the bottom one
        assert!(own.iter().all(|b| (b.a.y - 1.034).abs() < 1e-9 && (b.b.y - 1.634).abs() < 1e-9 && (b.depth - 0.05).abs() < 1e-9 && (b.drop - 0.146).abs() < 1e-9), "{:?}", own[0]);
        let rows = net_rows(&bars);
        assert_eq!(rows.len(), 5);
        assert!(rows[4].kind == BarKind::Extra && rows[4].count == 40 && matches!(rows[4].shape, BarShape::Hairpin(leg, apart) if (leg - 0.6).abs() < 1e-9 && (apart - 0.146).abs() < 1e-9));
        assert!((rows[4].length - 1.346).abs() < 1e-9);
        // three rods a hairpin
        assert_eq!(nets.solids(&slab, &[], &[hairpins]).unwrap().iter().filter(|s| s.0 == BarKind::Extra).count(), 120);
        // straight bars Ø12 of 2 m on top over a line in the middle: half to each side
        let over = Extra::Line { a: p(3.0, 0.5), b: p(3.0, 3.5), diameter: 0.012, spacing: 0.15, length: 2.0, form: LineForm::Top };
        let bars = nets.laid(&slab, &[], &[over]).unwrap();
        let own: Vec<&NetBar> = bars.iter().filter(|b| b.extra == Some(0)).collect();
        assert_eq!(own.len(), 20);
        assert!(own.iter().all(|b| (b.a.x.min(b.b.x) - 3.0).abs() < 1e-9 && (b.a.x.max(b.b.x) - 5.0).abs() < 1e-9 && b.drop == 0.0 && b.top));
        // a line outside the slab makes nothing, and is refused
        let outside = Extra::Line { a: p(0.0, -3.0), b: p(6.0, -3.0), diameter: 0.008, spacing: 0.15, length: 0.6, form: LineForm::Bottom };
        assert_eq!(nets.laid(&slab, &[], &[outside]), Err(Refusal::BarsDoNotFit));
    }
}
