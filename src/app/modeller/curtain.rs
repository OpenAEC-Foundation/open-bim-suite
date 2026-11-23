//! The grid of a curtain wall, edited as the reference modeller edits it.
//!
//! - **Grid line** (the tool): a click on a curtain wall puts a line there. Near its foot, its
//!   head or a horizontal line the line stands upright through the click; near an end or an
//!   upright line it lies level through it. Seen from above, where a click says no height, it
//!   stands upright. Shift and a click takes the nearest line away.
//!   A drag that starts on a line moves it to where the button is let go.
//! - **Panel** (the tool): a click on a bay gives it the next of glass, a closed panel, a door
//!   and nothing at all.
//! - A curtain wall whose grid was touched keeps its lines as its own (see `bim_core::OwnGrid`);
//!   its properties list them, each to be typed where it stands or removed, and "Equal bays"
//!   gives it back the equal grid of its spacing.
//!
//! Each change is one step to take back; what the kernel refuses (a line closer to the next than
//! a member is wide) is told and leaves the wall as it was.

use crate::app::App;
use bim_core::{BayPanel, ElementId, OwnGrid, Shape, SystemDef};
use glam::{DVec3, Vec3};
use openaec_ui::Language;

/// A tool on the grid of a curtain wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurtainTool {
    /// Adds a grid line where it is clicked; with Shift takes the nearest away.
    Line,
    /// Steps the bay clicked through glass, closed and open.
    Panel,
}

impl CurtainTool {
    pub const ALL: [CurtainTool; 2] = [CurtainTool::Line, CurtainTool::Panel];

    pub fn id(self) -> &'static str {
        match self {
            CurtainTool::Line => "model.curtain-line",
            CurtainTool::Panel => "model.curtain-panel",
        }
    }

    pub fn label(self, l: Language) -> &'static str {
        match self {
            CurtainTool::Line => l.text("Rasterlijn", "Grid line"),
            CurtainTool::Panel => l.text("Paneel", "Panel"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            CurtainTool::Line => "curtain.new",
            CurtainTool::Panel => "draw.filled-area",
        }
    }

    pub fn hint(self, l: Language) -> &'static str {
        match self {
            CurtainTool::Line => l.text(
                "Rasterlijn: klik op een vliesgevel (bij de voet of de kop: staand, bij een eind: liggend) · sleep een lijn om hem te verplaatsen · Shift + klik: lijn weghalen · Esc: stoppen",
                "Grid line: click a curtain wall (near its foot or head: upright, near an end: level) · drag a line to move it · Shift + click: take a line away · Esc: stop",
            ),
            CurtainTool::Panel => l.text("Paneel: klik een vak · glas, dicht, deur, open · Esc: stoppen", "Panel: click a bay · glass, closed, door, open · Esc: stop"),
        }
    }
}

/// A click is rounded to this (metres).
const ROUND: f64 = 0.01;
/// Shift and a click takes a line this near away (metres).
const TAKE: f64 = 0.3;
/// Seen from above, a click this near a curtain wall's line is on it (metres).
const BESIDE: f64 = 0.4;

/// The grid of `s` as lines of its own: its own, else the equal one of its spacing.
pub fn own_grid(s: &SystemDef) -> OwnGrid {
    s.grid.clone().unwrap_or_else(|| OwnGrid::equal((s.end - s.start).length(), s.height, s.spacing))
}

/// Whether `s` is a curtain wall whose grid can be its own: upright, with members on a stacked
/// grid (not a timber or steel frame, not a bond).
pub fn has_grid(s: &SystemDef) -> bool {
    !s.lies() && s.system_type.as_ref().is_none_or(|t| t.framing.is_none() && t.pattern == bim_library::system::Pattern::Stack)
}

/// Puts a line in `lines` at `at`, in order; the panels of the bays from there on count one on.
fn insert(lines: &mut Vec<f64>, at: f64) -> usize {
    let k = lines.iter().position(|u| *u > at).unwrap_or(lines.len());
    lines.insert(k, at);
    k
}

