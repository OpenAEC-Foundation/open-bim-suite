//! A 2D drawing to export: lines, texts and dimensions, each with a pen, built once from a view
//! or a sheet (as the sheet draws it) and written by every export format: DWG, DXF, IFCCAD, PDF,
//! PNG and SVG. Metres: in the model for a view (drawn at 1 : scale on paper), on the paper for a
//! sheet. Headless, like the document.

use crate::document::{Dimension, Document, Note, SHEET_SCALE};
use crate::drawing::Cut;
use bim_library::drawing::{Drawn, Line};
use std::sync::Arc;

/// Text on paper: 2.5 mm capitals (metres).
const TEXT: f64 = 0.0025;
/// Space between a placed view's lines and its frame (paper metres), as on a sheet.
const FRAME: f64 = 0.005;
/// Space around a view's drawing on its page (paper metres).
const MARGIN: f64 = 0.010;
/// How long the arrows of a section's mark are (paper metres), as on a sheet.
const ARROW: f64 = 0.005;
/// How wide a text is per character, for its capitals' height.
const CHARACTER: f64 = 0.8;
/// The radius of a grid's bubble on paper (metres), and how many lines draw it round.
const BUBBLE: f64 = 0.004;
const BUBBLE_SIDES: usize = 24;

/// What a line or text is, and so how it is drawn and which CAD layer it goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pen {
    /// Where the plan's or section's plane cuts the model: bold.
    Cut,
    /// Seen beyond the cut: thin.
    Beyond,
    /// The hatch of a cut face: thin.
    Hatch,
    Dimension,
    Text,
    /// The sheet's border, the marks of sections.
    Frame,
    /// Grid lines and levels, with their bubbles and names.
    Grids,
    /// What a view draws itself beside the model: detail lines, detail components, the shapes
    /// and leaders of its tags.
    Detail,
}

impl Pen {
    pub const ALL: [Pen; 8] = [Pen::Cut, Pen::Beyond, Pen::Hatch, Pen::Dimension, Pen::Text, Pen::Frame, Pen::Grids, Pen::Detail];

    /// The CAD layer name.
    pub fn name(self) -> &'static str {
        match self {
            Pen::Cut => "O3V-Cut",
            Pen::Beyond => "O3V-Beyond",
            Pen::Hatch => "O3V-Hatch",
            Pen::Dimension => "O3V-Dimensions",
            Pen::Text => "O3V-Text",
            Pen::Frame => "O3V-Frame",
            Pen::Grids => "O3V-Grids",
            Pen::Detail => "O3V-Detail",
        }
    }

    /// The line weight on paper (millimetres).
    pub fn weight_mm(self) -> f64 {
        match self {
            Pen::Cut => 0.50,
            Pen::Beyond | Pen::Dimension | Pen::Grids => 0.18,
            Pen::Hatch => 0.13,
            Pen::Text | Pen::Detail => 0.25,
            Pen::Frame => 0.35,
        }
    }
}

/// How a view is drawn on paper, as its kind of drawing in the project's drawing standard says
/// ([`Rules::of`]). Without a kind it is drawn as drawings were before there was a standard.
#[derive(Debug, Clone, PartialEq)]
pub struct Rules {
    /// What is cut: its width and dashes (millimetres on paper) and its colour.
    pub cut: Drawn,
    /// What is seen beyond the cut.
    pub beyond: Drawn,
    /// What is hidden.
    pub hidden: Drawn,
    /// Grid lines and levels.
    pub grid: Drawn,
    /// The line of a section on a floor plan.
    pub section: Drawn,
    /// The height of its texts and dimensions on paper (metres).
    pub text: f64,
    /// Whether what is cut is hatched by its material.
    pub hatched: bool,
}

impl Default for Rules {
    fn default() -> Self {
        let line = |width: f64, dashes: &[f64]| Drawn { width, dashes: dashes.to_vec(), colour: None };
        Rules {
            cut: line(Pen::Cut.weight_mm(), &[]),
            beyond: line(Pen::Beyond.weight_mm(), &[]),
            hidden: line(Pen::Beyond.weight_mm(), &[2.0, 1.0]),
            grid: line(Pen::Grids.weight_mm(), &[8.0, 1.5, 0.35, 1.5]),
            // a chain line, as a sheet's tab always drew it
            section: line(Pen::Frame.weight_mm(), &[8.0, 1.5, 0.35, 1.5]),
            text: TEXT,
            hatched: true,
        }
    }
}

impl Rules {
    /// The rules of kind of drawing `kind` of `standard` at `scale` (1 to this): each line in
    /// the line style the kind gives it, at the width its pen has at that scale, or by the
    /// kind's pens ([`bim_library::drawing::Standard::stroke`]); its text style's height and
    /// whether it hatches. A line the kind says nothing of, or a pen or a style the standard
    /// no longer has, leaves that rule as it is by default.
    pub fn of(standard: &bim_library::drawing::Standard, kind: &bim_library::drawing::DrawingKind, scale: f64) -> Self {
        let plain = Rules::default();
        let scale = if scale.is_finite() { scale.round().clamp(1.0, f64::from(u32::MAX)) as u32 } else { 1 };
        let line = |line: Line, plain: Drawn| standard.stroke(kind, line, scale).unwrap_or(plain);
        Rules {
            cut: line(Line::Cut, plain.cut),
            beyond: line(Line::Beyond, plain.beyond),
            hidden: line(Line::Hidden, plain.hidden),
            grid: line(Line::Grid, plain.grid),
            section: line(Line::Section, plain.section),
            text: standard.text_style(&kind.text).map_or(plain.text, |style| style.height / 1000.0),
            hatched: kind.hatched,
        }
    }
}

/// What a drawing's metres are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Space {
    /// The model's metres, drawn at 1 : `scale` on paper.
    Model { scale: f64 },
    /// The paper's metres.
    Paper,
}

/// A line from `a` to `b`.
#[derive(Debug, Clone, PartialEq)]
pub struct Line2d {
    pub pen: Pen,
    pub a: [f64; 2],
    pub b: [f64; 2],
    /// Its width on paper (millimetres): its pen's, unless its view's kind of drawing gives
    /// what is cut or what is seen beyond a width of its own.
    pub width: f64,
    pub color: Option<[u8; 3]>,
}

impl Line2d {
    /// A line in `pen`, as wide as that pen draws.
    pub fn new(pen: Pen, a: [f64; 2], b: [f64; 2]) -> Self {
        Line2d { pen, a, b, width: pen.weight_mm(), color: None }
    }
}

/// Which point of a text [`Text2d::at`] is: its lower left, the middle of its bottom or its lower
/// right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// A text standing on `at`, turned `angle` radians counter-clockwise about that point; `height`
/// is that of its capitals.
#[derive(Debug, Clone, PartialEq)]
pub struct Text2d {
    /// A specified font family uses nominal em height, in drawing units.
    pub family: Option<Arc<str>>,
    pub bold: bool,
    pub color: Option<[u8; 3]>,
    pub pen: Pen,
    pub at: [f64; 2],
    pub height: f64,
    pub text: String,
    pub align: Align,
    pub angle: f64,
}

impl Text2d {
    /// About how wide it is.
    pub fn width(&self) -> f64 {
        self.text.chars().count() as f64 * self.height * CHARACTER
    }

    /// About where its left and right ends are.
    fn span(&self) -> (f64, f64) {
        let w = self.width();
        match self.align {
            Align::Left => (self.at[0], self.at[0] + w),
            Align::Center => (self.at[0] - w / 2.0, self.at[0] + w / 2.0),
            Align::Right => (self.at[0] - w, self.at[0]),
        }
    }
}

/// A dimension between two points: its line `offset` to the left of `from` → `to`, with
/// extension lines and a slash at each end, and `value` (metres, what it measures in the model)
/// in whole millimetres beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Dim2d {
    pub first_witness: bool,
    /// The actual reference points from which the witness lines start.
    pub witnesses: [[f64; 2]; 2],
    pub units_per_paper_metre: f64,
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub value: f64,
    pub offset: f64,
    pub text_height: f64,
    /// End marks and witness line spacing in paper metres.
    pub style: crate::document::DimensionStyle,
}

