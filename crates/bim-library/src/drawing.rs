//! Drawing standards: the kinds of drawing an office makes and the rules each is drawn by, and
//! under them the settings those rules pick from: the scales, the line patterns, the line
//! weights (pens), the line styles and the text styles.
//!
//! The three of a line are kept apart, as Revit and AutoCAD keep them:
//!
//! - A **line pattern** ([`LinePattern`]) is dashes and gaps: Revit's line pattern, AutoCAD's
//!   linetype. A file of AutoCAD linetypes is read and written ([`from_lin`], [`lin`]).
//! - A **line weight** ([`Pen`]) is a width on paper, and where an office wants it another width
//!   at a scale ([`Pen::width_at`]), as Revit's model line weights have a width per scale.
//! - A **line style** ([`LineStyle`]) is a pen, a pattern and where it has one a colour under a
//!   name ("hidden line", "centre line"): Revit's line style, and what a layer gives a line in
//!   AutoCAD.
//!
//! A kind of drawing draws each of its lines ([`Line`]: what is cut, what is seen beyond, what is
//! hidden, grid lines, section lines) in a line style, or by its pens ([`Standard::stroke`]).
//!
//! A standard belongs to a country. The suite starts with the Dutch one ([`Standard::netherlands`]):
//!
//! - The kinds, their views, scales and numbers are those of the drawings and sheets of the INB
//!   template of the OpenAEC Foundation (`INB-Template 0.6.ifc`). A kind the template has no
//!   drawing of is added from Dutch practice and says so ([`Origin::Practice`]); its scales are a
//!   proposal.
//! - The line patterns are the fifteen basic line types of ISO 128-2, their dashes and gaps as
//!   multiples of the line width, as that standard gives them.
//! - The line styles are a proposal: the uses ISO 128 gives its line types (hidden, centre,
//!   cutting plane, in front of the cutting plane) and the ones a Revit template starts with.
//! - The line widths and the text heights are the ISO series. What is cut is drawn twice as wide
//!   as what is seen beyond, the ratio of wide to narrow lines of ISO 128.
//! - The hatches are those of the country's standard ([`DrawingStandard`]).
//!
//! Nothing here is fixed: a standard is data an office changes, kept in IFCX ([`nodes`],
//! [`read`]), one node per setting, so a file can hold a whole standard or one changed pen.

use crate::material::{DrawingStandard, Names};
use crate::{uuid, Lang, LibraryError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// The attribute of a standard's own node.
pub const STANDARD: &str = "openaec::drawing::standard";
/// The attribute of a kind of drawing.
pub const KIND: &str = "openaec::drawing::kind";
/// The attribute of a line pattern. It is called a line style in the file, as a pattern was
/// called before a line style was a pen, a pattern and a colour together; files keep reading.
pub const LINE_PATTERN: &str = "openaec::drawing::linestyle";
/// The attribute of a line style.
pub const LINE_STYLE: &str = "openaec::drawing::line";
/// The attribute of a pen.
pub const PEN: &str = "openaec::drawing::pen";
/// The attribute of a text style.
pub const TEXT_STYLE: &str = "openaec::drawing::textstyle";

/// What a drawing is a view of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum View {
    /// Seen from above, cut at a height.
    Plan,
    /// A ceiling seen from below, drawn as a plan.
    CeilingPlan,
    Section,
    Elevation,
    /// The model in 3D.
    Model,
}

impl View {
    pub const ALL: [View; 5] = [View::Plan, View::CeilingPlan, View::Section, View::Elevation, View::Model];
}

/// Whose drawing it is. Its code stands in a drawing's name (`01_BWK_PLG_00_begane_grond`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Discipline {
    General,
    Architecture,
    Structure,
    Services,
}

impl Discipline {
    pub const ALL: [Discipline; 4] = [Discipline::General, Discipline::Architecture, Discipline::Structure, Discipline::Services];

    /// Its code in the Netherlands, as the INB template writes it; the template has no drawings
    /// of services, so `INS` is the suite's.
    pub fn code(self) -> &'static str {
        match self {
            Discipline::General => "ALG",
            Discipline::Architecture => "BWK",
            Discipline::Structure => "CON",
            Discipline::Services => "INS",
        }
    }
}

/// Where a kind of drawing comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// The country's template has drawings of this kind.
    Template,
    /// Usual in practice; the template has none, and its scales are a proposal.
    Practice,
    /// Added by the office.
    Own,
}

/// A line pattern: a continuous line, or dashes and gaps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinePattern {
    pub key: String,
    pub names: Names,
    /// Dash, gap, dash, gap, ... as multiples of the line's width; empty for a continuous line.
    /// A dot is a dash of half a width ([`DOT`]).
    pub dashes: Vec<f64>,
    /// Its number among the line types of ISO 128-2 (1 to 15), where it is one of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u8>,
}

/// A dot of a line pattern: a dash of half the line's width, as ISO 128-2 has it.
pub const DOT: f64 = 0.5;

impl LinePattern {
    /// Its dashes and gaps in millimetres on paper, drawn `width` wide.
    pub fn lengths(&self, width: f64) -> Vec<f64> {
        self.dashes.iter().map(|d| d * width).collect()
    }
}

/// A pen: a line weight, the width of a line on paper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pen {
    pub key: String,
    pub names: Names,
    /// Millimetres on paper.
    pub width: f64,
    /// Its width at a scale (1 to the key) where it is another there, in millimetres on paper:
    /// an office that draws a pen narrower at 1:200 than at 1:50 says so here.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub at: std::collections::BTreeMap<u32, f64>,
}

impl Pen {
    /// A pen of one width at every scale.
    pub fn new(key: &str, width: f64) -> Self {
        Self { key: key.into(), names: Names::same(&format!("{width} mm")), width, at: Default::default() }
    }

    /// Its width at `scale` (1 to this): the one given for that scale, else the one of the
    /// nearest scale that has one when the pen has any, else its own. Nearest is by ratio, so
    /// 1:75 takes after 1:100 and not after 1:50; of two as near, the larger scale.
    pub fn width_at(&self, scale: u32) -> f64 {
        if let Some(width) = self.at.get(&scale) {
            return *width;
        }
        let far = |other: u32| (f64::from(other.max(1)) / f64::from(scale.max(1))).ln().abs();
        self.at.iter().min_by(|a, b| far(*a.0).total_cmp(&far(*b.0))).map_or(self.width, |(_, width)| *width)
    }
}

/// A line style: a line weight, a line pattern and where it has one a colour, under a name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineStyle {
    pub key: String,
    pub names: Names,
    /// The key of its pen.
    pub pen: String,
    /// The key of its line pattern.
    pub pattern: String,
    /// Its colour, red, green and blue; without one a line has the colour of what it is drawn
    /// for (black on paper).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<[u8; 3]>,
}