impl App {
    /// Switches a tool on the grid of curtain walls on, or off when it is on.
    pub(in crate::app) fn toggle_curtain_tool(&mut self, tool: CurtainTool) {
        let on = self.modeller.curtain == Some(tool);
        self.select_tool();
        self.modeller.curtain = (!on).then_some(tool);
        self.modeller.refusal = None;
    }

    /// The banner while a curtain tool is on.
    pub(in crate::app) fn curtain_hint(&self, l: Language) -> Option<String> {
        let tool = self.modeller.curtain?;
        Some(match self.modeller.refusal {
            Some(refusal) => format!("{} · {}", l.translate(refusal.reason()), tool.hint(l)),
            None => tool.hint(l).to_string(),
        })
    }

    /// The curtain wall under viewport point (x, y), and where on its face that is: along from
    /// its start and, when the view is not from above, up from its base. The nearest to the eye.
    fn curtain_under(&self, x: f32, y: f32, width: f32, height: f32) -> Option<(ElementId, f64, Option<f64>)> {
        let inv = (self.camera.view_proj(width / height) * self.model_matrix()).inverse();
        let (nx, ny) = (2.0 * x / width - 1.0, 1.0 - 2.0 * y / height);
        let (a, b) = (inv.project_point3(Vec3::new(nx, ny, 0.0)).as_dvec3(), inv.project_point3(Vec3::new(nx, ny, 1.0)).as_dvec3());
        let ray = (b - a).try_normalize()?;
        let grid = self.modeller.model.grid;
        let mut best: Option<(f64, ElementId, f64, Option<f64>)> = None;
        for e in self.modeller.model.iter() {
            let Shape::System(s) = e.shape_in_project(&grid) else { continue };
            if !has_grid(&s) {
                continue;
            }
            let length = (s.end - s.start).length();
            let Some(dir) = (s.end - s.start).try_normalize() else { continue };
            let normal = DVec3::new(-dir.y, dir.x, 0.0);
            let origin = s.start.extend(s.base);
            let facing = ray.dot(normal);
            if facing.abs() > 0.05 {
                // the ray meets the wall's face
                let t = (origin - a).dot(normal) / facing;
                let p = a + ray * t;
                let (u, v) = ((p - origin).truncate().dot(dir), p.z - s.base);
                if t > 0.0 && (0.0..=length).contains(&u) && (0.0..=s.height).contains(&v) && best.as_ref().is_none_or(|(least, ..)| t < *least) {
                    best = Some((t, e.id, u, Some(v)));
                }
            } else if ray.z.abs() > 0.5 {
                // seen from above: the point of the floor under the cursor, beside its line
                let t = (s.base - a.z) / ray.z;
                let p = (a + ray * t).truncate() - s.start;
                let (u, off) = (p.dot(dir), p.perp_dot(dir).abs());
                if (0.0..=length).contains(&u) && off <= BESIDE && best.as_ref().is_none_or(|(least, ..)| off < *least) {
                    best = Some((off, e.id, u, None));
                }
            }
        }
        best.map(|(_, id, u, v)| (id, u, v))
    }