impl Dim2d {
    /// Along `from` → `to` and to its left (units); `None` when the points coincide.
    pub fn axes(&self) -> Option<([f64; 2], [f64; 2])> {
        let (dx, dy) = (self.to[0] - self.from[0], self.to[1] - self.from[1]);
        let run = dx.hypot(dy);
        (run > 0.0).then(|| ([dx / run, dy / run], [-dy / run, dx / run]))
    }

    /// Where the dimension line runs: from, to.
    pub fn line(&self) -> Option<([f64; 2], [f64; 2])> {
        let (_, n) = self.axes()?;
        Some((add(self.from, n, self.offset), add(self.to, n, self.offset)))
    }

    /// Its extension lines (from just off the points to just past its line), its dimension line
    /// and its two slashes, as a sheet draws them.
    pub fn lines(&self) -> Vec<[[f64; 2]; 2]> {
        let (Some((u, n)), Some((a, b))) = (self.axes(), self.line()) else { return Vec::new() };
        let paper = self.units_per_paper_metre;
        let tick = self.style.tick_size * paper;
        let gap = self.style.gap * paper;
        let extension = self.style.extension * paper;
        let mut lines = Vec::new();
        for (q, mark) in self.witnesses.into_iter().zip([a,b]).skip(usize::from(!self.first_witness)) {
            let distance = (mark[0] - q[0]) * n[0] + (mark[1] - q[1]) * n[1];
            let side = if distance < 0.0 { -1.0 } else { 1.0 };
            lines.push([add(q, n, side * gap.min(distance.abs())), add(mark, n, side * extension)]);
        }
        lines.push([a,b]);
        use crate::document::Tick;
        for (q, inward) in [(a, 1.0), (b, -1.0)].into_iter().skip(usize::from(!self.first_witness)) {
            match self.style.tick {
                Tick::Slash => {
                    let diagonal = [u[0] + n[0], u[1] + n[1]];
                    lines.push([add(q, diagonal, -tick), add(q, diagonal, tick)]);
                }
                Tick::Dot => {} // Filled round caps are emitted by strokes at the tick diameter.
                Tick::Arrow => {
                    let back = add(q, u, inward * tick);
                    let (l, r) = (add(back, n, tick * 0.27), add(back, n, -tick * 0.27));
                    lines.extend([[q, l], [l, r], [r, q]]);
                }
                Tick::None => {}
            }
        }
        lines
    }

    /// Its text: the value in whole millimetres, level and centred beside the line, clear of it
    /// whichever way the line runs.
    pub fn label(&self) -> Text2d {
        let text = self.style.figure(self.value);
        let n = self.axes().map_or([0.0, 1.0], |(_, n)| n);
        let h = self.text_height;
        let w = text.chars().count() as f64 * h * CHARACTER;
        let clear = n[0].abs() * w / 2.0 + n[1].abs() * h / 2.0 + self.units_per_paper_metre * 0.0015;
        let middle = [(self.from[0] + self.to[0]) / 2.0, (self.from[1] + self.to[1]) / 2.0];
        let angle = self.style.label_angle(self.to[0]-self.from[0], self.to[1]-self.from[1]);
        let at = if self.style.aligned_text {
            let (sin, cos) = angle.sin_cos();
            add(add(middle, n, self.offset), [-sin,cos], crate::document::DimensionStyle::ALIGNED_LABEL_GAP * self.units_per_paper_metre)
        } else {
            let centre = add(middle, n, self.offset + clear);
            [centre[0], centre[1]-h/2.]
        };
        Text2d { family: self.style.label_family(), bold: false, color: None, pen: Pen::Dimension, at, height: h, text, align: Align::Center, angle }
    }
}

/// A grid line or a level on a view's drawing: its name, its ends (the drawing's metres) and, for a
/// level, its height.
#[derive(Debug, Clone, PartialEq)]
pub struct Datum2 {
    pub name: String,
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub level: Option<f64>,
}

/// A solid area in drawing coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon2d {
    pub points: Vec<[f64; 2]>,
    pub color: [u8; 3],
}

/// A drawing to export.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing2d {
    pub polygons: Vec<Polygon2d>,
    pub images: Vec<crate::raster::PlacedImage>,
    /// The view's or sheet's name, for the file name.
    pub name: String,
    pub space: Space,
    /// A sheet's paper (width, height, metres).
    pub paper: Option<[f64; 2]>,
    pub lines: Vec<Line2d>,
    pub texts: Vec<Text2d>,
    pub dimensions: Vec<Dim2d>,
}

impl Drawing2d {
    /// Draws grid lines and levels as the view's tab shows them: a grid with a bubble and its name
    /// past both ends, a level with its name and height past both ends.
    /// The lines as `grid` draws them: its width, its dashes on paper at every scale, its colour.
    pub fn datums(&mut self, datums: &[Datum2], grid: &Drawn) {
        let unit = 1.0 / self.paper_per_unit();
        let (r, h) = (BUBBLE * unit, TEXT * unit);
        for d in datums {
            let (dx, dy) = (d.b[0] - d.a[0], d.b[1] - d.a[1]);
            let run = dx.hypot(dy);
            if run <= 0.0 {
                continue;
            }
            let u = [dx / run, dy / run];
            self.dashed(Pen::Grids, d.a, d.b, grid, unit / 1000.0, &mut 0.0);
            match d.level {
                None => {
                    for (end, out) in [(d.a, [-u[0], -u[1]]), (d.b, u)] {
                        let c = add(end, out, r);
                        let point = |k: usize| {
                            let t = std::f64::consts::TAU * k as f64 / BUBBLE_SIDES as f64;
                            [c[0] + r * t.cos(), c[1] + r * t.sin()]
                        };
                        for k in 0..BUBBLE_SIDES {
                            self.line_as(Pen::Grids, point(k), point(k + 1), grid);
                        }
                        self.texts.push(Text2d { family: None, bold: false, color: None, pen: Pen::Grids, at: [c[0], c[1] - h / 2.0], height: h, text: d.name.clone(), align: Align::Center, angle: 0.0 });
                    }
                }
                Some(z) => {
                    let label = format!("{} {z:+.3}", d.name);
                    let gap = h * 0.6;
                    let (right, left) = (add(d.b, u, gap), add(d.a, [-u[0], -u[1]], gap));
                    self.texts.push(Text2d { family: None, bold: false, color: None, pen: Pen::Grids, at: [right[0], right[1] - h / 2.0], height: h, text: label.clone(), align: Align::Left, angle: 0.0 });
                    self.texts.push(Text2d { family: None, bold: false, color: None, pen: Pen::Grids, at: [left[0], left[1] - h / 2.0], height: h, text: label, align: Align::Right, angle: 0.0 });
                }
            }
        }
    }

    /// Paper metres per metre of the drawing.
    pub fn paper_per_unit(&self) -> f64 {
        match self.space {
            Space::Model { scale } => 1.0 / scale,
            Space::Paper => 1.0,
        }
    }

    /// Lower left and upper right of what is drawn; `None` for nothing.
    pub fn extent(&self) -> Option<([f64; 2], [f64; 2])> {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        let mut grow = |p: [f64; 2]| {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        };
        for polygon in &self.polygons { polygon.points.iter().copied().for_each(&mut grow); }
        for image in &self.images { image.clip.into_iter().for_each(&mut grow); }
        for l in &self.lines {
            grow(l.a);
            grow(l.b);
        }
        let labels: Vec<Text2d> = self.dimensions.iter().map(Dim2d::label).collect();
        for t in self.texts.iter().chain(&labels) {
            let (left, right) = t.span();
            grow([left, t.at[1]]);
            grow([right, t.at[1] + t.height]);
        }
        for [a, b] in self.dimensions.iter().flat_map(Dim2d::lines) {
            grow(a);
            grow(b);
        }
        lo[0].is_finite().then_some((lo, hi))
    }

    /// The page it is printed on, in its own metres: a sheet's paper, else what is drawn with a
    /// margin of 10 mm on paper (a page of 10 × 10 cm on paper when nothing is).
    pub fn page(&self) -> ([f64; 2], [f64; 2]) {
        if let Some(paper) = self.paper {
            return ([0.0, 0.0], paper);
        }
        let unit = 1.0 / self.paper_per_unit();
        match self.extent() {
            Some((lo, hi)) => {
                let m = MARGIN * unit;
                ([lo[0] - m, lo[1] - m], [hi[0] + m, hi[1] + m])
            }
            None => {
                let half = 0.05 * unit;
                ([-half, -half], [half, half])
            }
        }
    }