/// A line as a line style draws it at a scale.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    /// Millimetres on paper.
    pub width: f64,
    /// Dash, gap, dash, gap, ... in millimetres on paper; empty for a continuous line.
    pub dashes: Vec<f64>,
    pub colour: Option<[u8; 3]>,
}

/// What a drawing draws a line for. A kind of drawing gives each a line style, or leaves it to
/// its pens ([`Standard::stroke`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Line {
    /// Where the plan's or section's plane cuts the model.
    Cut,
    /// What is seen beyond the cut.
    Beyond,
    /// What is hidden behind what is seen.
    Hidden,
    /// Grid lines and levels.
    Grid,
    /// The line of a section on a floor plan.
    Section,
}

impl Line {
    pub const ALL: [Line; 5] = [Line::Cut, Line::Beyond, Line::Hidden, Line::Grid, Line::Section];

    /// Its name in a file: "cut", "grid".
    pub fn key(self) -> &'static str {
        match self {
            Line::Cut => "cut",
            Line::Beyond => "beyond",
            Line::Hidden => "hidden",
            Line::Grid => "grid",
            Line::Section => "section",
        }
    }

    pub fn of_key(key: &str) -> Option<Line> {
        Line::ALL.into_iter().find(|line| line.key() == key)
    }
}

/// A text style: a text height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub key: String,
    pub names: Names,
    /// Millimetres on paper.
    pub height: f64,
}

/// A kind of drawing and the rules it is drawn by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingKind {
    pub key: String,
    pub names: Names,
    /// Its code in a drawing's name ("PLG", "aanzicht").
    pub code: String,
    pub discipline: Discipline,
    pub view: View,
    /// The scales it is drawn at, as 1 to this, the usual one first.
    pub scales: Vec<u32>,
    /// The series its drawings are numbered in ("01"), or empty.
    #[serde(default)]
    pub number: String,
    /// The pen of what is cut.
    pub cut: String,
    /// The pen of what is seen beyond the cut.
    pub beyond: String,
    /// The line style of what is hidden.
    pub hidden: String,
    /// The text style of its texts and dimensions.
    pub text: String,
    /// Whether what is cut is hatched by its material.
    pub hatched: bool,
    /// The line style each line is drawn in, by its key. A line without one is drawn by the
    /// pens above: what is cut and what is seen beyond continuous in their pen, what is hidden
    /// in the pen of what is seen beyond; grid lines and section lines as a drawing draws them
    /// by itself.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub lines: std::collections::BTreeMap<Line, String>,
    pub origin: Origin,
}

impl DrawingKind {
    /// Its usual scale as it is written: "1:50".
    pub fn scale_text(&self) -> String {
        self.scales.first().map(|s| scale_text(*s)).unwrap_or_default()
    }
}

/// A scale as it is written: "1:50".
pub fn scale_text(scale: u32) -> String {
    format!("1:{scale}")
}

/// A drawing standard: its kinds of drawing and the settings they pick from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Standard {
    pub key: String,
    pub names: Names,
    /// The standard its hatches follow.
    pub hatches: DrawingStandard,
    /// The scales drawings are made at, as 1 to this.
    pub scales: Vec<u32>,
    /// The number of its first sheet; the next ones count on.
    pub first_sheet: u32,
    #[serde(default, skip_serializing)]
    pub line_patterns: Vec<LinePattern>,
    #[serde(default, skip_serializing)]
    pub pens: Vec<Pen>,
    #[serde(default, skip_serializing)]
    pub line_styles: Vec<LineStyle>,
    #[serde(default, skip_serializing)]
    pub text_styles: Vec<TextStyle>,
    #[serde(default, skip_serializing)]
    pub kinds: Vec<DrawingKind>,
}