    /// A click while a curtain tool is on. `take`: Shift is held.
    pub(in crate::app) fn curtain_click(&mut self, x: f32, y: f32, width: f32, height: f32, take: bool) {
        let Some(tool) = self.modeller.curtain else { return };
        self.modeller.refusal = None;
        let Some((id, u, v)) = self.curtain_under(x, y, width, height) else { return };
        let Some(e) = self.modeller.model.get(id).cloned() else { return };
        let mut shape = e.shape_in_project(&self.modeller.model.grid);
        let Shape::System(s) = &mut shape else { return };
        let (length, high) = ((s.end - s.start).length(), s.height);
        let mut own = own_grid(s);
        let (us, vs) = own.stops(length, high);
        let near = |stops: &[f64], at: f64| stops.iter().map(|q| (q - at).abs()).fold(f64::INFINITY, f64::min);
        let round = |q: f64| (q / ROUND).round() * ROUND;
        match tool {
            CurtainTool::Line if take => {
                // the nearest line of its own (not an edge), upright or level
                let nearest = |lines: &[f64], at: f64| lines.iter().enumerate().map(|(k, q)| (k, (q - at).abs())).min_by(|a, b| a.1.total_cmp(&b.1));
                let upright = nearest(&own.along, u);
                let level = v.and_then(|v| nearest(&own.up, v));
                match (upright, level) {
                    (Some((k, d)), level) if d <= TAKE && level.is_none_or(|(_, e)| d <= e) => {
                        own.along.remove(k);
                        // the bay to its right joins the one to its left
                        own.panels.retain(|(c, ..)| *c != k + 1);
                        own.panels.iter_mut().filter(|(c, ..)| *c > k + 1).for_each(|(c, ..)| *c -= 1);
                    }
                    (_, Some((k, d))) if d <= TAKE => {
                        own.up.remove(k);
                        own.panels.retain(|(_, r, _)| *r != k + 1);
                        own.panels.iter_mut().filter(|(_, r, _)| *r > k + 1).for_each(|(_, r, _)| *r -= 1);
                    }
                    _ => return,
                }
            }
            CurtainTool::Line => {
                // near a level line or edge it stands upright through the click, else it lies level
                let upright = v.is_none_or(|v| near(&vs, v) <= near(&us, u));
                if upright {
                    let k = insert(&mut own.along, round(u));
                    own.panels.iter_mut().filter(|(c, ..)| *c > k).for_each(|(c, ..)| *c += 1);
                } else if let Some(v) = v {
                    let k = insert(&mut own.up, round(v));
                    own.panels.iter_mut().filter(|(_, r, _)| *r > k).for_each(|(_, r, _)| *r += 1);
                }
            }
            CurtainTool::Panel => {
                let Some(v) = v else { return };
                let cell = |stops: &[f64], at: f64| stops.windows(2).position(|p| at >= p[0] && at <= p[1]);
                let (Some(column), Some(row)) = (cell(&us, u), cell(&vs, v)) else { return };
                let next = match own.panel(column, row) {
                    None => Some(BayPanel::Solid),
                    Some(BayPanel::Solid) => Some(BayPanel::Door),
                    Some(BayPanel::Door) => Some(BayPanel::Open),
                    Some(BayPanel::Open) => None,
                };
                own.panels.retain(|(c, r, _)| (*c, *r) != (column, row));
                own.panels.extend(next.map(|p| (column, row, p)));
            }
        }
        s.grid = Some(own);
        self.change_shape(&e, shape);
    }

    /// A drag in the viewport from `from` to `to` (pixels) while the Grid line tool is on, the
    /// button let go (`done`): the line of a curtain wall the drag began on goes to where it
    /// ended, on that wall. False when the tool is off.
    pub(in crate::app) fn curtain_drag(&mut self, from: (f32, f32), to: (f32, f32), width: f32, height: f32, done: bool) -> bool {
        if self.modeller.curtain != Some(CurtainTool::Line) {
            return false;
        }
        if !done {
            return true;
        }
        let (width, height) = (width.max(1.0), height.max(1.0));
        self.modeller.refusal = None;
        let (Some((id, u0, v0)), Some((there, u1, v1))) = (self.curtain_under(from.0, from.1, width, height), self.curtain_under(to.0, to.1, width, height)) else { return true };
        if id != there {
            return true;
        }
        let Some(e) = self.modeller.model.get(id).cloned() else { return true };
        let mut shape = e.shape_in_project(&self.modeller.model.grid);
        let Shape::System(s) = &mut shape else { return true };
        let mut own = own_grid(s);
        let nearest = |lines: &[f64], at: f64| lines.iter().enumerate().map(|(k, q)| (k, (q - at).abs())).min_by(|a, b| a.1.total_cmp(&b.1));
        let round = |q: f64| (q / ROUND).round() * ROUND;
        let upright = nearest(&own.along, u0);
        let level = v0.and_then(|v| nearest(&own.up, v));
        // the line the drag began on: the nearer of the upright and the level one, within reach;
        // it keeps its place among the others (one dragged past its neighbour is refused)
        match (upright, level, v1) {
            (Some((k, d)), level, _) if d <= TAKE && level.is_none_or(|(_, e)| d <= e) => own.along[k] = round(u1),
            (_, Some((k, d)), Some(v1)) if d <= TAKE => own.up[k] = round(v1),
            _ => return true,
        }
        s.grid = Some(own);
        self.change_shape(&e, shape);
        true
    }