    fn line(&mut self, pen: Pen, a: [f64; 2], b: [f64; 2]) {
        self.lines.push(Line2d::new(pen, a, b));
    }

    /// A line in `pen`, as wide as `stroke` and in its colour, whole whatever its dashes.
    fn line_as(&mut self, pen: Pen, a: [f64; 2], b: [f64; 2], stroke: &Drawn) {
        self.lines.push(Line2d { pen, a, b, width: stroke.width, color: stroke.colour });
    }

    /// The line from `a` to `b` in `pen` as `stroke` draws it: whole, or in its dashes and gaps,
    /// `per_mm` of the drawing's metres to a millimetre on paper. `phase` is how far into the
    /// pattern the line begins (millimetres on paper) and is left where it ends, so the pieces
    /// of one run carry the pattern on round a corner.
    fn dashed(&mut self, pen: Pen, a: [f64; 2], b: [f64; 2], stroke: &Drawn, per_mm: f64, phase: &mut f64) {
        let pattern: Vec<f64> = stroke.dashes.iter().map(|mm| mm * per_mm).collect();
        let period: f64 = pattern.iter().sum();
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        if period <= 0.0 || !period.is_finite() || pattern.iter().any(|part| *part < 0.0) || length <= 0.0 {
            self.line_as(pen, a, b, stroke);
            return;
        }
        let at = |d: f64| [a[0] + (b[0] - a[0]) * d / length, a[1] + (b[1] - a[1]) * d / length];
        let (mut d, mut within) = (0.0, (*phase * per_mm).rem_euclid(period));
        while d < length {
            // the part of the pattern `within` is in (a dash when it is even), and how far that
            // part still runs; at its very end the next one begins
            let (mut part, mut from) = (0, 0.0);
            while part + 1 < pattern.len() && within >= from + pattern[part] - period * 1e-6 {
                from += pattern[part];
                part += 1;
            }
            let step = (from + pattern[part] - within).max(period * 1e-6);
            let end = (d + step).min(length);
            if part % 2 == 0 {
                self.line_as(pen, at(d), at(end), stroke);
            }
            within = (within + (end - d)) % period;
            d = end;
        }
        *phase = within / per_mm;
    }

    /// `segments` of a view's drawing (cut back to `crop`, through `to`) in `pen` as `stroke`
    /// draws them: each whole when the stroke is continuous, else run by run in its dashes, so
    /// they run on round a curve of many short pieces.
    fn stroked(&mut self, pen: Pen, segments: &[crate::drawing::Segment], crop: Option<([f64; 2], [f64; 2])>, to: &dyn Fn([f64; 2]) -> [f64; 2], stroke: &Drawn, per_mm: f64) {
        let piece = |a: [f32; 2], b: [f32; 2]| {
            let (a, b) = (a.map(f64::from), b.map(f64::from));
            match crop {
                Some((lo, hi)) => clip(a, b, lo, hi),
                None => Some((a, b)),
            }
        };
        if stroke.dashes.is_empty() {
            for [a, b] in segments {
                if let Some((a, b)) = piece(*a, *b) {
                    self.line_as(pen, to(a), to(b), stroke);
                }
            }
            return;
        }
        for run in crate::drawing::chained(segments) {
            let mut phase = 0.0f64;
            for pair in run.windows(2) {
                if let Some((a, b)) = piece(pair[0], pair[1]) {
                    self.dashed(pen, to(a), to(b), stroke, per_mm, &mut phase);
                }
            }
        }
    }

    /// The widths its lines of `pen` have (millimetres on paper), the narrowest first: a
    /// writer draws the lines of a pen a width at a time.
    pub fn widths(&self, pen: Pen) -> Vec<f64> {
        let mut widths: Vec<f64> = self.lines.iter().filter(|l| l.pen == pen).map(|l| l.width).collect();
        widths.sort_by(f64::total_cmp);
        widths.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        widths
    }

    fn rectangle(&mut self, pen: Pen, lo: [f64; 2], hi: [f64; 2]) {
        let corners = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
        for k in 0..4 {
            self.line(pen, corners[k], corners[(k + 1) % 4]);
        }
    }

    fn text(&mut self, at: [f64; 2], height: f64, text: impl Into<String>, align: Align) {
        self.turned_text(at, height, text, align, 0.0);
    }

    fn turned_text(&mut self, at: [f64; 2], height: f64, text: impl Into<String>, align: Align, angle: f64) {
        self.texts.push(Text2d { family: None, bold: false, color: None, pen: Pen::Text, at, height, text: text.into(), align, angle });
    }

    fn view_details(&mut self, doc: &Document, view: &str, to: &dyn Fn([f64; 2]) -> [f64; 2]) {
        for mut polygon in view_polygons(doc, view) {
            polygon.points.iter_mut().for_each(|q| *q = to(*q));
            self.polygons.push(polygon);
        }
        let crop = doc.view_marks(view).and_then(|(_, crop)| crop);
        let pens = doc.detail_pens(view);
        let lines = doc.view_lines(view).iter().chain(doc.view_symbols(view));
        for (index, &[a, b]) in lines.enumerate() {
            let Some((a, b)) = crop_segment(a, b, crop) else { continue };
            let mut line = Line2d::new(Pen::Detail, to(a), to(b));
            if let Some((_, rgb, width)) = pens.iter().find(|(range, _, _)| range.contains(&index)) {
                line.color = Some(*rgb);
                line.width = *width;
            }
            self.lines.push(line);
        }
    }

    /// Coloured strokes for a writer that emits one path per width and colour.
    pub fn strokes(&self, pen: Pen) -> Vec<(f64, [u8; 3], Vec<[[f64; 2]; 2]>)> {
        let mut groups = Vec::new();
        for width in self.widths(pen) {
            let mut colors = std::collections::BTreeMap::<_, Vec<_>>::new();
            for l in self.lines.iter().filter(|l| l.pen == pen && (l.width - width).abs() < 1e-9) {
                colors.entry(l.color.unwrap_or_default()).or_default().push([l.a, l.b]);
            }
            groups.extend(colors.into_iter().map(|(color, lines)| (width, color, lines)));
        }
        if pen == Pen::Dimension && !self.dimensions.is_empty() {
            groups.push((pen.weight_mm(), [0; 3], self.dimensions.iter().flat_map(Dim2d::lines).collect()));
            for dim in self.dimensions.iter().filter(|d| d.style.tick == crate::document::Tick::Dot) {
                if let Some((a, b)) = dim.line() {
                    let dots = if dim.first_witness { vec![[a,a],[b,b]] } else { vec![[b,b]] };
                    groups.push((dim.style.tick_size * 1000.0, [0; 3], dots));
                }
            }
        }
        groups
    }