impl Standard {
    /// The Dutch standard the suite starts with (see the module).
    pub fn netherlands() -> Self {
        let mut number = 0;
        let mut pattern = |key: &str, nl: &str, en: &str, de: &str, fr: &str, dashes: &[f64]| {
            number += 1;
            LinePattern { key: key.into(), names: Names::new(nl, en, de, fr), dashes: dashes.to_vec(), number: Some(number) }
        };
        let pen = |width: f64| Pen::new(&format!("{width}"), width);
        let line = |key: &str, nl: &str, en: &str, de: &str, fr: &str, pen: &str, pattern: &str| LineStyle { key: key.into(), names: Names::new(nl, en, de, fr), pen: pen.into(), pattern: pattern.into(), colour: None };
        let text = |height: f64| {
            let name = format!("{height} mm");
            TextStyle { key: format!("{height}"), names: Names::same(&name), height }
        };
        // what is cut twice as wide as what is seen beyond, the wider the larger the scale
        let pens = |scale: u32| match scale {
            0..=20 => ("0.7", "0.35"),
            21..=50 => ("0.5", "0.25"),
            _ => ("0.35", "0.18"),
        };
        #[allow(clippy::too_many_arguments)]
        let kind = |key: &str, names: Names, code: &str, discipline, view, scales: &[u32], number: &str, origin| {
            let (cut, beyond) = pens(scales[0]);
            DrawingKind {
                key: key.into(),
                names,
                code: code.into(),
                discipline,
                view,
                scales: scales.to_vec(),
                number: number.into(),
                cut: cut.into(),
                beyond: beyond.into(),
                hidden: "dashed".into(),
                text: "2.5".into(),
                hatched: !matches!(view, View::Elevation | View::Model),
                // what is cut and seen beyond by the pens of the scale; the rest by a line style
                lines: [(Line::Hidden, "hidden"), (Line::Grid, "grid"), (Line::Section, "cutting-plane")].map(|(line, style)| (line, style.to_string())).into(),
                origin,
            }
        };
        use Discipline::{Architecture, General, Services, Structure};
        use Origin::{Practice, Template};
        Standard {
            key: "nl".into(),
            names: Names::new("Nederland", "Netherlands", "Niederlande", "Pays-Bas"),
            hatches: DrawingStandard::Nen47,
            // the scales the template has a style sheet for, and the two of a site plan
            scales: vec![1, 2, 5, 10, 20, 50, 100, 200, 500, 1000],
            first_sheet: 300,
            // ISO 128-2, types 01 to 15 in their order: dots half a line width, gaps 3, short
            // dashes 6, dashes 12, long dashes 24 and wide gaps 18 times the line width
            line_patterns: vec![
                pattern("continuous", "doorgetrokken", "continuous", "Volllinie", "continu", &[]),
                pattern("dashed", "streeplijn", "dashed", "Strichlinie", "interrompu", &[12.0, 3.0]),
                pattern("dashed-spaced", "streeplijn, wijd", "dashed spaced", "Strich-Abstandlinie", "interrompu espacé", &[12.0, 18.0]),
                pattern("chain", "streep-stiplijn", "long-dashed dotted", "Strich-Punktlinie", "mixte", &[24.0, 3.0, DOT, 3.0]),
                pattern("double-chain", "streep-stip-stiplijn", "long-dashed double-dotted", "Strich-Zweipunktlinie", "mixte à deux points", &[24.0, 3.0, DOT, 3.0, DOT, 3.0]),
                pattern("triple-chain", "streep-stip-stip-stiplijn", "long-dashed triplicate-dotted", "Strich-Dreipunktlinie", "mixte à trois points", &[24.0, 3.0, DOT, 3.0, DOT, 3.0, DOT, 3.0]),
                pattern("dotted", "stippellijn", "dotted", "Punktlinie", "pointillé", &[DOT, 3.0]),
                pattern("long-short", "lange en korte streep", "long-dashed short-dashed", "Strich-Kurzstrichlinie", "mixte à tiret court", &[24.0, 3.0, 6.0, 3.0]),
                pattern("long-double-short", "lange en twee korte strepen", "long-dashed double-short-dashed", "Strich-Zweikurzstrichlinie", "mixte à deux tirets courts", &[24.0, 3.0, 6.0, 3.0, 6.0, 3.0]),
                pattern("dash-dot", "korte streep-stiplijn", "dashed dotted", "Kurzstrich-Punktlinie", "tiret-point", &[12.0, 3.0, DOT, 3.0]),
                pattern("double-dash-dot", "twee strepen-stiplijn", "double-dashed dotted", "Zweistrich-Punktlinie", "deux tirets-point", &[12.0, 3.0, 12.0, 3.0, DOT, 3.0]),
                pattern("dash-double-dot", "korte streep-stip-stiplijn", "dashed double-dotted", "Kurzstrich-Zweipunktlinie", "tiret-deux points", &[12.0, 3.0, DOT, 3.0, DOT, 3.0]),
                pattern("double-dash-double-dot", "twee strepen-stip-stiplijn", "double-dashed double-dotted", "Zweistrich-Zweipunktlinie", "deux tirets-deux points", &[12.0, 3.0, 12.0, 3.0, DOT, 3.0, DOT, 3.0]),
                pattern("dash-triple-dot", "korte streep-drie stippenlijn", "dashed triplicate-dotted", "Kurzstrich-Dreipunktlinie", "tiret-trois points", &[12.0, 3.0, DOT, 3.0, DOT, 3.0, DOT, 3.0]),
                pattern("double-dash-triple-dot", "twee strepen-drie stippenlijn", "double-dashed triplicate-dotted", "Zweistrich-Dreipunktlinie", "deux tirets-trois points", &[12.0, 3.0, 12.0, 3.0, DOT, 3.0, DOT, 3.0, DOT, 3.0]),
            ],
            // the ISO series of line widths
            pens: [0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1.0, 1.4, 2.0].map(pen).to_vec(),
            // what a line is drawn for: the uses of ISO 128 and of a Revit template's line styles
            line_styles: vec![
                line("thin", "dunne lijn", "thin line", "schmale Linie", "trait fin", "0.18", "continuous"),
                line("medium", "middeldikke lijn", "medium line", "mittlere Linie", "trait moyen", "0.35", "continuous"),
                line("wide", "dikke lijn", "wide line", "breite Linie", "trait fort", "0.5", "continuous"),
                line("extra-wide", "extra dikke lijn", "extra wide line", "sehr breite Linie", "trait très fort", "0.7", "continuous"),
                line("hidden", "verborgen lijn", "hidden line", "verdeckte Kante", "arête cachée", "0.18", "dashed"),
                line("overhead", "boven het snijvlak", "above the cut", "vor der Schnittebene", "en avant du plan de coupe", "0.18", "double-chain"),
                line("centre", "hartlijn", "centre line", "Mittellinie", "ligne d'axe", "0.13", "chain"),
                line("grid", "stramienlijn", "grid line", "Rasterlinie", "ligne de trame", "0.18", "chain"),
                line("cutting-plane", "snijlijn", "cutting plane", "Schnittebene", "plan de coupe", "0.5", "chain"),
                line("demolished", "te slopen", "to be demolished", "Abbruch", "à démolir", "0.18", "dashed-spaced"),
                line("boundary", "erfgrens", "plot boundary", "Grundstücksgrenze", "limite de parcelle", "0.35", "long-double-short"),
            ],
            text_styles: [1.8, 2.5, 3.5, 5.0, 7.0].map(text).to_vec(),
            kinds: vec![
                kind("overview", Names::new("overzicht", "overview", "Übersicht", "vue d'ensemble"), "totaaloverzicht", General, View::Plan, &[50], "", Template),
                kind("floor-plan", Names::new("plattegrond", "floor plan", "Grundriss", "plan d'étage"), "PLG", Architecture, View::Plan, &[50, 100], "01", Template),
                kind("foundation-plan", Names::new("funderingsplattegrond", "foundation plan", "Fundamentplan", "plan de fondation"), "PLG", Architecture, View::Plan, &[50], "01", Template),
                kind("area-plan", Names::new("BVO-plattegrond", "gross floor area plan", "BGF-Plan", "plan des surfaces brutes"), "BVO", Architecture, View::Plan, &[50], "02", Template),
                kind("elevation", Names::new("aanzicht", "elevation", "Ansicht", "élévation"), "aanzicht", Architecture, View::Elevation, &[50, 100], "", Template),
                kind("section", Names::new("doorsnede", "section", "Schnitt", "coupe"), "doorsnede", Architecture, View::Section, &[50, 100], "", Template),
                kind("detail", Names::new("detail", "detail", "Detail", "détail"), "detail", Architecture, View::Section, &[5, 10], "", Template),
                kind("structural-plan", Names::new("constructieve plattegrond", "structural plan", "Tragwerksplan", "plan de structure"), "PLG", Structure, View::Plan, &[50], "01", Template),
                kind("pile-plan", Names::new("palenplan", "pile plan", "Pfahlplan", "plan des pieux"), "PLG", Structure, View::Plan, &[100], "01", Template),
                kind("structural-detail", Names::new("constructief detail", "structural detail", "Tragwerksdetail", "détail de structure"), "details", Structure, View::Section, &[10], "", Template),
                kind("legend", Names::new("renvooi", "legend", "Legende", "légende"), "renvooi", Structure, View::Plan, &[50, 100], "", Template),
                kind("model", Names::new("3D-aanzicht", "3D view", "3D-Ansicht", "vue 3D"), "3D", General, View::Model, &[100], "", Template),
                kind("site-plan", Names::new("situatie", "site plan", "Lageplan", "plan de situation"), "situatie", General, View::Plan, &[500, 1000, 200], "", Practice),
                kind("roof-plan", Names::new("dakplan", "roof plan", "Dachaufsicht", "plan de toiture"), "dakplan", Architecture, View::Plan, &[100, 50], "", Practice),
                kind("ceiling-plan", Names::new("plafondplan", "ceiling plan", "Deckenspiegel", "plan de plafond"), "plafond", Architecture, View::CeilingPlan, &[50, 100], "", Practice),
                kind("fire-plan", Names::new("brandveiligheidsplattegrond", "fire safety plan", "Brandschutzplan", "plan de sécurité incendie"), "brand", Architecture, View::Plan, &[100, 200], "", Practice),
                kind("demolition-plan", Names::new("sloopplattegrond", "demolition plan", "Abbruchplan", "plan de démolition"), "sloop", Architecture, View::Plan, &[100, 50], "", Practice),
                kind("reinforcement", Names::new("wapeningstekening", "reinforcement drawing", "Bewehrungsplan", "plan de ferraillage"), "wapening", Structure, View::Plan, &[50, 20], "", Practice),
                kind("services-plan", Names::new("installatieplattegrond", "services plan", "Installationsplan", "plan des installations"), "PLG", Services, View::Plan, &[50, 100], "", Practice),
                kind("drainage-plan", Names::new("rioleringsplattegrond", "drainage plan", "Entwässerungsplan", "plan d'assainissement"), "riolering", Services, View::Plan, &[100, 50], "", Practice),
            ],
        }
    }

