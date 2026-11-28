//! What a click of a tool lands on, each kind switched on or off at the right of the status bar:
//! the ends and corners of elements, the middles of their lines, the grid lines' ends and
//! crossings, the nearest point on a line, straight on from the last point, and whole
//! centimetres.

use crate::app::App;
use bim_core::element::PlanarForm;
use bim_core::{PlanarDef, Shape};
use glam::DVec2;
use openaec_ui::Language;

/// A kind of snap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Snap {
    /// Ends of walls, beams and columns, corners of slabs, where an object stands.
    End,
    /// The middle of a wall's or member's line and of a slab's edge.
    Middle,
    /// The ends of the grid lines and where they cross.
    Grid,
    /// The nearest point on a wall's or member's line, a slab's edge or a grid line.
    Line,
    /// Straight on from the last point clicked, along the axes.
    Ortho,
    Polar,
    /// Whole centimetres.
    Round,
    /// Square only: whatever is drawn runs along x or along y from the last point, whichever is
    /// nearer, wherever the click is. Off: any angle.
    Square,
}

impl Snap {
    pub const ALL: [Snap; 8] = [Snap::End, Snap::Middle, Snap::Grid, Snap::Line, Snap::Ortho, Snap::Polar, Snap::Round, Snap::Square];

    /// Its name on its switch.
    pub fn label(self, l: Language) -> &'static str {
        match self {
            Snap::End => l.text("Eindpunt", "Endpoint"),
            Snap::Middle => l.text("Midden", "Midpoint"),
            Snap::Grid => l.text("Stramien", "Grid"),
            Snap::Line => l.text("Op lijn", "On line"),
            Snap::Ortho => l.text("Ortho", "Ortho"),
            Snap::Polar => l.text("Polair", "Polar"),
            Snap::Round => l.text("Hele cm", "Whole cm"),
            Snap::Square => l.text("Alleen haaks", "Square only"),
        }
    }

    /// What it does, for its tooltip.
    pub fn tip(self, l: Language) -> &'static str {
        match self {
            Snap::End => l.text("Snap op eindpunten en hoeken van elementen", "Snap to the ends and corners of elements"),
            Snap::Middle => l.text("Snap op het midden van wanden, balken en vloerranden", "Snap to the middle of walls, beams and slab edges"),
            Snap::Grid => l.text("Snap op stramienlijnen: hun einden en kruisingen", "Snap to grid lines: their ends and crossings"),
            Snap::Line => l.text("Snap op het dichtstbijzijnde punt van een lijn", "Snap to the nearest point on a line"),
            Snap::Ortho => l.text("Trek recht door vanaf het vorige punt", "Go straight on from the last point"),
            Snap::Polar => l.text("Volg een richting om de 45 graden", "Track directions at 45 degree intervals"),
            Snap::Round => l.text("Rond af op hele centimeters", "Round to whole centimetres"),
            Snap::Square => l.text("Teken alleen horizontaal en verticaal vanaf het vorige punt; uit: elke hoek", "Draw only level and upright from the last point; off: any angle"),
        }
    }
}

/// Which snaps are on. All of them, until switched off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snaps {
    pub end: bool,
    pub middle: bool,
    pub grid: bool,
    pub line: bool,
    pub ortho: bool,
    pub polar: bool,
    pub round: bool,
    /// Square only (see [`Snap::Square`]); off until it is asked for.
    pub square: bool,
}

impl Default for Snaps {
    fn default() -> Self {
        Self { end: true, middle: true, grid: true, line: true, ortho: true, round: true, square: false, polar: false }
    }
}

impl Snaps {
    fn slot(&mut self, snap: Snap) -> &mut bool {
        match snap {
            Snap::End => &mut self.end,
            Snap::Middle => &mut self.middle,
            Snap::Grid => &mut self.grid,
            Snap::Line => &mut self.line,
            Snap::Ortho => &mut self.ortho,
            Snap::Polar => &mut self.polar,
            Snap::Round => &mut self.round,
            Snap::Square => &mut self.square,
        }
    }

    pub fn is(mut self, snap: Snap) -> bool {
        *self.slot(snap)
    }

    pub fn toggle(&mut self, snap: Snap) {
        let slot = self.slot(snap);
        *slot = !*slot;
    }
}

/// `p` brought square to `from`: onto the line along x or along y through `from`, whichever it
/// lies nearer.
pub fn squared(from: DVec2, p: DVec2) -> DVec2 {
    let d = p - from;
    if d.x.abs() >= d.y.abs() {
        DVec2::new(p.x, from.y)
    } else {
        DVec2::new(from.x, p.y)
    }
}