    /// A view's lines (cut back to its crop), dimensions and texts, through `to` (a point of the
    /// view's drawing to this drawing), `unit` drawing metres per paper metre, `per_metre` metres
    /// of the view's drawing per paper metre (its scale), drawn as `rules` say.
    /// `to` keeps lengths but for that scale (it moves, turns and scales alike both ways).
    #[allow(clippy::too_many_arguments)]
    fn view(&mut self, cut: Option<&Cut>, crop: Option<([f64; 2], [f64; 2])>, marks: (&[Dimension], &[Note]), doc: &Document, to: &dyn Fn([f64; 2]) -> [f64; 2], unit: f64, per_metre: f64, rules: &Rules) {
        if let Some(cut) = cut {
            // the hatch lines of the cut faces, at least 0.7 mm apart on paper, where the view's
            // kind of drawing hatches what is cut
            for fill in cut.fills.iter().filter(|_| rules.hatched) {
                // the colour behind its hatch, where its material has one: a sign that is a
                // fill alone (reinforced concrete cast in situ, steel) is drawn by it
                if let Some((behind, _)) = fill.look.filter(|(behind, _)| *behind != [255; 3]) {
                    let loops: Vec<Vec<[f64; 2]>> = fill.loops.iter().map(|l| l.iter().map(|p| p.map(f64::from)).collect()).collect();
                    for points in filled(&loops, crop) {
                        self.polygons.push(Polygon2d { points: points.into_iter().map(to).collect(), color: behind });
                    }
                }
                for [a, b] in fill.lines(fill.spacing() / per_metre as f32, 0.0007) {
                    let (a, b) = (a.map(f64::from), b.map(f64::from));
                    let piece = match crop {
                        Some((lo, hi)) => clip(a, b, lo, hi),
                        None => Some((a, b)),
                    };
                    if let Some((a, b)) = piece {
                        self.line(Pen::Hatch, to(a), to(b));
                    }
                }
            }
            // every line as the view's kind of drawing draws it: what a structural plan sees
            // through the floor in its hidden line, what is seen beyond the cut, what is cut
            let per_mm = unit / 1000.0;
            for segments in [&cut.hidden, &cut.faint_hidden] {
                self.stroked(Pen::Beyond, segments, crop, to, &rules.hidden, per_mm);
            }
            let inked = cut.inked.iter().map(|i| (if i.cut { Pen::Cut } else { Pen::Beyond }, &i.lines));
            for (pen, segments) in [(Pen::Beyond, &cut.faint_below), (Pen::Beyond, &cut.faint_segments), (Pen::Beyond, &cut.below), (Pen::Cut, &cut.segments)].into_iter().chain(inked) {
                self.stroked(pen, segments, crop, to, if pen == Pen::Cut { &rules.cut } else { &rules.beyond }, per_mm);
            }
        }
        let (dimensions, notes) = marks;
        for d in dimensions {
            let style = doc.dimension_style(d.style);
            let height = if style.label_family().is_some() { style.paper_font_size() } else { rules.text };
            for (i, (actual, [a,b])) in d.intervals().enumerate() {
                self.dimensions.push(Dim2d { first_witness: i == 0, from: to(a), to: to(b), witnesses: actual.map(to), units_per_paper_metre: unit,
                    value: d.interval_value(a,b,doc.dimension_style(d.style)), offset: d.paper_offset() * unit,
                    style: style.clone(), text_height: height * unit });
            }
        }
        for n in notes {
            if crop_segment(n.at, n.at, crop).is_none() { continue; }
            let align = match n.align {
                crate::document::Align::Left => Align::Left,
                crate::document::Align::Center => Align::Center,
                crate::document::Align::Right => Align::Right,
            };
            let (mut at, h) = (to(n.at), rules.text * unit * 0.9);
            if n.level {
                // its triangle, point down on the height, a short line at that height, the text beside it
                let (l, r) = ([at[0] - h * 0.5, at[1] + h], [at[0] + h * 0.5, at[1] + h]);
                for (a, b) in [(at, l), (l, r), (r, at), ([at[0] - h, at[1]], [at[0] + h * 3.5, at[1]])] {
                    self.lines.push(Line2d::new(Pen::Text, a, b));
                }
                at = [at[0] + h * 0.8, at[1] + h * 0.1];
            }
            let family = Some(n.paper_face().family);
            let height = n.size;
            self.texts.push(Text2d { family, bold: n.face.bold, color: n.face.color, pen: Pen::Text, at, height: height * unit, text: n.text.clone(), align, angle: n.angle });
        }
    }
}

/// `p` moved `d` along `n`.
fn add(p: [f64; 2], n: [f64; 2], d: f64) -> [f64; 2] {
    [p[0] + n[0] * d, p[1] + n[1] * d]
}

/// Clip each filled triangle to the view bounds. Triangulating first preserves holes.
pub(crate) fn view_polygons(doc: &Document, path: &str) -> Vec<Polygon2d> {
    let crop = doc.view_marks(path).and_then(|(_, crop)| crop);
    doc.view_fills(path).flat_map(|fill| filled(&fill.loops, crop).into_iter().map(|points| Polygon2d { points, color: fill.color })).collect()
}

/// What the outlines `loops` fill (by the even-odd rule, so holes stay open) as triangles, each
/// cut back to `crop`.
fn filled(loops: &[Vec<[f64; 2]>], crop: Option<([f64; 2], [f64; 2])>) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    for triangle in crate::caps::triangulate(loops) {
        let mut points = triangle.to_vec();
        if let Some((lo, hi)) = crop {
            for (axis, edge, sign) in [(0, lo[0], 1.0), (1, lo[1], 1.0), (0, hi[0], -1.0), (1, hi[1], -1.0)] {
                let Some(&mut_last) = points.last() else { break };
                let mut prev = mut_last;
                let input = std::mem::take(&mut points);
                for p in input {
                    let inside = (p[axis] - edge) * sign >= 0.0;
                    if inside != ((prev[axis] - edge) * sign >= 0.0) {
                        let t = (edge - prev[axis]) / (p[axis] - prev[axis]);
                        points.push([prev[0] + t * (p[0] - prev[0]), prev[1] + t * (p[1] - prev[1])]);
                    }
                    if inside { points.push(p); }
                    prev = p;
                }
            }
        }
        if points.len() >= 3 { out.push(points); }
    }
    out
}

pub(crate) fn crop_segment(a: [f64; 2], b: [f64; 2], crop: Option<([f64; 2], [f64; 2])>) -> Option<([f64; 2], [f64; 2])> {
    match crop { Some((lo, hi)) => clip(a, b, lo, hi), None => Some((a, b)) }
}

/// The part of segment `a`–`b` inside the rectangle `lo`–`hi` (Liang–Barsky), as a sheet crops.
pub fn clip(a: [f64; 2], b: [f64; 2], lo: [f64; 2], hi: [f64; 2]) -> Option<([f64; 2], [f64; 2])> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-d[0], a[0] - lo[0]), (d[0], hi[0] - a[0]), (-d[1], a[1] - lo[1]), (d[1], hi[1] - a[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 <= t1).then(|| ([a[0] + t0 * d[0], a[1] + t0 * d[1]], [a[0] + t1 * d[0], a[1] + t1 * d[1]]))
}

/// Visible section references of a plan, clipped to its view window before placement.
pub(crate) fn plan_sections(doc: &Document, path: &str) -> Vec<(String, [f64; 2], [f64; 2])> {
    let Some(plan) = doc.plan(path) else { return Vec::new() };
    let marks = &plan.presentation["section_marks"];
    let sections = marks.as_array();
    let show = marks.as_bool().or_else(|| sections.map(|_| true))
        .unwrap_or_else(|| plan.drawing.as_deref() != Some("pile-plan") && !doc.view_is_schedule(path));
    if !show { return Vec::new(); }
    doc.sections().iter().filter_map(|s| {
        if sections.is_some_and(|paths| !paths.iter().any(|p| p.as_str() == Some(s.path.as_str()))) {
            return None;
        }
        let (a,b) = match plan.crop {
            Some((lo, hi)) => clip(s.from, s.to, lo, hi)?,
            None => (s.from, s.to),
        };
        (a != b).then(|| (s.mark(), a,b))
    }).collect()
}

/// The dimensions and texts of a view.
fn marks<'a>(doc: &'a Document, path: &str) -> (&'a [Dimension], Vec<Note>, Option<([f64; 2], [f64; 2])>) {
    let (dimensions, crop) = doc.view_marks(path).unwrap_or((&[], None));
    // its own texts, then what the model says on it by itself (the names of its spaces)
    (dimensions, doc.view_notes(path).unwrap_or(&[]).iter().chain(doc.view_tags(path)).cloned().collect(), crop)
}

/// The scale view `path` is drawn at by itself (1 to this): that of the first sheet it is placed
/// on, else its own.
pub fn drawn_scale(doc: &Document, path: &str) -> f64 {
    doc.sheets().iter().flat_map(|s| &s.views).find(|v| v.plan == path).map_or_else(|| doc.view_scale(path).unwrap_or(SHEET_SCALE), |v| v.scale)
}

/// The drawing of a floor plan, section or elevation (`path`) from its `cut`, with its dimensions
/// and texts, at the scale of the first sheet it is placed on (else its own), cut back to its
/// crop and drawn as `rules` say.
pub fn view_drawing(doc: &Document, path: &str, cut: &Cut, rules: &Rules) -> Option<Drawing2d> {
    let name = doc.view_name(path)?.to_string();
    let scale = drawn_scale(doc, path);
    let (dimensions, notes, crop) = marks(doc, path);
    let mut d = Drawing2d { polygons: Vec::new(), images: Vec::new(), name, space: Space::Model { scale }, paper: None, lines: Vec::new(), texts: Vec::new(), dimensions: Vec::new() };
    if let Some(photo) = doc.plan(path).and_then(|p| p.orthophoto.as_ref()) { d.images.extend(photo.image.placed(crop, |q| q)); }
    d.view(Some(cut), crop, (dimensions, &notes), doc, &|q| q, scale, scale, rules);
    d.view_details(doc, path, &|q| q);
    Some(d)
}