    pub fn kind(&self, key: &str) -> Option<&DrawingKind> {
        self.kinds.iter().find(|k| k.key == key)
    }

    pub fn pen(&self, key: &str) -> Option<&Pen> {
        self.pens.iter().find(|p| p.key == key)
    }

    pub fn line_pattern(&self, key: &str) -> Option<&LinePattern> {
        self.line_patterns.iter().find(|s| s.key == key)
    }

    pub fn line_style(&self, key: &str) -> Option<&LineStyle> {
        self.line_styles.iter().find(|s| s.key == key)
    }

    /// A line of line style `key` as it is drawn at `scale` (1 to this): the width of its pen
    /// there, its pattern's dashes in millimetres and its colour. None when the standard has no
    /// such style.
    pub fn drawn(&self, key: &str, scale: u32) -> Option<Drawn> {
        let style = self.line_style(key)?;
        let width = self.pen(&style.pen)?.width_at(scale);
        Some(Drawn { width, dashes: self.line_pattern(&style.pattern)?.lengths(width), colour: style.colour })
    }

    /// How kind of drawing `kind` draws `line` at `scale` (1 to this): by the line style the
    /// kind gives that line; without one what is cut and what is seen beyond continuous in the
    /// kind's pen for them at that scale, and what is hidden in the kind's line pattern for it,
    /// as wide as what is seen beyond. None where the kind says nothing of the line (a grid
    /// line or a section line without a line style), or names what the standard no longer has.
    pub fn stroke(&self, kind: &DrawingKind, line: Line, scale: u32) -> Option<Drawn> {
        if let Some(style) = kind.lines.get(&line) {
            return self.drawn(style, scale);
        }
        let plain = |pen: &str| self.pen(pen).map(|pen| pen.width_at(scale));
        match line {
            Line::Cut => plain(&kind.cut).map(|width| Drawn { width, dashes: Vec::new(), colour: None }),
            Line::Beyond => plain(&kind.beyond).map(|width| Drawn { width, dashes: Vec::new(), colour: None }),
            Line::Hidden => {
                let width = plain(&kind.beyond)?;
                Some(Drawn { width, dashes: self.line_pattern(&kind.hidden)?.lengths(width), colour: None })
            }
            Line::Grid | Line::Section => None,
        }
    }

    /// The pen whose width is nearest `width` (millimetres on paper): the pen a line weight from
    /// elsewhere (a DXF's, a Revit file's) is drawn with. Of two as near, the narrower.
    pub fn nearest_pen(&self, width: f64) -> Option<&Pen> {
        self.pens.iter().min_by(|a, b| (a.width - width).abs().total_cmp(&(b.width - width).abs()).then(a.width.total_cmp(&b.width)))
    }

    pub fn text_style(&self, key: &str) -> Option<&TextStyle> {
        self.text_styles.iter().find(|t| t.key == key)
    }

    pub fn name(&self, lang: Lang) -> &str {
        self.names.get(lang)
    }

    /// Whether the standard holds together: no key empty or twice, every width, height, dash
    /// and scale positive, and every kind's scales, pens and styles those of the standard.
    pub fn check(&self) -> Result<(), LibraryError> {
        fn keys<'a>(keys: impl Iterator<Item = &'a str>) -> Result<(), LibraryError> {
            let mut seen: Vec<&str> = Vec::new();
            for key in keys {
                if key.trim().is_empty() {
                    return Err(LibraryError::NoKey);
                }
                if seen.contains(&key) {
                    return Err(LibraryError::Duplicate(key.to_string()));
                }
                seen.push(key);
            }
            Ok(())
        }
        let positive = |x: f64| x.is_finite() && x > 0.0;
        keys(std::iter::once(self.key.as_str()))?;
        keys(self.line_patterns.iter().map(|s| s.key.as_str()))?;
        keys(self.pens.iter().map(|p| p.key.as_str()))?;
        keys(self.line_styles.iter().map(|s| s.key.as_str()))?;
        keys(self.text_styles.iter().map(|t| t.key.as_str()))?;
        keys(self.kinds.iter().map(|k| k.key.as_str()))?;
        if self.scales.contains(&0) || !self.pens.iter().all(|p| positive(p.width) && p.at.iter().all(|(scale, width)| *scale > 0 && positive(*width))) || !self.text_styles.iter().all(|t| positive(t.height)) || !self.line_patterns.iter().all(|s| s.dashes.iter().all(|d| positive(*d))) {
            return Err(LibraryError::NotPositive);
        }
        // dashes come with their gaps
        if let Some(style) = self.line_patterns.iter().find(|s| s.dashes.len() % 2 == 1) {
            return Err(LibraryError::UnknownSetting(style.key.clone()));
        }
        // a line style is drawn with a pen and a pattern of the standard
        for style in &self.line_styles {
            if self.pen(&style.pen).is_none() {
                return Err(LibraryError::UnknownSetting(style.pen.clone()));
            }
            if self.line_pattern(&style.pattern).is_none() {
                return Err(LibraryError::UnknownSetting(style.pattern.clone()));
            }
        }
        for kind in &self.kinds {
            if kind.scales.is_empty() {
                return Err(LibraryError::UnknownSetting(format!("{}: scale", kind.key)));
            }
            if let Some(scale) = kind.scales.iter().find(|s| !self.scales.contains(s)) {
                return Err(LibraryError::UnknownSetting(scale_text(*scale)));
            }
            for pen in [&kind.cut, &kind.beyond] {
                if self.pen(pen).is_none() {
                    return Err(LibraryError::UnknownSetting(pen.clone()));
                }
            }
            if self.line_pattern(&kind.hidden).is_none() {
                return Err(LibraryError::UnknownSetting(kind.hidden.clone()));
            }
            if self.text_style(&kind.text).is_none() {
                return Err(LibraryError::UnknownSetting(kind.text.clone()));
            }
            if let Some(style) = kind.lines.values().find(|style| self.line_style(style).is_none()) {
                return Err(LibraryError::UnknownSetting(style.clone()));
            }
        }
        Ok(())
    }
}