impl App {
    /// The lines a click can land on (in plan): the axes of walls, members and curtain walls, the
    /// edges of slabs and the grid lines.
    pub(in crate::app) fn snap_lines(&self) -> Vec<(DVec2, DVec2)> {
        let grid = self.modeller.model.grid;
        let mut lines = Vec::new();
        for e in self.modeller.model.iter() {
            match e.shape_in_project(&grid) {
                Shape::Linear(l) => lines.push((l.start.truncate(), l.end.truncate())),
                Shape::Planar(PlanarDef { form: PlanarForm::Wall { start, end, .. }, .. }) => lines.push((start, end)),
                Shape::Planar(PlanarDef { form: PlanarForm::Slab { outline, .. }, .. }) => {
                    let n = outline.len();
                    lines.extend((0..n).map(|k| (outline[k], outline[(k + 1) % n])));
                }
                Shape::Planar(PlanarDef { form: PlanarForm::Stair { shape, .. }, .. }) => match shape {
                    bim_core::StairShape::Run { path, .. } => lines.extend(path.windows(2).map(|w| (w[0], w[1]))),
                    bim_core::StairShape::Outline { outline, .. } => {
                        let n = outline.len();
                        lines.extend((0..n).map(|k| (outline[k], outline[(k + 1) % n])));
                    }
                },
                Shape::System(s) => lines.push((s.start, s.end)),
                Shape::Point(_) | Shape::Connection(_) => {}
            }
        }
        lines.extend(self.modeller.model.grids().iter().map(|g| (g.start, g.end)));
        lines.retain(|(a, b)| (*b - *a).length_squared() > 1e-12);
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click, modelling};
    use super::super::ModelTool;
    use super::*;
    use glam::DVec3;

    #[test]
    fn a_floor_plan_looks_straight_down_even_when_empty_and_does_not_turn_and_the_origin_snaps() {
        let mut app = App::default();
        app.modeller.in_modeller = true;
        app.new_model("Level 0");
        // a floor plan of its level, opened: from above, flat, also with nothing in it yet
        let level = app.document.levels()[0].path.clone();
        let plan = app.document.add_plan(&level).expect("a plan");
        let _ = app.update(crate::app::Message::OpenPlan(plan.into()));
        app.finish_animation();
        assert!(app.camera.ortho, "a plan is flat");
        assert!(app.camera.pitch.to_degrees() > 89.0, "from above: {}", app.camera.pitch.to_degrees());
        assert!(app.fixed_view());
        // a drag that would turn it moves it instead
        let (yaw, pitch, target) = (app.camera.yaw, app.camera.pitch, app.camera.target);
        app.orbit_drag(40.0, 25.0);
        assert_eq!((app.camera.yaw, app.camera.pitch), (yaw, pitch));
        assert_ne!(app.camera.target, target);
        // the Grid tool lands on the origin from near it
        app.toggle_grid_tool();
        let (w, h) = (800.0, 600.0);
        let near = DVec3::new(0.04, -0.03, 0.0);
        let (x, y) = app.on_screen(near, w, h).unwrap();
        assert_eq!(app.snapped_as(near, x, y, w, h), (DVec3::ZERO, Some(Snap::Grid)));
    }

    #[test]
    fn with_ortho_on_a_wall_runs_straight_from_its_last_point_whatever_the_click_lands_on() {
        let mut app = modelling();
        // a wall to snap to: its end off the line of the next one
        app.set_model_tool(Some((ModelTool::Wall, "Wall")));
        click(&mut app, -40.0, -30.0);
        click(&mut app, 60.0, -30.0);
        app.set_model_tool(None);
        app.set_model_tool(Some((ModelTool::Wall, "Wall")));
        click(&mut app, 0.0, 0.0);
        let (w, h) = (800.0, 600.0);
        let at = |app: &App, p: DVec3| {
            let (x, y) = app.on_screen(p, w, h).unwrap();
            app.snapped(p, x, y, w, h)
        };
        // 15 degrees off: still along x, not only within a few degrees of it
        assert_eq!(at(&app, DVec3::new(5.0, 1.34, 0.0)), DVec3::new(5.0, 0.0, 0.0));
        // nearer upright: along y
        assert_eq!(at(&app, DVec3::new(1.2, 4.0, 0.0)), DVec3::new(0.0, 4.0, 0.0));
        // the end of the other wall, taken across onto the axis: its x, along x from the start
        let end = at(&app, DVec3::new(60.02, -29.98, 0.0));
        assert_eq!(end, DVec3::new(60.0, 0.0, 0.0), "the end's x along the first axis");
        // Ortho off: any angle; Polar on: the diagonals too, so not held to x or y
        app.modeller.snaps.toggle(Snap::Ortho);
        assert_eq!(at(&app, DVec3::new(5.0, 1.34, 0.0)), DVec3::new(5.0, 1.34, 0.0));
        app.modeller.snaps.toggle(Snap::Ortho);
        app.modeller.snaps.toggle(Snap::Polar);
        assert_eq!(at(&app, DVec3::new(5.0, 1.34, 0.0)), DVec3::new(5.0, 1.34, 0.0));
    }