    /// Removes grid line `index` of the selected curtain wall: an upright one, or a level one.
    pub(in crate::app) fn remove_grid_line(&mut self, upright: bool, index: usize) {
        let Some(e) = self.defined().cloned() else { return };
        let mut shape = e.shape_in_project(&self.modeller.model.grid);
        let Shape::System(s) = &mut shape else { return };
        let Some(own) = s.grid.as_mut() else { return };
        let lines = if upright { &mut own.along } else { &mut own.up };
        if index >= lines.len() {
            return;
        }
        lines.remove(index);
        if upright {
            own.panels.retain(|(c, ..)| *c != index + 1);
            own.panels.iter_mut().filter(|(c, ..)| *c > index + 1).for_each(|(c, ..)| *c -= 1);
        } else {
            own.panels.retain(|(_, r, _)| *r != index + 1);
            own.panels.iter_mut().filter(|(_, r, _)| *r > index + 1).for_each(|(_, r, _)| *r -= 1);
        }
        self.modeller.refusal = None;
        self.change_shape(&e, shape);
    }

    /// The selected curtain wall gets the equal grid of its spacing back: its own lines and
    /// panels go.
    pub(in crate::app) fn equal_grid(&mut self) {
        let Some(e) = self.defined().cloned() else { return };
        let mut shape = e.shape_in_project(&self.modeller.model.grid);
        let Shape::System(s) = &mut shape else { return };
        s.grid = None;
        self.modeller.refusal = None;
        self.change_shape(&e, shape);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click, modelling};
    use super::super::ModelTool;
    use super::*;
    use crate::app::Message;
    use crate::viewport::Navigate;

    /// A curtain wall of 6 m along x, 3 m high, in bays of 1.5, and its id.
    fn wall() -> (App, ElementId) {
        let mut app = modelling();
        app.modeller.in_modeller = true;
        app.set_model_tool(Some((ModelTool::System, "Curtain wall")));
        app.place_message(crate::app::placement::PlaceMessage::Field(crate::app::placement::PlaceField::Height, "3".into()));
        click(&mut app, 0.0, 0.0);
        click(&mut app, 6.0, 0.0);
        app.set_model_tool(None);
        app.modeller.points.clear();
        let id = app.modeller.model.iter().next().unwrap().id;
        (app, id)
    }

    fn system(app: &App, id: ElementId) -> SystemDef {
        let Shape::System(s) = app.modeller.model.get(id).unwrap().shape_in_project(&app.modeller.model.grid) else { panic!() };
        s
    }

    /// A click on the wall's face at `u` along and `v` up, seen from the front.
    fn face_click(app: &mut App, u: f64, v: f64, add: bool) {
        let (sx, sy) = app.on_screen(DVec3::new(u, 0.0, v), 1000.0, 800.0).unwrap();
        let _ = app.update(Message::Navigate(Navigate::Pick { x: sx, y: sy, width: 1000.0, height: 800.0, add }));
    }