/// The path of a node of standard `standard`: its own (`what` "standard", `key` empty) or one of
/// its settings.
pub fn path(standard: &str, what: &str, key: &str) -> String {
    uuid(&format!("openaec-library/drawing/{standard}/{what}/{key}"))
}

/// The nodes of a standard: its own, then one per line pattern, pen, line style, text style and kind, each
/// saying which standard it is of.
pub fn nodes(standard: &Standard) -> Vec<Value> {
    fn setting<T: Serialize>(standard: &Standard, attribute: &str, what: &str, key: &str, name: &str, item: &T) -> Value {
        let mut data = serde_json::to_value(item).unwrap_or(Value::Null);
        if let Value::Object(map) = &mut data {
            map.insert("standard".into(), json!(standard.key));
        }
        json!({"path": path(&standard.key, what, key), "attributes": {"bsi::ifc::prop::Name": name, attribute: data}})
    }
    let mut nodes = vec![json!({"path": path(&standard.key, "standard", ""), "attributes": {"bsi::ifc::prop::Name": standard.name(Lang::En), STANDARD: standard}})];
    nodes.extend(standard.line_patterns.iter().map(|s| setting(standard, LINE_PATTERN, "linestyle", &s.key, s.names.get(Lang::En), s)));
    nodes.extend(standard.pens.iter().map(|p| setting(standard, PEN, "pen", &p.key, p.names.get(Lang::En), p)));
    nodes.extend(standard.line_styles.iter().map(|s| setting(standard, LINE_STYLE, "line", &s.key, s.names.get(Lang::En), s)));
    nodes.extend(standard.text_styles.iter().map(|t| setting(standard, TEXT_STYLE, "textstyle", &t.key, t.names.get(Lang::En), t)));
    nodes.extend(standard.kinds.iter().map(|k| setting(standard, KIND, "kind", &k.key, k.names.get(Lang::En), k)));
    nodes
}

/// A file of drawing standards alone, as [`crate::ifcx::file`] is of a library.
pub fn file(standards: &[Standard], id: &str, author: &str, timestamp: &str) -> Value {
    json!({
        "header": {"id": id, "ifcxVersion": "ifcx_alpha", "dataVersion": "1.0.0", "author": author, "timestamp": timestamp},
        "imports": [],
        "schemas": {},
        "data": standards.iter().flat_map(nodes).collect::<Vec<_>>(),
    })
}

/// The standards in an IFCX file, in the order of their own nodes, each with the settings that
/// name it. The attributes of nodes with the same path are composed, a later one winning, as
/// IFCX layers them; a setting of a standard the file does not hold is left out. Refused when a
/// standard does not hold together ([`Standard::check`]).
pub fn read(file: &Value) -> Result<Vec<Standard>, LibraryError> {
    let data = file.get("data").and_then(Value::as_array).ok_or_else(|| LibraryError::Unreadable("no data".into()))?;
    // compose the nodes by path, keeping the order of first appearance
    let mut order: Vec<&str> = Vec::new();
    let mut composed: std::collections::HashMap<&str, Map<String, Value>> = std::collections::HashMap::new();
    for node in data {
        let (Some(path), Some(attributes)) = (node.get("path").and_then(Value::as_str), node.get("attributes").and_then(Value::as_object)) else { continue };
        let entry = composed.entry(path).or_insert_with(|| {
            order.push(path);
            Map::new()
        });
        for (key, value) in attributes {
            entry.insert(key.clone(), value.clone());
        }
    }
    fn of<T: for<'a> Deserialize<'a>>(attributes: &Map<String, Value>, attribute: &str) -> Result<Option<(String, T)>, LibraryError> {
        let Some(value) = attributes.get(attribute) else { return Ok(None) };
        let standard = value.get("standard").and_then(Value::as_str).unwrap_or_default().to_string();
        serde_json::from_value(value.clone()).map(|item| Some((standard, item))).map_err(|e| LibraryError::Unreadable(format!("{attribute}: {e}")))
    }
    let mut standards: Vec<Standard> = Vec::new();
    for path in &order {
        if let Some(value) = composed[path].get(STANDARD) {
            standards.push(serde_json::from_value(value.clone()).map_err(|e| LibraryError::Unreadable(format!("{STANDARD}: {e}")))?);
        }
    }
    fn find<'a>(standards: &'a mut [Standard], key: &str) -> Option<&'a mut Standard> {
        standards.iter_mut().find(|s| s.key == key)
    }
    for path in &order {
        let attributes = &composed[path];
        if let Some((key, item)) = of::<LinePattern>(attributes, LINE_PATTERN)? {
            if let Some(standard) = find(&mut standards, &key) {
                standard.line_patterns.push(item);
            }
        }
        if let Some((key, item)) = of::<Pen>(attributes, PEN)? {
            if let Some(standard) = find(&mut standards, &key) {
                standard.pens.push(item);
            }
        }
        if let Some((key, item)) = of::<LineStyle>(attributes, LINE_STYLE)? {
            if let Some(standard) = find(&mut standards, &key) {
                standard.line_styles.push(item);
            }
        }
        if let Some((key, item)) = of::<TextStyle>(attributes, TEXT_STYLE)? {
            if let Some(standard) = find(&mut standards, &key) {
                standard.text_styles.push(item);
            }
        }
        if let Some((key, item)) = of::<DrawingKind>(attributes, KIND)? {
            if let Some(standard) = find(&mut standards, &key) {
                standard.kinds.push(item);
            }
        }
    }
    for standard in &standards {
        standard.check()?;
    }
    Ok(standards)
}

/// The nodes of drawing standards among `nodes`, when they read as standards ([`read`]); none
/// when there are none or they do not hold together.
pub fn held(nodes: &[Value]) -> Option<Vec<Value>> {
    let own = [STANDARD, KIND, LINE_PATTERN, PEN, LINE_STYLE, TEXT_STYLE];
    let ours: Vec<Value> = nodes.iter().filter(|node| node.get("attributes").and_then(Value::as_object).is_some_and(|a| own.iter().any(|key| a.contains_key(*key)))).cloned().collect();
    let standards = read(&json!({ "data": ours })).ok()?;
    (!standards.is_empty()).then_some(ours)
}

