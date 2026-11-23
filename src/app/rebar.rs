//! The reinforcement tools of the Modeller: reinforcement drawn with clicks in the model.
//!
//! - **Reinforcement**: a click on a beam, a column, a slab or a wall gives it its usual
//!   reinforcement (a member its bars and stirrups, a slab or a wall its nets).
//! - **Stirrups**: two clicks on a reinforced beam or column give a stretch of it stirrups of
//!   its own, which stand there instead of its usual ones (closer together near a support).
//! - **Extra bars**: two clicks on a beam or a column give a stretch of it more bars along it;
//!   two clicks on a slab draw a line in plan with straight bars square to it.
//! - **Hairpins**: two clicks on a slab draw a line in plan with hairpins along it (a slab's
//!   edge), their legs towards the slab's inside.
//!
//! The first click picks the element (one without reinforcement gets its usual one first) and
//! is where the stretch or the line starts; the second is where it ends. On a member the place
//! clicked is the point of its axis nearest the cursor, rounded to 10 mm; on a slab the point
//! of its top under the cursor, snapped as other clicks are. The tool stays on; Esc drops the
//! first click, then lets the tool go. What is drawn is an extra of the element
//! (`bim_core::reinforcement::Extra`): its sizes are changed in the properties.

use super::App;
use bim_core::element::PlanarForm;
use bim_core::reinforcement::{Extra, LineForm};
use bim_core::{PlanarDef, Shape};
use glam::{DVec2, DVec3, Vec3};
use openaec_ui::Language;

/// Places along a member are rounded to this (metres).
const ROUND: f64 = 0.01;
/// What the tools draw until its sizes are typed (metres): stirrups this far apart, this many
/// bars along a member, and along a line in a slab hairpins and straight bars of these
/// diameters, spacing and lengths.
const STIRRUPS: f64 = 0.1;
const BARS: u32 = 2;
const HAIRPIN: (f64, f64, f64) = (0.008, 0.15, 0.6);
const STRAIGHT: (f64, f64, f64) = (0.012, 0.15, 2.0);

/// A reinforcement tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RebarTool {
    /// The usual reinforcement of what is clicked.
    Usual,
    /// Stirrups over a stretch of a member.
    Stirrups,
    /// More bars over a stretch of a member, or straight bars along a line in a slab.
    Bars,
    /// Hairpins along a line in a slab.
    Hairpins,
}

impl RebarTool {
    pub fn label(self, l: Language) -> &'static str {
        match self {
            RebarTool::Usual => l.text("Wapening", "Reinforcement"),
            RebarTool::Stirrups => l.text("Beugels", "Stirrups"),
            RebarTool::Bars => l.text("Bijleg", "Extra bars"),
            RebarTool::Hairpins => l.text("Haarspelden", "Hairpins"),
        }
    }

    /// What to click next; `started`: the first click is made.
    pub fn hint(self, l: Language, started: bool) -> &'static str {
        match (self, started) {
            (RebarTool::Usual, _) => l.text("Klik een balk, kolom, vloer of wand · Esc: stoppen", "Click a beam, a column, a slab or a wall · Esc: stop"),
            (_, true) => l.text("Klik het einde · Esc: opnieuw", "Click the end · Esc: start again"),
            (RebarTool::Stirrups, false) => l.text("Klik op een balk of kolom waar het traject begint · Esc: stoppen", "Click a beam or a column where the stretch starts · Esc: stop"),
            (RebarTool::Bars, false) => l.text("Klik op een balk, kolom of vloer waar de bijleg begint · Esc: stoppen", "Click a beam, a column or a slab where the extra bars start · Esc: stop"),
            (RebarTool::Hairpins, false) => l.text("Klik op een vloer waar de lijn begint · Esc: stoppen", "Click a slab where the line starts · Esc: stop"),
        }
    }
}

/// How far along the line from `start` to `end` its point nearest the ray from `a` along `d`
/// lies (metres from `start`, within the line). `None` where the ray runs along the line.
fn along_axis(start: DVec3, end: DVec3, a: DVec3, d: DVec3) -> Option<f64> {
    let length = start.distance(end);
    let (u, v) = ((end - start).try_normalize()?, d.try_normalize()?);
    let w = start - a;
    let (b, e, f) = (u.dot(v), u.dot(w), v.dot(w));
    let across = 1.0 - b * b;
    (across > 1e-9).then(|| ((b * f - e) / across).clamp(0.0, length))
}