/// The drawing of a sheet as its tab draws it: the paper's edge, its border and title, and each
/// placed view's lines (`cut` gives a view's cut by its path), dimensions, texts, the marks of the
/// sections on the floor plans, its name and scale. `rules` gives how a view is drawn, by its
/// path and the scale it is placed at.
pub fn sheet_drawing(doc: &Document, path: &str, cut: &dyn Fn(&str) -> Option<Arc<Cut>>, datums: &dyn Fn(&str) -> Vec<Datum2>, rules: &dyn Fn(&str, f64) -> Rules) -> Option<Drawing2d> {
    sheet_drawing_with_style(doc, &bim_library::house::HouseStyle::starter(), path, cut, datums, rules)
}

pub fn sheet_drawing_with_style(doc: &Document, style: &bim_library::house::HouseStyle, path: &str, cut: &dyn Fn(&str) -> Option<Arc<Cut>>, datums: &dyn Fn(&str) -> Vec<Datum2>, rules: &dyn Fn(&str, f64) -> Rules) -> Option<Drawing2d> {
    let sheet = doc.sheet(path)?;
    let [w, h] = sheet.paper;
    let mut d = Drawing2d { polygons: Vec::new(), images: Vec::new(), name: sheet.title.clone(), space: Space::Paper, paper: Some(sheet.paper), lines: Vec::new(), texts: Vec::new(), dimensions: Vec::new() };
    d.rectangle(Pen::Frame, [0.0, 0.0], [w, h]);
    let block = crate::title_block::title_block(doc, sheet, style);
    if !block.owns_frame {
        d.rectangle(Pen::Frame, [0.02, 0.01], [w - 0.01, h - 0.01]);
    }
    d.images.extend(block.logo);
    d.polygons.extend(block.polygons);
    d.lines.extend(block.lines);
    d.texts.extend(block.texts);
    for v in &sheet.views {
        let c = cut(&v.plan);
        let (dimensions, notes, crop) = marks(doc, &v.plan);
        let extent = c.as_ref().and_then(|c| c.extent);
        let middle = doc.view_origin(&v.plan).or_else(|| extent.map(|(lo, hi)| [(lo[0] + hi[0]) as f64 / 2.0, (lo[1] + hi[1]) as f64 / 2.0])).unwrap_or_default();
        let to = |q: [f64; 2]| v.to_paper(middle, q);
        if let Some(photo) = doc.plan(&v.plan).and_then(|p| p.orthophoto.as_ref()) { d.images.extend(photo.image.placed(crop, &to)); }
        let (lo, hi) = match (crop, extent) {
            (Some((lo, hi)), _) => (to(lo), to(hi)),
            (None, Some((lo, hi))) => {
                (to(lo.map(f64::from)).map(|x| x - FRAME), to(hi.map(f64::from)).map(|x| x + FRAME))
            }
            (None, None) => ([v.at[0] - 0.02, v.at[1] - 0.015], [v.at[0] + 0.02, v.at[1] + 0.015]),
        };
        let drawn = c.as_ref().is_some_and(|c| crate::drawing::drawn(c).next().is_some());
        let rules = rules(&v.plan, v.scale);
        d.view(c.as_deref().filter(|_| drawn), crop, (dimensions, &notes), doc, &to, 1.0, v.scale, &rules);
        // the lines drawn on the view that are no model, and those the model draws by itself
        d.view_details(doc, &v.plan, &to);
        // its grid lines and levels, as its own tab shows them
        let placed: Vec<Datum2> = datums(&v.plan).into_iter().map(|m| Datum2 { a: to(m.a), b: to(m.b), ..m }).collect();
        d.datums(&placed, &rules.grid);
        if doc.view_has_sheet_caption(&v.plan) && (!drawn || crop.is_some()) {
            d.rectangle(Pen::Frame, lo, hi);
        }
        for (label, a, b) in plan_sections(doc, &v.plan) {
            section_mark(&mut d, to(a), to(b), label, &rules.section);
        }
        if !doc.placed_view_has_title(v) { continue; }
        let name = doc.view_name(&v.plan).unwrap_or_default().to_string();
        d.text([lo[0], lo[1] - 0.009], 0.003, name, Align::Left);
        d.text([lo[0], lo[1] - 0.0135], 0.0022, format!("1:{}", v.scale), Align::Left);
    }
    Some(d)
}

/// The drawing of a sheet read from a file (a Revit sheet), on its paper in metres from the
/// paper's lower left corner: its title block (lines and texts), and each viewport that shows a
/// view of the document with that view's lines, its own detail lines, dimensions and texts, where
/// the sheet has it, turned as it lies. Legends, drafting views, schedules and images are left
/// out. `cut` gives a view's cut by its path.
pub fn model_sheet_drawing(doc: &Document, sheet: &crate::model::Sheet, cut: &dyn Fn(&str) -> Option<Arc<Cut>>) -> Drawing2d {
    let covers = sheet.paper.or_else(|| sheet.extent()).unwrap_or(crate::model::SheetRect { min: [0.0, 0.0], max: [0.42, 0.297] });
    let zero = [f64::from(covers.min[0]), f64::from(covers.min[1])];
    let size = [f64::from(covers.max[0]) - zero[0], f64::from(covers.max[1]) - zero[1]];
    let on_paper = |p: [f32; 2]| [f64::from(p[0]) - zero[0], f64::from(p[1]) - zero[1]];
    let mut d = Drawing2d { polygons: Vec::new(), images: Vec::new(), name: sheet.title.clone(), space: Space::Paper, paper: Some(size), lines: Vec::new(), texts: Vec::new(), dimensions: Vec::new() };
    d.rectangle(Pen::Frame, [0.0, 0.0], size);
    for line in &sheet.lines {
        d.line(Pen::Frame, on_paper(line[0]), on_paper(line[1]));
    }
    for t in sheet.texts.iter().filter(|t| !t.text.trim().is_empty()) {
        let align = match t.align {
            1 => Align::Center,
            2 => Align::Right,
            _ => Align::Left,
        };
        // each of its lines, the first at the top of its box
        let leading = f64::from(t.size) * 1.3;
        for (k, row) in t.text.lines().enumerate() {
            let at = on_paper(t.at);
            d.texts.push(Text2d { family: None, bold: false, color: None, pen: Pen::Text, at: [at[0], at[1] - k as f64 * leading], height: f64::from(t.size) * 0.7, text: row.to_string(), align, angle: 0.0 });
        }
    }
    for v in &sheet.viewports {
        let Some(path) = v.drawing.as_deref() else { continue };
        let c = cut(path);
        let (dimensions, notes, crop) = marks(doc, path);
        let extent = c.as_ref().and_then(|c| c.extent).map(|(lo, hi)| (lo.map(f64::from), hi.map(f64::from)));
        // the point of the view's drawing that the sheet holds, and where: as the sheet's tab
        let anchor = match (v.anchor, crop, extent) {
            (Some(anchor), ..) => Some((anchor, v.origin)),
            (None, Some((lo, _)), _) => Some((lo, v.rect.min)),
            (None, None, Some((lo, hi))) => Some(([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0], v.rect.center())),
            (None, None, None) => None,
        };
        let Some((from, at)) = anchor else { continue };
        let (ratio, [cx, cy]) = (f64::from(v.ratio), v.turn);
        let at = on_paper(at);
        let to = |q: [f64; 2]| {
            let (dx, dy) = (q[0] - from[0], q[1] - from[1]);
            [at[0] + (dx * cx + dy * cy) * ratio, at[1] + (dy * cx - dx * cy) * ratio]
        };
        let per_metre = if ratio > 0.0 { 1.0 / ratio } else { SHEET_SCALE };
        d.view(c.as_deref(), crop, (dimensions, &notes), doc, &to, 1.0, per_metre, &Rules::default());
        d.view_details(doc, path, &to);
        // its title, where the sheet has it
        if let Some(label) = v.label {
            let title = v.written.clone().unwrap_or_else(|| format!("{}  {}", v.title, v.scale));
            if !title.trim().is_empty() {
                d.text(on_paper(label), 0.0025, title, Align::Left);
            }
        }
    }
    d
}