/// The width a file of AutoCAD linetypes is written and read for when none is said: its lengths
/// are millimetres on paper at a linetype scale of 1, a pattern's are multiples of the line
/// width.
pub const LIN_WIDTH: f64 = 0.25;

/// The name of a linetype for a pattern: its key in capitals, as AutoCAD names its own.
fn lin_name(key: &str) -> String {
    key.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect()
}

/// `patterns` as a file of AutoCAD linetypes (`.lin`), drawn `width` millimetres wide: per
/// pattern its name and what it is (`*NAME,description`) and its dashes, gaps (negative) and
/// dots (0) in millimetres (`A,6,-0.75,0,-0.75`). A continuous line is left out: AutoCAD has
/// its own.
pub fn lin(patterns: &[LinePattern], width: f64) -> String {
    let number = |x: f64| {
        let text = format!("{x:.4}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let mut text = String::from(";; Open BIM Suite line patterns\n");
    for pattern in patterns.iter().filter(|p| !p.dashes.is_empty()) {
        let parts: Vec<String> = pattern.dashes.iter().enumerate().map(|(i, d)| if i % 2 == 1 { format!("-{}", number(d * width)) } else if *d <= DOT { "0".into() } else { number(d * width) }).collect();
        text.push_str(&format!("*{},{}\nA,{}\n", lin_name(&pattern.key), pattern.names.get(Lang::En), parts.join(",")));
    }
    text
}

/// The line patterns of a file of AutoCAD linetypes (`.lin`), for a line `width` millimetres
/// wide: each linetype of dashes, gaps and dots, its name its key in lower case. Left out: a
/// linetype with text or a shape in it (`[...]`), one without a gap, and one whose dashes and
/// gaps do not come in pairs. A linetype that starts with a gap starts with its first dash here.
pub fn from_lin(text: &str, width: f64) -> Vec<LinePattern> {
    let mut patterns: Vec<LinePattern> = Vec::new();
    let mut named: Option<String> = None;
    for line in text.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('*') {
            named = header.split(',').next().map(|name| name.trim().to_string()).filter(|name| !name.is_empty());
            continue;
        }
        let Some(name) = named.take() else { continue };
        let Some(values) = line.strip_prefix("A,").or_else(|| line.strip_prefix("a,")) else { continue };
        if values.contains('[') || !(width.is_finite() && width > 0.0) {
            continue;
        }
        let Ok(values) = values.split(',').map(|v| v.trim().parse::<f64>()).collect::<Result<Vec<f64>, _>>() else { continue };
        // dashes and gaps in line widths, a run of dashes or of gaps taken together
        let mut runs: Vec<(bool, f64)> = Vec::new();
        for value in values {
            let (gap, length) = (value < 0.0, if value == 0.0 { DOT } else { value.abs() / width });
            match runs.last_mut() {
                Some((last, total)) if *last == gap => *total += length,
                _ => runs.push((gap, length)),
            }
        }
        if runs.first().is_some_and(|(gap, _)| *gap) {
            runs.rotate_left(1);
        }
        if runs.len() < 2 || runs.len() % 2 == 1 {
            continue;
        }
        let key: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
        if patterns.iter().any(|p| p.key == key) {
            continue;
        }
        patterns.push(LinePattern { key, names: Names::same(&name), dashes: runs.into_iter().map(|(_, length)| length).collect(), number: None });
    }
    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dutch_standard_holds_together() {
        let nl = Standard::netherlands();
        assert_eq!(nl.check(), Ok(()));
        // the kinds of the template's drawings, with their views, scales and numbers
        let plan = nl.kind("floor-plan").expect("a floor plan");
        assert_eq!((plan.code.as_str(), plan.discipline.code(), plan.view, plan.scale_text().as_str(), plan.number.as_str()), ("PLG", "BWK", View::Plan, "1:50", "01"));
        assert_eq!(nl.kind("pile-plan").map(|k| (k.discipline, k.scales.clone())), Some((Discipline::Structure, vec![100])));
        assert_eq!(nl.kind("elevation").map(|k| (k.view, k.hatched)), Some((View::Elevation, false)));
        assert_eq!(nl.first_sheet, 300);
        // what is cut is twice as wide as what is seen beyond
        for kind in &nl.kinds {
            let (cut, beyond) = (nl.pen(&kind.cut).unwrap().width, nl.pen(&kind.beyond).unwrap().width);
            assert!((cut / beyond - 2.0).abs() < 0.06, "{}: {cut} and {beyond}", kind.key);
        }
        // a kind the template has no drawing of says so
        assert_eq!(nl.kind("site-plan").map(|k| k.origin), Some(Origin::Practice));
        assert!(nl.kinds.iter().filter(|k| k.origin == Origin::Template).count() >= 12);
        // a continuous line has no dashes; the others pair each dash with a gap
        assert!(nl.line_pattern("continuous").unwrap().dashes.is_empty());
        assert_eq!(nl.line_pattern("chain").unwrap().dashes, [24.0, 3.0, 0.5, 3.0]);
        // what is hidden is drawn with a line pattern
        assert!(nl.kinds.iter().all(|k| k.hidden == "dashed"));
    }

    #[test]
    fn a_kind_of_drawing_draws_its_lines_by_a_line_style_or_by_its_pens() {
        let mut nl = Standard::netherlands();
        let plan = nl.kind("floor-plan").unwrap().clone();
        // what is cut and what is seen beyond by the kind's pens, continuous
        assert_eq!(nl.stroke(&plan, Line::Cut, 50), Some(Drawn { width: 0.5, dashes: vec![], colour: None }));
        assert_eq!(nl.stroke(&plan, Line::Beyond, 50), Some(Drawn { width: 0.25, dashes: vec![], colour: None }));
        // what is hidden, grid lines and section lines by their line styles
        assert_eq!(nl.stroke(&plan, Line::Hidden, 50), nl.drawn("hidden", 50));
        assert_eq!(nl.stroke(&plan, Line::Grid, 50).map(|d| d.width), Some(0.18));
        assert_eq!(nl.stroke(&plan, Line::Section, 50), nl.drawn("cutting-plane", 50));
        // a kind without line styles (a file of before): hidden in its pattern, as wide as
        // what is seen beyond; of grid lines and section lines it says nothing
        let old = DrawingKind { lines: Default::default(), ..plan.clone() };
        assert_eq!(nl.stroke(&old, Line::Hidden, 50), Some(Drawn { width: 0.25, dashes: vec![3.0, 0.75], colour: None }));
        assert_eq!((nl.stroke(&old, Line::Grid, 50), nl.stroke(&old, Line::Section, 50)), (None, None));
        // a line style for what is cut wins from the pen, with its colour and at the scale
        nl.line_styles[2].colour = Some([200, 0, 0]);
        nl.pens.iter_mut().find(|p| p.key == "0.5").unwrap().at = [(50, 0.5), (200, 0.35)].into();
        let own = DrawingKind { lines: [(Line::Cut, "wide".to_string())].into(), ..old.clone() };
        assert_eq!(nl.stroke(&own, Line::Cut, 50), Some(Drawn { width: 0.5, dashes: vec![], colour: Some([200, 0, 0]) }));
        assert_eq!(nl.stroke(&own, Line::Cut, 200).map(|d| d.width), Some(0.35));
        // the kind's own pen follows the scale too
        assert_eq!(nl.stroke(&old, Line::Cut, 200).map(|d| d.width), Some(0.35));
        // it is written with the kind and read back; a kind of before reads without
        let text = serde_json::to_string(&own).unwrap();
        assert!(text.contains(r#""lines":{"cut":"wide"}"#), "{text}");
        assert_eq!(serde_json::from_str::<DrawingKind>(&text).unwrap(), own);
        let before = serde_json::to_string(&old).unwrap();
        assert!(!before.contains("lines"));
        assert_eq!(serde_json::from_str::<DrawingKind>(&before).unwrap(), old);
        assert_eq!(Line::ALL.map(|line| Line::of_key(line.key())), Line::ALL.map(Some));
    }

    #[test]
    fn a_standard_is_checked() {
        let nl = Standard::netherlands();
        let with = |change: &dyn Fn(&mut Standard)| {
            let mut s = nl.clone();
            change(&mut s);
            s.check()
        };
        assert_eq!(with(&|s| s.kinds[1].cut = "2.0".into()), Err(LibraryError::UnknownSetting("2.0".into())));
        assert_eq!(with(&|s| s.kinds[1].scales = vec![75]), Err(LibraryError::UnknownSetting("1:75".into())));
        assert_eq!(with(&|s| s.kinds[1].hidden = "wavy".into()), Err(LibraryError::UnknownSetting("wavy".into())));
        assert_eq!(with(&|s| s.pens[0].width = 0.0), Err(LibraryError::NotPositive));
        assert_eq!(with(&|s| s.pens[1].key = s.pens[0].key.clone()), Err(LibraryError::Duplicate("0.13".into())));
        assert_eq!(with(&|s| s.text_styles[0].key.clear()), Err(LibraryError::NoKey));
        assert_eq!(with(&|s| s.line_patterns[1].dashes = vec![12.0]), Err(LibraryError::UnknownSetting("dashed".into())));
        // a line style is drawn with a pen and a pattern the standard has
        assert_eq!(with(&|s| s.line_styles[0].pen = "3".into()), Err(LibraryError::UnknownSetting("3".into())));
        assert_eq!(with(&|s| s.line_styles[0].pattern = "wavy".into()), Err(LibraryError::UnknownSetting("wavy".into())));
        assert_eq!(with(&|s| s.line_styles[1].key = "thin".into()), Err(LibraryError::Duplicate("thin".into())));
        // the pattern of a line style stays while the style is there
        assert_eq!(with(&|s| s.line_patterns.retain(|p| p.key != "long-double-short")), Err(LibraryError::UnknownSetting("long-double-short".into())));
        // a kind draws a line in a line style the standard has, and that style stays meanwhile
        assert_eq!(with(&|s| s.kinds[1].lines = [(Line::Cut, "wavy".to_string())].into()), Err(LibraryError::UnknownSetting("wavy".into())));
        assert_eq!(with(&|s| s.line_styles.retain(|l| l.key != "grid")), Err(LibraryError::UnknownSetting("grid".into())));
        // a pen's width at a scale is a width
        assert_eq!(with(&|s| s.pens[0].at = [(100, 0.0)].into()), Err(LibraryError::NotPositive));
        assert_eq!(with(&|s| s.pens[0].at = [(0, 0.1)].into()), Err(LibraryError::NotPositive));
    }

    #[test]
    fn the_line_patterns_are_those_of_iso_128() {
        let nl = Standard::netherlands();
        // the fifteen line types, numbered in their order
        assert_eq!(nl.line_patterns.iter().map(|p| p.number).collect::<Vec<_>>(), (1..=15).map(Some).collect::<Vec<_>>());
        // each dash with its gap; only dots, short dashes, dashes, long dashes and the two gaps
        for pattern in &nl.line_patterns {
            assert_eq!(pattern.dashes.len() % 2, 0, "{}", pattern.key);
            for (i, d) in pattern.dashes.iter().enumerate() {
                let allowed: &[f64] = if i % 2 == 1 { &[3.0, 18.0] } else { &[DOT, 6.0, 12.0, 24.0] };
                assert!(allowed.contains(d), "{}: {d}", pattern.key);
            }
        }
        // no two alike
        for (i, a) in nl.line_patterns.iter().enumerate() {
            assert!(nl.line_patterns[i + 1..].iter().all(|b| a.dashes != b.dashes), "{}", a.key);
        }
        // in millimetres, drawn 0.25 wide
        assert_eq!(nl.line_pattern("chain").unwrap().lengths(0.25), [6.0, 0.75, 0.125, 0.75]);
    }

    #[test]
    fn a_pen_has_a_width_at_a_scale() {
        let nl = Standard::netherlands();
        // the ISO series, each a step of the square root of two
        assert_eq!(nl.pens.iter().map(|p| p.width).collect::<Vec<_>>(), [0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1.0, 1.4, 2.0]);
        let mut pen = nl.pen("0.5").unwrap().clone();
        // without a width at a scale, its own at every scale
        assert_eq!((pen.width_at(50), pen.width_at(500)), (0.5, 0.5));
        // with some: the one of the scale, else of the nearest by ratio
        pen.at.insert(100, 0.35);
        pen.at.insert(20, 0.7);
        assert_eq!((pen.width_at(100), pen.width_at(200), pen.width_at(75), pen.width_at(10), pen.width_at(1)), (0.35, 0.35, 0.35, 0.7, 0.7));
        // it goes into the file and comes back
        let back: Pen = serde_json::from_value(serde_json::to_value(&pen).unwrap()).unwrap();
        assert_eq!(back, pen);
        assert!(serde_json::to_value(nl.pen("0.5").unwrap()).unwrap().get("at").is_none());
        // a width from elsewhere takes the nearest pen
        assert_eq!(nl.nearest_pen(0.3).map(|p| p.key.as_str()), Some("0.25"));
        assert_eq!(nl.nearest_pen(0.53).map(|p| p.key.as_str()), Some("0.5"));
        assert_eq!(nl.nearest_pen(9.0).map(|p| p.key.as_str()), Some("2"));
    }

    #[test]
    fn a_line_style_is_a_pen_a_pattern_and_a_colour() {
        let mut nl = Standard::netherlands();
        assert_eq!(nl.line_styles.len(), 11);
        // a hidden line: narrow and dashed, 12 and 3 line widths
        let hidden = nl.drawn("hidden", 50).expect("a hidden line");
        assert_eq!(hidden.width, 0.18);
        assert!((hidden.dashes[0] - 2.16).abs() < 1e-12 && (hidden.dashes[1] - 0.54).abs() < 1e-12 && hidden.colour.is_none(), "{hidden:?}");
        assert_eq!(nl.drawn("wide", 50), Some(Drawn { width: 0.5, dashes: vec![], colour: None }));
        // the cutting plane is the centre line's pattern, wide
        assert_eq!((nl.line_style("cutting-plane").unwrap().pattern.as_str(), nl.line_style("centre").unwrap().pattern.as_str()), ("chain", "chain"));
        assert!(nl.drawn("cutting-plane", 50).unwrap().width > nl.drawn("centre", 50).unwrap().width);
        // its pen narrower at a scale: the line and its dashes with it
        nl.pens.iter_mut().find(|p| p.key == "0.18").unwrap().at.insert(200, 0.13);
        let small = nl.drawn("hidden", 200).unwrap();
        assert_eq!(small.width, 0.13);
        assert!((small.dashes[0] - 1.56).abs() < 1e-12);
        // a colour of its own
        nl.line_styles.iter_mut().find(|s| s.key == "demolished").unwrap().colour = Some([255, 200, 0]);
        assert_eq!(nl.drawn("demolished", 100).unwrap().colour, Some([255, 200, 0]));
        assert_eq!(nl.drawn("wavy", 100), None);
        // styles, pens at a scale and colours come back from the file
        assert_eq!(nl.check(), Ok(()));
        assert_eq!(read(&file(&[nl.clone()], "s.ifcx", "test", "2026-11-23T00:00:00Z")), Ok(vec![nl.clone()]));
        // a file from before line styles reads as a standard without them
        let mut old = file(&[nl.clone()], "s.ifcx", "test", "2026-11-23T00:00:00Z");
        old["data"].as_array_mut().unwrap().retain(|node| node["attributes"].get(LINE_STYLE).is_none());
        for node in old["data"].as_array_mut().unwrap() {
            if let Some(kind) = node["attributes"].get_mut(KIND).and_then(|kind| kind.as_object_mut()) {
                kind.remove("lines");
            }
        }
        let read_back = read(&old).unwrap();
        assert!(read_back[0].line_styles.is_empty() && read_back[0].line_patterns.len() == 15);
    }

    #[test]
    fn line_patterns_go_to_and_from_an_autocad_linetype_file() {
        let nl = Standard::netherlands();
        let text = lin(&nl.line_patterns, LIN_WIDTH);
        // a continuous line is AutoCAD's own; the others by name, in millimetres, a dot as 0
        assert!(!text.contains("*CONTINUOUS") && text.contains("*CHAIN,long-dashed dotted\nA,6,-0.75,0,-0.75\n"), "{text}");
        assert!(text.contains("*DASHED_SPACED,dashed spaced\nA,3,-4.5\n"));
        // read back, the dashes are the same
        let back = from_lin(&text, LIN_WIDTH);
        assert_eq!(back.len(), 14);
        for pattern in &back {
            let ours = nl.line_pattern(&pattern.key).unwrap_or_else(|| panic!("{}", pattern.key));
            assert_eq!(pattern.dashes.len(), ours.dashes.len(), "{}", pattern.key);
            assert!(pattern.dashes.iter().zip(&ours.dashes).all(|(a, b)| (a - b).abs() < 1e-9), "{}", pattern.key);
        }
        // linetypes as AutoCAD's own file writes them, in millimetres
        let acad = ";; comment\n*CENTER,Center ____ _ ____ _ ____\nA,31.75,-6.35,6.35,-6.35\n*DOT2,Dot (.5x) .....\nA,0,-3.175\n*GAS_LINE,Gas line ----GAS----\nA,12.7,-5.08,[\"GAS\",STANDARD,S=2.54,U=0.0,X=-2.54,Y=-1.27],-6.35\n*ODD,one dash\nA,5\n*GAPFIRST,gap first\nA,-2,4\n*CENTER,again\nA,1,-1\n";
        let read = from_lin(acad, 0.25);
        assert_eq!(read.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(), ["center", "dot2", "gapfirst"]);
        assert_eq!(read[0].dashes, [127.0, 25.4, 25.4, 25.4]);
        assert_eq!((read[0].names.nl.as_str(), read[0].number), ("CENTER", None));
        assert_eq!(read[1].dashes, [DOT, 12.7]);
        assert_eq!(read[2].dashes, [16.0, 8.0]);
        // read into a standard, they hold together
        let mut with = nl.clone();
        with.line_patterns.extend(read);
        assert_eq!(with.check(), Ok(()));
        assert!(from_lin(acad, 0.0).is_empty());
    }

    #[test]
    fn a_standard_reads_back_from_ifcx_setting_by_setting() {
        let nl = Standard::netherlands();
        let file = file(&[nl.clone()], "standards.ifcx", "test", "2026-11-18T00:00:00Z");
        assert_eq!(file["data"].as_array().map(Vec::len), Some(1 + nl.line_patterns.len() + nl.pens.len() + nl.line_styles.len() + nl.text_styles.len() + nl.kinds.len()));
        assert_eq!(read(&file), Ok(vec![nl.clone()]));
        // written twice, a node has the same path
        assert_eq!(nodes(&nl)[3]["path"], nodes(&nl)[3]["path"]);
        // a later layer changes one setting: the floor plan drawn at 1:100
        let mut layered = file.clone();
        let mut plan = nl.kind("floor-plan").unwrap().clone();
        plan.scales = vec![100];
        let mut change = serde_json::to_value(&plan).unwrap();
        change["standard"] = json!("nl");
        layered["data"].as_array_mut().unwrap().push(json!({"path": path("nl", "kind", "floor-plan"), "attributes": {KIND: change}}));
        let read_back = read(&layered).unwrap();
        assert_eq!(read_back[0].kind("floor-plan").unwrap().scales, [100]);
        assert_eq!(read_back[0].kinds.len(), nl.kinds.len());
        // a setting that does not fit its standard refuses the file
        plan.cut = "9".into();
        let mut bad = serde_json::to_value(&plan).unwrap();
        bad["standard"] = json!("nl");
        layered["data"].as_array_mut().unwrap().push(json!({"path": path("nl", "kind", "floor-plan"), "attributes": {KIND: bad}}));
        assert_eq!(read(&layered), Err(LibraryError::UnknownSetting("9".into())));
        // a file without data is no file of standards
        assert!(matches!(read(&json!({})), Err(LibraryError::Unreadable(_))));
    }
}