    #[test]
    fn grid_lines_are_added_taken_away_and_typed_and_a_bay_gets_a_panel_of_its_own() {
        let (mut app, id) = wall();
        assert_eq!(system(&app, id).grid, None);
        // from above a click on the wall adds an upright line there
        let _ = app.update(Message::CurtainTool(CurtainTool::Line));
        click(&mut app, 2.2, 0.1);
        assert_eq!(system(&app, id).grid.unwrap().along, [1.5, 2.2, 3.0, 4.5]);
        // from the front: near the foot an upright line, in the middle of a bay near its side a level one
        app.camera.set_preset(crate::viewport::camera::ViewPreset::Front);
        face_click(&mut app, 5.2, 0.1, false);
        face_click(&mut app, 0.1, 2.0, false);
        let own = system(&app, id).grid.unwrap();
        assert_eq!((own.along.clone(), own.up.clone()), (vec![1.5, 2.2, 3.0, 4.5, 5.2], vec![1.5, 2.0]));
        // Shift and a click takes the line nearest away
        face_click(&mut app, 3.05, 0.6, true);
        assert_eq!(system(&app, id).grid.unwrap().along, [1.5, 2.2, 4.5, 5.2]);
        // the Panel tool steps a bay through closed, open and glass again
        let plates = |app: &App| app.modeller.model.evaluate(id).unwrap().iter().filter(|p| matches!(p.class, "IfcPlate" | "IfcCovering" | "IfcDoor")).count();
        let all = plates(&app);
        let _ = app.update(Message::CurtainTool(CurtainTool::Panel));
        face_click(&mut app, 0.7, 0.7, false);
        assert_eq!((system(&app, id).grid.unwrap().panels, plates(&app)), (vec![(0, 0, BayPanel::Solid)], all));
        face_click(&mut app, 0.7, 0.7, false);
        assert_eq!(system(&app, id).grid.unwrap().panels, vec![(0, 0, BayPanel::Door)]);
        assert_eq!(app.modeller.model.evaluate(id).unwrap().iter().filter(|p| p.class == "IfcDoor").count(), 1);
        face_click(&mut app, 0.7, 0.7, false);
        assert_eq!((system(&app, id).grid.unwrap().panels, plates(&app)), (vec![(0, 0, BayPanel::Open)], all - 1));
        face_click(&mut app, 0.7, 0.7, false);
        assert_eq!(plates(&app), all);
        // a drag with the Grid line tool moves the line it began on: the one at 4.5 to 4.0
        let _ = app.update(Message::CurtainTool(CurtainTool::Line));
        let (a, b) = (app.on_screen(DVec3::new(4.52, 0.0, 0.8), 1000.0, 800.0).unwrap(), app.on_screen(DVec3::new(4.0, 0.0, 0.9), 1000.0, 800.0).unwrap());
        for done in [false, true] {
            let _ = app.update(Message::Navigate(Navigate::Box { x0: a.0, y0: a.1, x1: b.0, y1: b.1, width: 1000.0, height: 800.0, done, add: false }));
        }
        assert_eq!(system(&app, id).grid.unwrap().along, [1.5, 2.2, 4.0, 5.2]);
        assert!(app.box_select.is_none());
        let _ = app.update(Message::CurtainTool(CurtainTool::Line));
        // a line is typed where it stands and removed in the properties; one step back each
        let _ = app.update(Message::CurtainTool(CurtainTool::Panel));
        app.select_modelled(id);
        let _ = app.update(Message::ElementField(crate::app::element_props::ElementField::GridAlong(1), "2.4".into()));
        assert_eq!(system(&app, id).grid.unwrap().along, [1.5, 2.4, 4.0, 5.2]);
        let _ = app.update(Message::RemoveGridLine(false, 0));
        assert_eq!(system(&app, id).grid.unwrap().up, [2.0]);
        let _ = app.update(Message::Undo);
        assert_eq!(system(&app, id).grid.unwrap().up, [1.5, 2.0]);
        app.select_modelled(id);
        // a line closer to the next than a member is wide is refused, and the wall stays
        let _ = app.update(Message::ElementField(crate::app::element_props::ElementField::GridAlong(1), "1.52".into()));
        assert_eq!(system(&app, id).grid.unwrap().along, [1.5, 2.4, 4.0, 5.2]);
        assert!(app.modeller.refusal.is_some(), "and it is told why");
        // Equal bays gives the grid of its spacing back
        let _ = app.update(Message::EqualGrid);
        assert_eq!(system(&app, id).grid, None);
    }
}