/// A section's mark on a floor plan (paper): its line as `stroke` draws it, an arrow at each end
/// the way it looks and its letter past them.
fn section_mark(d: &mut Drawing2d, from: [f64; 2], to: [f64; 2], letter: String, stroke: &Drawn) {
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    let run = dx.hypot(dy);
    if run <= 0.0 {
        return;
    }
    let n = [-dy / run, dx / run];
    d.dashed(Pen::Frame, from, to, stroke, 0.001, &mut 0.0);
    for end in [from, to] {
        d.line_as(Pen::Frame, end, add(end, n, ARROW), stroke);
        let at = add(end, n, ARROW * 1.8);
        d.text([at[0], at[1] - TEXT / 2.0], TEXT, letter.clone(), Align::Center);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_of_a_file_is_drawn_from_its_papers_corner() {
        use crate::model::{DraftText, Sheet, SheetRect};
        let mut sheet = Sheet::new("A-01".into(), "Plans".into(), Some(SheetRect { min: [-0.5, 0.25], max: [-0.08, 0.547] }), Vec::new());
        sheet.lines.push([[-0.5, 0.25], [-0.4, 0.25]]);
        sheet.texts.push(DraftText { element: 1, at: [-0.4, 0.3], width: 0.1, height: 0.01, size: 0.004, align: 0, angle: 0.0, text: "Project\nNumber".into(), face: Default::default() });
        let d = model_sheet_drawing(&Document::default(), &sheet, &|_| None);
        let paper = d.paper.expect("a paper");
        assert!((paper[0] - 0.42).abs() < 1e-6 && (paper[1] - 0.297).abs() < 1e-6, "{paper:?}");
        // the title block's line starts at the paper's corner, and its text is written line by line
        assert!(d.lines.iter().any(|l| l.pen == Pen::Frame && l.a[0].abs() < 1e-6 && l.a[1].abs() < 1e-6 && (l.b[0] - 0.1).abs() < 1e-6));
        let texts: Vec<&str> = d.texts.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["Project", "Number"]);
        assert!(d.texts[1].at[1] < d.texts[0].at[1]);
    }
    use crate::document::A3_LANDSCAPE;

    /// The walls of a room of 8 × 5 m (its lower left at 0, 0) as a plan's cut, and one line seen
    /// below it.
    fn room() -> Cut {
        let segments = vec![
            [[0.0, 0.0], [8.0, 0.0]],
            [[8.0, 0.0], [8.0, 5.0]],
            [[8.0, 5.0], [0.0, 5.0]],
            [[0.0, 5.0], [0.0, 0.0]],
        ];
        Cut { segments, below: vec![[[1.0, 1.0], [7.0, 1.0]]], extent: Some(([0.0, 0.0], [8.0, 5.0])), fills: Vec::new(), ..Cut::default() }
    }

    /// A document with a floor plan of the ground floor, an 8 m dimension and a text on it.
    fn plan() -> (Document, String) {
        let mut doc = Document::default();
        doc.add_level("Begane grond", 0.0).unwrap();
        let level = doc.levels()[0].path.clone();
        let plan = doc.add_plan(&level).unwrap();
        assert!(doc.add_dimension(&plan, [0.0, 0.0], [8.0, 0.0]));
        doc.add_note(&plan, [1.0, 1.0], "Kamer ë");
        (doc, plan)
    }

    fn count(d: &Drawing2d, pen: Pen) -> usize {
        d.lines.iter().filter(|l| l.pen == pen).count()
    }

    #[test]
    fn a_floor_plan_gives_cut_and_beyond_lines_its_dimension_and_note() {
        let (doc, plan) = plan();
        let d = view_drawing(&doc, &plan, &room(), &Rules::default()).unwrap();
        assert_eq!(d.name, "Begane grond");
        assert_eq!(d.space, Space::Model { scale: 100.0 });
        assert_eq!((count(&d, Pen::Cut), count(&d, Pen::Beyond)), (4, 1));
        assert_eq!(d.dimensions.len(), 1);
        assert_eq!(d.dimensions[0].label().text, "8000");
        // 6 mm and 2.5 mm on paper at 1:100
        assert!((d.dimensions[0].offset - 0.6).abs() < 1e-9 && (d.dimensions[0].text_height - 0.25).abs() < 1e-9);
        assert_eq!(d.texts.len(), 1);
        assert_eq!((d.texts[0].text.as_str(), d.texts[0].at, d.texts[0].align), ("Kamer ë", [1.0, 1.0], Align::Left));
        assert!((d.texts[0].height - 0.25).abs() < 1e-9);
        assert!((d.paper_per_unit() - 0.01).abs() < 1e-12);
    }

    #[test]
    fn a_cut_of_a_material_drawn_by_a_colour_is_filled_with_it() {
        let (doc, plan) = plan();
        // a column of reinforced concrete cast in situ (NEN 47: grey, no lines) with a duct
        // through it, and a wall hatched on the paper
        let square = |x: f32, y: f32, side: f32| vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side]];
        let concrete = crate::drawing::Fill { loops: vec![square(1.0, 1.0, 0.4), square(1.1, 1.1, 0.2)], look: Some(([192; 3], [0; 3])), ..Default::default() };
        let brick = crate::drawing::Fill { loops: vec![square(3.0, 1.0, 0.4)], look: Some(([255; 3], [0; 3])), ..Default::default() };
        let cut = Cut { fills: vec![concrete, brick], ..room() };
        let d = view_drawing(&doc, &plan, &cut, &Rules { hatched: true, ..Rules::default() }).unwrap();
        assert!(!d.polygons.is_empty() && d.polygons.iter().all(|p| p.color == [192; 3]), "the concrete alone, in its grey");
        let area: f64 = d.polygons.iter().map(|p| (0..p.points.len()).map(|k| { let (a, b) = (p.points[k], p.points[(k + 1) % p.points.len()]); a[0] * b[1] - b[0] * a[1] }).sum::<f64>().abs() / 2.0).sum();
        assert!((area - (0.16 - 0.04)).abs() < 1e-6, "the duct through it stays open: {area}");
        // a drawing that hatches nothing fills nothing
        assert!(view_drawing(&doc, &plan, &cut, &Rules { hatched: false, ..Rules::default() }).unwrap().polygons.is_empty());
    }

    #[test]
    fn a_chain_draws_each_projected_interval_and_an_optional_separate_total() {
        let (mut doc, plan) = plan();
        assert!(doc.insert_dimension_reference(&plan, 0, [3.,1.], None));
        let draw = |doc: &Document| view_drawing(doc, &plan, &room(), &Rules::default()).unwrap();
        let d = draw(&doc);
        assert_eq!(d.dimensions.iter().map(|d| d.value).collect::<Vec<_>>(), [3.,5.]);
        assert_eq!(d.dimensions[0].witnesses[1], [3.,1.]);
        assert!(doc.add_dimension_chain_total(&plan, 0));
        assert!(!doc.add_dimension_chain_total(&plan, 0));
        let d = draw(&doc);
        assert_eq!(d.dimensions[2].value, 8.);
        assert!(d.dimensions[2].offset > d.dimensions[0].offset);
    }

    #[test]
    fn vertical_aligned_figures_read_upward_on_either_side_of_a_chain() {
        let (mut doc, plan) = plan();
        let style = crate::document::DimensionStyle::from_ifcx(&serde_json::json!({"name":"3BM ronde stip"})).unwrap();
        let k = doc.set_dimension_style(style);
        assert!(doc.add_dimension_chain(&plan,&[[1.,6.],[1.,4.],[1.,0.]]));
        assert!(doc.set_dimension_style_of(&plan,1,k));
        let drawing = view_drawing(&doc,&plan,&room(),&Rules::default()).unwrap();
        for dim in &drawing.dimensions[1..] {
            let label = dim.label();
            assert_eq!(label.angle, std::f64::consts::FRAC_PI_2);
            let (line,_) = dim.line().unwrap();
            assert!((label.at[0] - (line[0]-0.080)).abs() < 1e-9);
        }
    }

    #[test]
    fn a_dimension_has_its_line_to_the_left_with_slashes_and_its_text_beside_it() {
        let dim = Dim2d { first_witness: true, units_per_paper_metre: 100., witnesses: [[0.0, 0.0], [8.0, 0.0]], from: [0.0, 0.0], to: [8.0, 0.0], value: 8.0, offset: 0.6, text_height: 0.25, style: crate::document::DimensionStyle::standard() };
        let lines = dim.lines();
        // two extension lines, the dimension line, two slashes
        assert_eq!(lines.len(), 5);
        assert!(lines.contains(&[[0.0, 0.6], [8.0, 0.6]]), "{lines:?}");
        let label = dim.label();
        assert_eq!((label.text.as_str(), label.align), ("8000", Align::Center));
        assert!((label.at[0] - 4.0).abs() < 1e-9 && label.at[1] > 0.6, "{label:?}");
    }

    #[test]
    fn a_crop_leaves_out_what_is_outside_it() {
        let (mut doc, plan) = plan();
        assert!(doc.set_crop(&plan, Some(([0.0, 0.0], [4.0, 5.0]))));
        let d = view_drawing(&doc, &plan, &room(), &Rules::default()).unwrap();
        // the right wall is outside; the lower and upper walls are cut back to x = 4
        assert_eq!(count(&d, Pen::Cut), 3);
        assert!(d.lines.iter().all(|l| l.a[0] <= 4.0 + 1e-9 && l.b[0] <= 4.0 + 1e-9), "{:?}", d.lines);
    }

    #[test]
    fn two_placements_have_independent_titles_after_reopening() {
        let (mut doc, plan) = plan();
        let sheet = doc.add_sheet("Blad");
        doc.place_plan(&sheet, &plan).unwrap();
        doc.place_plan(&sheet, &plan).unwrap();
        doc.put_placed(&sheet, 0, [0.09, 0.12]);
        doc.put_placed(&sheet, 1, [0.28, 0.12]);
        let render = |doc: &Document| sheet_drawing(doc, &sheet, &|_| None, &|_| vec![], &|_, _| Rules::default()).unwrap();
        let name = doc.view_name(&plan).unwrap().to_owned();
        let title_x = |d: &Drawing2d| d.texts.iter().filter(|t| t.text == name).map(|t| t.at[0]).collect::<Vec<_>>();
        let normal = render(&doc);
        let both = title_x(&normal);
        assert_eq!(both.len(), 2);
        doc.set_placed_title(&sheet, 0, Some(false));
        assert_eq!(title_x(&render(&doc)), both[1..]);
        assert_eq!(count(&render(&doc), Pen::Frame), count(&normal, Pen::Frame));
        doc.set_view_drawing(&plan, Some("pile-plan".into()));
        assert!(title_x(&render(&doc)).is_empty());
        let frames = count(&render(&doc), Pen::Frame);
        doc.set_placed_title(&sheet, 1, Some(true));
        let nodes = doc.ifcx_drawing_nodes().into_iter().map(|n| (n["path"].as_str().unwrap().to_owned(), n["attributes"].clone())).collect::<Vec<_>>();
        let mut reopened = Document::default();
        reopened.add_level("Begane grond", 0.0).unwrap();
        reopened.adopt_ifcx(&nodes);
        let places = &reopened.sheet(&sheet).unwrap().views;
        assert_eq!((places[0].show_title, places[1].show_title), (Some(false), Some(true)));
        let drawing = render(&reopened);
        assert_eq!(title_x(&drawing), both[1..]);
        assert_eq!(count(&drawing, Pen::Frame), frames);
        reopened.set_placed_title(&sheet, 1, None);
        assert!(title_x(&render(&reopened)).is_empty());
        let sheet_node = reopened.ifcx_drawing_nodes().into_iter().find(|n| n["path"] == sheet).unwrap();
        assert!(sheet_node["attributes"]["o3v::sheet"]["views"][1].get("showTitle").is_none());
    }

    #[test]
    fn a_sheet_places_a_view_at_its_scale_around_its_middle() {
        let (mut doc, plan) = plan();
        let sheet = doc.add_sheet("Plattegronden");
        doc.place_plan(&sheet, &plan).unwrap();
        assert!(doc.move_placed(&sheet, 0, [0.2, 0.15]));
        let cut = Arc::new(room());
        let d = sheet_drawing(&doc, &sheet, &|p| (p == plan).then(|| cut.clone()), &|_| Vec::new(), &|_, _| Rules::default()).unwrap();
        assert_eq!(d.space, Space::Paper);
        // the room's middle (4, 2.5) stands at (0.2, 0.15); the 8 m wall is 8 cm long
        let wall = d.lines.iter().find(|l| l.pen == Pen::Cut && (l.a[1] - l.b[1]).abs() < 1e-12 && l.a[1] < 0.13).unwrap();
        let (lo, hi) = (wall.a[0].min(wall.b[0]), wall.a[0].max(wall.b[0]));
        assert!((lo - 0.16).abs() < 1e-9 && (hi - 0.24).abs() < 1e-9 && (wall.a[1] - 0.125).abs() < 1e-9, "{wall:?}");
        // the dimension measures the model, not the paper
        assert_eq!(d.dimensions[0].label().text, "8000");
        assert!(d.texts.iter().any(|t| t.text == "Kamer ë"));
        assert!(d.texts.iter().any(|t| t.text == "1:100"));
    }

    #[test]
    fn a_sheet_has_its_frame_and_title() {
        let mut doc = Document::default();
        let sheet = doc.add_sheet("Plattegronden");
        let d = sheet_drawing(&doc, &sheet, &|_| None, &|_| Vec::new(), &|_, _| Rules::default()).unwrap();
        assert_eq!(d.paper, Some(A3_LANDSCAPE));
        let near = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).abs() < 1e-12 && (p[1] - q[1]).abs() < 1e-12;
        assert!(d.lines.iter().any(|l| l.pen == Pen::Frame && near(l.a, [0.02, 0.01]) && near(l.b, [0.41, 0.01])), "{:?}", d.lines);
        for value in ["A01", "Plattegronden", "A3", "OpenAEC"] {
            assert!(d.texts.iter().any(|t| t.text == value), "{value}");
        }
        assert_eq!(d.page(), ([0.0, 0.0], A3_LANDSCAPE));
    }

    #[test]
    fn an_empty_cut_gives_a_drawing_without_lines() {
        let mut doc = Document::default();
        doc.add_level("Begane grond", 0.0).unwrap();
        let level = doc.levels()[0].path.clone();
        let plan = doc.add_plan(&level).unwrap();
        let d = view_drawing(&doc, &plan, &Cut::default(), &Rules::default()).unwrap();
        assert!(d.lines.is_empty() && d.texts.is_empty() && d.dimensions.is_empty());
        assert_eq!(d.extent(), None);
        // a page of 10 x 10 cm on paper: 10 x 10 m at 1:100
        let (lo, hi) = d.page();
        assert!((hi[0] - lo[0] - 10.0).abs() < 1e-9 && (hi[1] - lo[1] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_crop_trims_crossing_section_references_and_omits_outside_references() {
        let (mut doc, path) = plan();
        doc.set_crop(&path, Some(([0.,0.],[8.,5.])));
        doc.add_section("Section A", [2.,-200.], [2.,300.], 10.).unwrap();
        doc.add_section("Section B", [-9.,-200.], [-9.,300.], 10.).unwrap();
        let marks = plan_sections(&doc, &path);
        assert_eq!(marks.len(), 1);
        let (label,a,b) = &marks[0];
        assert_eq!(label,"A");
        assert!((a[1]).abs() < 1e-9 && (b[1]-5.).abs() < 1e-9);
    }

    #[test]
    fn a_sheets_edges_frames_and_section_marks_are_on_the_frame_pen() {
        let (mut doc, plan) = plan();
        let sheet = doc.add_sheet("Plattegronden");
        doc.place_plan(&sheet, &plan).unwrap();
        let base = sheet_drawing(&doc, &sheet, &|_| None, &|_| Vec::new(), &|_, _| Rules::default()).unwrap();
        doc.add_section("Section A", [0.0, 2.0], [8.0, 2.0], 10.0).unwrap();
        // the section's line continuous: the line and an arrow at each end
        let whole = |colour| Rules { section: Drawn { width: 0.5, dashes: Vec::new(), colour }, ..Rules::default() };
        let d = sheet_drawing(&doc, &sheet, &|_| None, &|_| Vec::new(), &|_, _| whole(Some([0, 0, 200]))).unwrap();
        let marks: Vec<&Line2d> = d.lines.iter().filter(|l| l.color == Some([0, 0, 200])).collect();
        assert!(marks.len() == 3 && marks.iter().all(|l| l.pen == Pen::Frame && l.width == 0.5), "{marks:?}");
        // by itself a chain line, as the sheet's tab draws it: 8 m at 1:100 is 80 mm on paper,
        // a long dash and a short one to every 11.35 mm begun, and the two arrows
        let chain = sheet_drawing(&doc, &sheet, &|_| None, &|_| Vec::new(), &|_, _| Rules::default()).unwrap();
        let scale = doc.sheet(&sheet).unwrap().views[0].scale;
        let periods = 8000.0 / scale / 11.35;
        assert!(count(&chain, Pen::Frame) as f64 >= count(&base, Pen::Frame) as f64 + 2.0 * periods.floor() + 2.0, "{} at 1:{scale}", count(&chain, Pen::Frame));
        let d = sheet_drawing(&doc, &sheet, &|_| None, &|_| Vec::new(), &|_, _| whole(None)).unwrap();
        // a level of the view goes with it: its line and its name at both ends, at the view's place
        let with = sheet_drawing(&doc, &sheet, &|_| None, &|_| vec![Datum2 { name: "01".into(), a: [-1.0, 3.0], b: [1.0, 3.0], level: Some(3.0) }], &|_, _| whole(None)).unwrap();
        assert!(count(&with, Pen::Grids) > count(&d, Pen::Grids));
        assert_eq!(with.texts.iter().filter(|t| t.text == "01 +3.000").count(), 2);
        assert_eq!(count(&d, Pen::Frame), count(&base, Pen::Frame) + 3);
        // cut: only what lies beyond is thin
        let cut = Arc::new(room());
        let d = sheet_drawing(&doc, &sheet, &|p| (p == plan).then(|| cut.clone()), &|_| Vec::new(), &|_, _| Rules::default()).unwrap();
        assert_eq!(count(&d, Pen::Beyond), 1, "{:?}", d.lines);
    }

    #[test]
    fn lines_are_drawn_as_their_line_style_says() {
        let (doc, plan) = plan();
        // a cut of two cut lines round a corner, one edge beyond and one hidden
        let cut = Cut {
            segments: vec![[[0.0, 0.0], [4.0, 0.0]], [[4.0, 0.0], [4.0, 3.0]]],
            below: vec![[[1.0, 1.0], [3.0, 1.0]]],
            hidden: vec![[[0.0, 2.0], [4.0, 2.0]]],
            extent: Some(([0.0, 0.0], [4.0, 3.0])),
            ..Cut::default()
        };
        let line = |width: f64, dashes: &[f64], colour| Drawn { width, dashes: dashes.to_vec(), colour };
        // what is cut red in dashes of 10 mm and gaps of 5 mm, what is seen beyond blue and whole
        let rules = Rules { cut: line(0.7, &[10.0, 5.0], Some([200, 0, 0])), beyond: line(0.25, &[], Some([0, 0, 200])), hidden: line(0.13, &[1.0, 1.0], None), ..Rules::default() };
        let d = view_drawing(&doc, &plan, &cut, &rules).unwrap();
        let Space::Model { scale } = d.space else { panic!("a view's drawing is in the model's metres") };
        let of = |pen: Pen, width: f64| d.lines.iter().filter(|l| l.pen == pen && l.width == width).collect::<Vec<_>>();
        let length = |lines: &[&Line2d]| lines.iter().map(|l| (l.b[0] - l.a[0]).hypot(l.b[1] - l.a[1])).sum::<f64>();
        // the 7 m of cut line is one run: two thirds of it drawn, and the pattern runs on round
        // the corner instead of beginning again there
        let cut_lines = of(Pen::Cut, 0.7);
        assert!(cut_lines.iter().all(|l| l.color == Some([200, 0, 0])));
        let (dash, period) = (0.010 * scale, 0.015 * scale);
        let drawn: f64 = (0..).map(|k| k as f64 * period).take_while(|from| *from < 7.0).map(|from| (from + dash).min(7.0) - from).sum();
        assert!((length(&cut_lines) - drawn).abs() < 1e-6, "{} of {drawn}", length(&cut_lines));
        assert!(cut_lines.iter().all(|l| (l.b[0] - l.a[0]).hypot(l.b[1] - l.a[1]) <= dash + 1e-9));
        // what is seen beyond whole and blue; what is hidden half drawn, in no colour of its own
        let beyond = of(Pen::Beyond, 0.25);
        assert!(beyond.len() == 1 && beyond[0].color == Some([0, 0, 200]) && beyond[0].a == [1.0, 1.0] && beyond[0].b == [3.0, 1.0], "{beyond:?}");
        let hidden = of(Pen::Beyond, 0.13);
        assert!(hidden.iter().all(|l| l.color.is_none()) && (length(&hidden) - 2.0).abs() < 0.002 * scale, "{}", length(&hidden));
        // a writer gets the colours with the widths
        assert!(d.strokes(Pen::Cut).iter().all(|(width, colour, _)| *width == 0.7 && *colour == [200, 0, 0]));
        // grid lines in their own style: the width and colour of the bubbles too, and no dashes
        // for a continuous one
        let mut with = d.clone();
        with.datums(&[Datum2 { name: "A".into(), a: [0.0, -1.0], b: [0.0, 6.0], level: Some(0.0) }], &line(0.35, &[], Some([0, 150, 0])));
        let grids: Vec<&Line2d> = with.lines.iter().filter(|l| l.pen == Pen::Grids).collect();
        assert!(grids.len() == 1 && grids[0].width == 0.35 && grids[0].color == Some([0, 150, 0]) && grids[0].b == [0.0, 6.0], "{grids:?}");
        // a pattern that cannot be drawn (no length at all) draws the line whole
        let mut odd = d.clone();
        odd.datums(&[Datum2 { name: "A".into(), a: [0.0, -1.0], b: [0.0, 6.0], level: Some(0.0) }], &line(0.35, &[0.0, 0.0], None));
        assert_eq!(odd.lines.iter().filter(|l| l.pen == Pen::Grids).count(), 1);
    }

    #[test]
    fn pens_have_their_layer_and_weight() {
        assert_eq!(Pen::ALL.map(Pen::name), ["O3V-Cut", "O3V-Beyond", "O3V-Hatch", "O3V-Dimensions", "O3V-Text", "O3V-Frame", "O3V-Grids", "O3V-Detail"]);
        assert_eq!(Pen::ALL.map(Pen::weight_mm), [0.50, 0.18, 0.13, 0.18, 0.25, 0.35, 0.18, 0.25]);
    }

    #[test]
    fn grids_and_levels_are_drawn_with_bubbles_names_and_heights() {
        let (doc, plan) = plan();
        let mut d = view_drawing(&doc, &plan, &room(), &Rules::default()).unwrap();
        d.datums(&[
            Datum2 { name: "A".into(), a: [0.0, -1.0], b: [0.0, 6.0], level: None },
            Datum2 { name: "01".into(), a: [-1.0, 3.0], b: [9.0, 3.0], level: Some(3.0) },
        ], &Rules::default().grid);
        let grid_lines = d.lines.iter().filter(|l| l.pen == Pen::Grids).count();
        // the two lines, and a round bubble at both ends of the grid
        assert!(grid_lines >= 2 + 2 * 16, "{grid_lines}");
        assert_eq!(d.texts.iter().filter(|t| t.text == "A" && t.align == Align::Center).count(), 2);
        // the bubbles stand past the grid's ends: the one below under y = -1
        assert!(d.texts.iter().any(|t| t.text == "A" && t.at[1] < -1.0), "{:?}", d.texts);
        // the level's name and height past both ends
        let level: Vec<&Text2d> = d.texts.iter().filter(|t| t.text == "01 +3.000").collect();
        assert_eq!(level.len(), 2, "{:?}", d.texts);
        assert!(level.iter().any(|t| t.at[0] > 9.0 && t.align == Align::Left) && level.iter().any(|t| t.at[0] < -1.0 && t.align == Align::Right));
    }
}