    #[test]
    fn a_click_lands_on_the_middle_of_a_wall_and_on_its_line_unless_switched_off() {
        let mut app = modelling();
        app.set_model_tool(Some((ModelTool::Wall, "Wall")));
        click(&mut app, -20.0, 0.0);
        click(&mut app, 20.0, 0.0);
        app.set_model_tool(None);
        let (w, h) = (800.0, 600.0);
        let at = |app: &App, p: DVec3| {
            let (x, y) = app.on_screen(p, w, h).unwrap();
            app.snapped(p, x, y, w, h)
        };
        // near the middle: the middle; near the line elsewhere: on the line
        assert_eq!(at(&app, DVec3::new(0.03, 0.04, 0.0)), DVec3::new(0.0, 0.0, 0.0));
        let on = at(&app, DVec3::new(10.513, 0.04, 0.0));
        assert!((on - DVec3::new(10.513, 0.0, 0.0)).length() < 1e-9, "{on}");
        // switched off, the click is where it is, in whole centimetres; and not even those (the
        // grid's off too: the wall's middle is on the origin, which the grid snaps to)
        app.modeller.snaps.toggle(Snap::Middle);
        app.modeller.snaps.toggle(Snap::Line);
        app.modeller.snaps.toggle(Snap::Grid);
        assert_eq!(at(&app, DVec3::new(0.03, 0.04, 0.0)), DVec3::new(0.03, 0.04, 0.0));
        assert_eq!(at(&app, DVec3::new(10.513, 0.044, 0.0)), DVec3::new(10.51, 0.04, 0.0));
        app.modeller.snaps.toggle(Snap::Round);
        assert_eq!(at(&app, DVec3::new(10.513, 0.044, 0.0)), DVec3::new(10.513, 0.044, 0.0));
        // the ends go with their own switch
        assert_eq!(at(&app, DVec3::new(20.02, 0.03, 0.0)), DVec3::new(20.0, 0.0, 0.0));
        app.modeller.snaps.toggle(Snap::End);
        assert_eq!(at(&app, DVec3::new(20.02, 0.03, 0.0)), DVec3::new(20.02, 0.03, 0.0));
        assert!(Snap::ALL.iter().filter(|s| app.modeller.snaps.is(**s)).count() == 1);
        // the cursor's point carries the mark of its snap: the grid, with all snaps on again
        app.modeller.snaps = Snaps::default();
        app.add_grid_line(glam::DVec2::new(5.0, -3.0), glam::DVec2::new(5.0, 9.0)).unwrap();
        app.set_model_tool(Some((ModelTool::Wall, "Wall")));
        let hover = |app: &mut App, p: DVec3| {
            let (x, y) = app.on_screen(p, w, h).unwrap();
            app.model_hover(x, y, w, h);
            (app.modeller.cursor.unwrap(), app.modeller.snap_mark)
        };
        assert_eq!(hover(&mut app, DVec3::new(5.02, 9.03, 0.0)), (DVec3::new(5.0, 9.0, 0.0), Some(Snap::Grid)));
        assert_eq!(hover(&mut app, DVec3::new(-20.02, 0.03, 0.0)), (DVec3::new(-20.0, 0.0, 0.0), Some(Snap::End)));
        assert_eq!(hover(&mut app, DVec3::new(0.02, 0.03, 0.0)), (DVec3::new(0.0, 0.0, 0.0), Some(Snap::Middle)));
        assert_eq!(hover(&mut app, DVec3::new(-10.513, 0.04, 0.0)).1, Some(Snap::Line));
        assert_eq!(hover(&mut app, DVec3::new(-10.513, 7.0, 0.0)).1, None);
    }
}