impl App {
    /// Switches reinforcement tool `tool` on, or off when it is on; every other tool goes off.
    pub(super) fn toggle_rebar_tool(&mut self, tool: RebarTool) {
        let on = self.modeller.rebar_tool != Some(tool);
        self.select_tool();
        self.modeller.rebar_tool = on.then_some(tool);
        self.modeller.refusal = None;
    }

    /// Esc while a reinforcement tool is on: drop the first click, else switch the tool off.
    pub(super) fn escape_rebar_tool(&mut self) {
        self.modeller.refusal = None;
        if self.modeller.rebar_on.take().is_none() {
            self.modeller.rebar_tool = None;
        }
        self.modeller.points.clear();
    }

    /// Where a click at (x, y) lies on the selected element for a stretch or a line: on a
    /// member, how far along its axis (in `x`); on a slab, the point of its top in plan.
    fn rebar_place(&self, x: f32, y: f32, width: f32, height: f32) -> Option<DVec3> {
        let e = self.defined()?;
        match e.shape_in_project(&self.modeller.model.grid) {
            Shape::Linear(l) => {
                let inv = (self.camera.view_proj(width / height) * self.model_matrix()).inverse();
                let (nx, ny) = (2.0 * x / width - 1.0, 1.0 - 2.0 * y / height);
                let a = inv.project_point3(Vec3::new(nx, ny, 0.0)).as_dvec3();
                let d = inv.project_point3(Vec3::new(nx, ny, 1.0)).as_dvec3() - a;
                let s = along_axis(l.start, l.end, a, d)?;
                Some(DVec3::new((s / ROUND).round() * ROUND, 0.0, 0.0))
            }
            Shape::Planar(PlanarDef { form: PlanarForm::Slab { top, .. }, .. }) => {
                let p = self.plane_point(x, y, width, height, top)?;
                Some(self.snapped(p, x, y, width, height))
            }
            _ => None,
        }
    }

    /// A click of a reinforcement tool selected an element. The Reinforcement tool gives it its
    /// usual reinforcement. Another starts its stretch or its line there, on an element it can
    /// draw in (one without reinforcement gets its usual one first).
    pub(super) fn rebar_first(&mut self, tool: RebarTool, x: f32, y: f32, width: f32, height: f32) {
        self.modeller.points.clear();
        self.modeller.rebar_on = None;
        if tool == RebarTool::Usual {
            return self.reinforce_clicked();
        }
        self.modeller.refusal = None;
        let Some(e) = self.defined() else { return };
        let takes = match (&e.shape, tool) {
            (Shape::Linear(_), RebarTool::Stirrups | RebarTool::Bars) => true,
            (Shape::Planar(PlanarDef { form: PlanarForm::Slab { slope: None, .. }, .. }), RebarTool::Bars | RebarTool::Hairpins) => true,
            _ => false,
        };
        if !takes {
            return;
        }
        let id = e.id;
        if e.reinforcement.is_none() && e.nets.is_none() {
            self.reinforce(true);
            if self.modeller.refusal.is_some() {
                return;
            }
            self.select_modelled(id);
        }
        if let Some(p) = self.rebar_place(x, y, width, height) {
            self.modeller.points.push(p);
            self.modeller.rebar_on = Some(id);
        }
    }

    /// The second click of a reinforcement tool: the stretch or the line ends there, and the
    /// element it started on gets it as an extra, with the tool's usual sizes. One the element
    /// cannot be made with is refused and told. The tool stays on for the next.
    pub(super) fn rebar_second(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let (Some(id), Some(tool), Some(&first)) = (self.modeller.rebar_on.take(), self.modeller.rebar_tool, self.modeller.points.first()) else { return };
        self.modeller.points.clear();
        self.select_modelled(id);
        let (Some(e), Some(second)) = (self.defined().cloned(), self.rebar_place(x, y, width, height)) else { return };
        let extra = match (e.shape_in_project(&self.modeller.model.grid), tool, e.reinforcement, e.nets) {
            (Shape::Linear(_), RebarTool::Stirrups, Some(r), _) => Extra::Stirrups { from: first.x.min(second.x), to: first.x.max(second.x), diameter: r.stirrup, spacing: STIRRUPS },
            (Shape::Linear(_), RebarTool::Bars, Some(r), _) => Extra::Bars { from: first.x.min(second.x), to: first.x.max(second.x), count: BARS, diameter: r.bottom.diameter.max(r.stirrup), top: false },
            (Shape::Planar(PlanarDef { form: PlanarForm::Slab { outline, .. }, .. }), RebarTool::Bars | RebarTool::Hairpins, _, Some(_)) if outline.len() > 1 => {
                // in the slab's own frame: from its first corner, along its first edge
                let along = (outline[1] - outline[0]).normalize_or_zero();
                let own = |p: DVec3| DVec2::new((p.truncate() - outline[0]).dot(along), (p.truncate() - outline[0]).dot(along.perp()));
                let (diameter, spacing, length, form) = if tool == RebarTool::Hairpins { (HAIRPIN.0, HAIRPIN.1, HAIRPIN.2, LineForm::Hairpin) } else { (STRAIGHT.0, STRAIGHT.1, STRAIGHT.2, LineForm::Top) };
                Extra::Line { a: own(first), b: own(second), diameter, spacing, length, form }
            }
            _ => return,
        };
        let mut extras = e.extras.clone();
        extras.push(extra);
        self.change_extras(&e, extras);
        self.select_modelled(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::element_props::ElementField;
    use crate::app::Message;
    use crate::viewport::Navigate;
    use bim_core::{LinearDef, ProfileDef};

    /// A new model seen from the top, 10 px per metre, with a beam of 300 × 500 from (0, 0) to
    /// (6, 0) and a slab of 6 × 4 from (0, 2), 250 thick.
    fn model() -> (App, bim_core::ElementId, bim_core::ElementId) {
        let mut app = App::default();
        app.new_model("Level 0");
        let level = app.work_plane().2;
        let beam = Shape::Linear(LinearDef { start: DVec3::new(0.0, 0.0, 3.0), end: DVec3::new(6.0, 0.0, 3.0), profile: ProfileDef::Rectangle { width: 0.3, height: 0.5 }, roll: 0.0 });
        let beam = app.add_element("Beam", level.clone(), beam).unwrap();
        let p = DVec2::new;
        let slab = PlanarForm::Slab { outline: vec![p(0.0, 2.0), p(6.0, 2.0), p(6.0, 6.0), p(0.0, 6.0)], holes: Vec::new(), top: 0.0, bulges: Vec::new(), role: Default::default(), slope: None };
        let slab = app.add_element("Slab", level, Shape::Planar(PlanarDef { form: slab, thickness: 0.25, openings: Vec::new(), build_up: None })).unwrap();
        app.viewport_size = (1000.0, 800.0);
        app.camera.set_preset(crate::viewport::camera::ViewPreset::Top);
        app.camera.ortho = true;
        app.camera.target = Vec3::ZERO;
        app.camera.distance = 40.0 / (app.camera.fov_y * 0.5).tan();
        (app, beam, slab)
    }

    fn click(app: &mut App, at: DVec3) {
        let (x, y) = app.on_screen(at, 1000.0, 800.0).unwrap();
        let _ = app.update(Message::Navigate(Navigate::Pick { x, y, width: 1000.0, height: 800.0, add: false }));
    }

    #[test]
    fn two_clicks_on_a_beam_draw_stirrups_or_bars_over_a_stretch() {
        let (mut app, beam, _) = model();
        let _ = app.update(Message::RebarTool(RebarTool::Stirrups));
        click(&mut app, DVec3::new(0.0, 0.0, 3.25));
        // the beam had no reinforcement: it gets its usual one, and the stretch starts
        assert!(app.modeller.model.get(beam).unwrap().reinforcement.is_some() && app.modeller.rebar_on == Some(beam));
        click(&mut app, DVec3::new(1.5, 0.0, 3.25));
        let e = app.modeller.model.get(beam).unwrap().clone();
        let [Extra::Stirrups { from, to, diameter, spacing }] = e.extras[..] else { panic!("{:?}", e.extras) };
        assert!(from.abs() < 1e-9 && (to - 1.5).abs() < 1e-9 && diameter == 0.008 && spacing == 0.1, "{from} {to}");
        assert!(app.modeller.rebar_tool == Some(RebarTool::Stirrups) && app.modeller.rebar_on.is_none(), "the tool stays on for the next");
        // bars over the middle
        let _ = app.update(Message::RebarTool(RebarTool::Bars));
        click(&mut app, DVec3::new(4.0, 0.0, 3.25));
        click(&mut app, DVec3::new(2.0, 0.0, 3.25));
        let e = app.modeller.model.get(beam).unwrap().clone();
        assert_eq!(e.extras[1], Extra::Bars { from: 2.0, to: 4.0, count: 2, diameter: 0.016, top: false });
        // the schedule lists them: the usual stirrups past the stretch, the stretch, the bars
        let listed = app.schedules.iter().find(|s| s.name == "Bar schedule").unwrap();
        let cells: Vec<Vec<String>> = listed.rows.iter().map(|r| r.cells[1..5].iter().map(|c| c.clone().unwrap_or_default()).collect()).collect();
        assert_eq!(cells[2..], [["3", "stirrup", "8", "30"], ["4", "stirrup", "8", "15"], ["5", "extra", "16", "2"]]);
        // its sizes are typed in the properties; what does not fit is refused and told
        app.select_modelled(beam);
        app.type_element_field(ElementField::ExtraAmount(0), "0,075".into());
        app.type_element_field(ElementField::ExtraAmount(1), "9".into());
        let e = app.modeller.model.get(beam).unwrap().clone();
        assert!(matches!(e.extras[0], Extra::Stirrups { spacing, .. } if spacing == 0.075) && matches!(e.extras[1], Extra::Bars { count: 2, .. }));
        assert_eq!(app.modeller.refusal, Some(bim_core::Refusal::BarsDoNotFit));
        // on top, and taken away again
        let _ = app.update(Message::ExtraForm(1));
        assert!(matches!(app.modeller.model.get(beam).unwrap().extras[1], Extra::Bars { top: true, .. }));
        let _ = app.update(Message::ExtraRemove(0));
        assert_eq!(app.modeller.model.get(beam).unwrap().extras.len(), 1);
        // Esc drops the first click, then lets the tool go
        click(&mut app, DVec3::new(1.0, 0.0, 3.25));
        assert!(app.modeller.rebar_on.is_some());
        let _ = app.update(Message::Escape);
        assert!(app.modeller.rebar_on.is_none() && app.modeller.rebar_tool.is_some());
        let _ = app.update(Message::Escape);
        assert!(app.modeller.rebar_tool.is_none());
    }

    #[test]
    fn two_clicks_on_a_slab_draw_a_line_with_hairpins_shown_in_its_drawing() {
        let (mut app, _, slab) = model();
        let _ = app.update(Message::RebarTool(RebarTool::Hairpins));
        // along its first edge, a little inside it
        click(&mut app, DVec3::new(1.5, 2.0, 0.0));
        click(&mut app, DVec3::new(4.5, 2.0, 0.0));
        let e = app.modeller.model.get(slab).unwrap().clone();
        assert!(e.nets.is_some(), "the slab gets its nets first");
        let [Extra::Line { a, b, diameter, spacing, length, form }] = e.extras[..] else { panic!("{:?} {:?}", e.extras, app.modeller.refusal) };
        assert!((a - DVec2::new(1.5, 0.0)).length() < 0.02 && (b - DVec2::new(4.5, 0.0)).length() < 0.02, "{a} {b}");
        assert_eq!((diameter, spacing, length, form), (0.008, 0.15, 0.6, LineForm::Hairpin));
        // 2.94 of line takes 20 hairpins: a row of the schedule, and in the model three rods each
        let listed = app.schedules.iter().find(|s| s.name == "Bar schedule").unwrap();
        let last: Vec<String> = listed.rows.last().unwrap().cells[1..9].iter().map(|c| c.clone().unwrap_or_default()).collect();
        assert_eq!(last, ["5", "extra", "8", "20", "hairpin", "600", "146", "1346"]);
        // the drawing names them at the line
        app.select_modelled(slab);
        let _ = app.update(Message::ElementDrawing(Language::English));
        let view = app.document.plans().iter().find(|v| v.name.starts_with("Reinforcement drawing")).expect("the drawing");
        assert!(view.notes.iter().any(|n| n.text == "(5) hairpins Ø8-150"), "{:?}", view.notes.iter().map(|n| &n.text).collect::<Vec<_>>());
        // made straight bars on top with the button, longer, and listed so
        app.select_modelled(slab);
        let _ = app.update(Message::ExtraForm(0));
        app.type_element_field(ElementField::ExtraLength(0), "1,2".into());
        assert!(matches!(app.modeller.model.get(slab).unwrap().extras[0], Extra::Line { form: LineForm::Top, length, .. } if length == 1.2));
        // the stirrups tool draws nothing in a slab
        let _ = app.update(Message::RebarTool(RebarTool::Stirrups));
        click(&mut app, DVec3::new(3.0, 4.0, 0.0));
        assert!(app.modeller.rebar_on.is_none());
    }
}
