//! Grid lines in the Modeller: the Grid tool, and what follows the grid lines of the modelled
//! project.
//!
//! - **The tool.** Two clicks on the work plane make a grid line; the second is put in line with
//!   the first when nearly so, as a wall's is. The line names itself (a number when it runs
//!   north-south, else a letter: see `bim_core::datum`), is picked, and the tool goes off.
//! - **Where they show.** The grid lines are part of what the modelled project shows, so the
//!   viewport draws them as it draws the grid lines of an opened file: in 3D at the lowest level,
//!   in a floor plan over its cut, in a section where it crosses them.
//! - **Snapping.** A click of any modelling tool snaps to the ends of the grid lines and to where
//!   they cross.
//! - **Picking.** A click picks a grid line; Shift and a click adds one or lets it go, and a
//!   selection box takes those in it (drawn to the left: those it touches). Picked grid lines
//!   show in light blue.
//! - **Moving.** A drag that starts on a picked grid line moves the picked ones with the cursor;
//!   Move, Copy, Rotate and Mirror take them as they take elements. Rotate has a field for the
//!   angle: Enter turns them by it about the centre clicked, else about their own middle.
//! - **Removing.** Delete (or Delete in the menu of the right button) removes the picked grid
//!   lines.
//! - **A series.** A picked grid line has a plus past its end: it opens a field for the
//!   spacings of the lines to add beside it, in millimetres ("5000 3000 3x5400": 5 m on, 3 m
//!   further, then three times 5.4 m). The new lines are parallel copies with the next free
//!   names: to the right of a line that runs up the plan, above one that runs across it. The line
//!   stays picked, so more can be added from it.
//!
//! They are kept in the kernel's model beside the elements, so undo takes a grid line back as it
//! takes back an element, and the project's IFCX file holds them.

use crate::app::placing::PlacingOverlay;
use crate::app::App;
use crate::model::GridLine;
use glam::{DVec2, DVec3};
use iced::Color;
use openaec_ui::Language;

/// The plus of a picked grid line: its size, and how far past the line's end it stands (pixels).
const PLUS: f32 = 24.0;
const PLUS_PAST: f32 = 56.0;
/// How wide the series field's card is.
const CARD: f32 = 240.0;

/// A drag of grid lines is rounded to this (metres).
const DRAG_ROUND: f64 = 0.01;

/// The picked grid lines being dragged: which (of the modelled project), the grid lines as they
/// were when the drag began, and where on the work plane it began.
#[derive(Debug, Clone, PartialEq)]
pub struct GridDrag {
    indices: Vec<usize>,
    before: Vec<bim_core::GridDef>,
    origin: DVec2,
}

/// A message of the series field of the picked grid line.
#[derive(Debug, Clone, PartialEq)]
pub enum SeriesMessage {
    /// The plus: opens the field, or closes it.
    Toggle,
    Typed(String),
    /// Enter or Create: the lines are added.
    Create,
}

/// The spacings a series is typed as, in metres: millimetres apart, `NxD` for N times D.
/// `None` when the text is no such list, or empty.
pub fn spacings(typed: &str) -> Option<Vec<f64>> {
    let mut out = Vec::new();
    for word in typed.split_whitespace() {
        let (times, each) = match word.to_ascii_lowercase().split_once('x') {
            Some((n, d)) => (n.parse::<usize>().ok().filter(|n| (1..=200).contains(n))?, d.to_string()),
            None => (1, word.to_string()),
        };
        let each = each.replace(',', ".").parse::<f64>().ok().filter(|d| d.is_finite() && *d > 0.0)? / 1000.0;
        out.extend(std::iter::repeat_n(each, times));
    }
    (!out.is_empty() && out.len() <= 500).then_some(out)
}

impl App {
    /// A message of the series field.
    pub(in crate::app) fn series_message(&mut self, message: SeriesMessage) {
        match message {
            SeriesMessage::Toggle => self.modeller.grid_series = if self.modeller.grid_series.is_some() { None } else { Some(String::new()) },
            SeriesMessage::Typed(text) => self.modeller.grid_series = Some(text),
            SeriesMessage::Create => {
                let Some(apart) = self.modeller.grid_series.as_deref().and_then(spacings) else { return };
                if self.add_grid_series(&apart) {
                    self.modeller.grid_series = None;
                }
            }
        }
    }

    /// Adds grid lines beside the picked one, each `apart` further than the one before: to the
    /// right of a line that runs up the plan, above one that runs across it. The picked line
    /// stays picked. False when no modelled grid line is picked.
    pub(in crate::app) fn add_grid_series(&mut self, apart: &[f64]) -> bool {
        let Some(index) = self.picked_modelled_grid() else { return false };
        let Some(line) = self.modeller.model.grids().get(index).cloned() else { return false };
        let along = (line.end - line.start).normalize_or_zero();
        // square to the line, towards the right, or upwards for a line that runs across
        let mut side = along.perp();
        if side.x < -1e-9 || (side.x.abs() <= 1e-9 && side.y < 0.0) {
            side = -side;
        }
        let mut offset = 0.0;
        for d in apart {
            offset += d;
            let _ = self.modeller.model.add_grid(line.start + side * offset, line.end + side * offset);
        }
        self.show_modelled();
        let shown = self.shown_grid(index);
        self.pick_grid(shown);
        true
    }

    /// The modelled grid lines `indices` go through `map`: moved, turned or mirrored, or (`copy`)
    /// left where they are with a line made there under the next name of its kind. The lines
    /// moved or made are picked, so they can be moved or copied on.
    pub(in crate::app) fn map_grids(&mut self, indices: &[usize], map: &crate::app::transform::Map, copy: bool) {
        if indices.is_empty() {
            return;
        }
        let mut now = Vec::new();
        for &index in indices {
            let Some(line) = self.modeller.model.grids().get(index).cloned() else { continue };
            let (start, end) = (map.plan(line.start), map.plan(line.end));
            let made = if copy {
                self.modeller.model.add_grid(start, end).ok()
            } else {
                self.modeller.model.set_grid(index, bim_core::GridDef { start, end, ..line }).ok().map(|_| index)
            };
            now.extend(made);
        }
        self.show_modelled();
        self.pick_modelled_grids(&now);
    }

    /// Picks the modelled grid lines `indices`, beside what is selected.
    fn pick_modelled_grids(&mut self, indices: &[usize]) {
        let shown: Vec<usize> = indices.iter().filter_map(|&i| self.shown_grid(i)).collect();
        self.set_picked_grids(shown);
    }

    /// The grid lines a selection box takes (indices among the grid lines shown): drawn to the
    /// right those that lie in it whole, drawn to the left those it touches too, as with elements.
    pub(in crate::app) fn grids_in_box(&self, rect: (glam::Vec2, glam::Vec2), width: f32, height: f32) -> Vec<usize> {
        if !self.show_grids {
            return Vec::new();
        }
        let (width, height) = (width.max(1.0), height.max(1.0));
        let view_proj = self.camera.view_proj(width / height);
        let (lo, hi) = (rect.0.min(rect.1), rect.0.max(rect.1));
        let touching = rect.1.x < rect.0.x;
        let inside = |p: glam::Vec2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
        let on_screen = |q: [f32; 3]| {
            let c = view_proj.project_point3(glam::Vec3::from_array(q));
            (0.0..=1.0).contains(&c.z).then(|| glam::Vec2::new((c.x + 1.0) / 2.0 * width, (1.0 - c.y) / 2.0 * height))
        };
        // whether segment a-b crosses segment c-d
        let crosses = |a: glam::Vec2, b: glam::Vec2, c: glam::Vec2, d: glam::Vec2| {
            let side = |p: glam::Vec2, q: glam::Vec2, r: glam::Vec2| (q - p).perp_dot(r - p);
            side(a, b, c) * side(a, b, d) <= 0.0 && side(c, d, a) * side(c, d, b) <= 0.0
        };
        let corners = [lo, glam::Vec2::new(hi.x, lo.y), hi, glam::Vec2::new(lo.x, hi.y)];
        let mut taken = Vec::new();
        for datum in self.datums() {
            let crate::annotations::DatumKind::Grid(g) = datum.kind else { continue };
            let (Some(a), Some(b)) = (on_screen(datum.line.a), on_screen(datum.line.b)) else { continue };
            let whole = inside(a) && inside(b);
            let touched = whole || inside(a) || inside(b) || (0..4).any(|k| crosses(a, b, corners[k], corners[(k + 1) % 4]));
            if (whole || (touching && touched)) && !taken.contains(&g) {
                taken.push(g);
            }
        }
        taken
    }

    /// A drag in the viewport from `from` to `to` (pixels), `done` when the button is let go: when
    /// it began on a picked grid line of the modelled project, the picked grid lines follow the
    /// cursor over the work plane, rounded to 10 mm, and it is one step to take back. False when
    /// it is no such drag (the drag then draws a selection box).
    pub(in crate::app) fn drag_grids(&mut self, from: (f32, f32), to: (f32, f32), width: f32, height: f32, done: bool) -> bool {
        let (width, height) = (width.max(1.0), height.max(1.0));
        let (z, _, _) = self.work_plane();
        if self.modeller.grid_drag.is_none() {
            if !self.drag_selects() {
                return false;
            }
            let Some(under) = self.grid_under(from.0, from.1, width, height, None) else { return false };
            let indices = self.picked_modelled_grids();
            if !self.grids_picked().contains(&under) || indices.is_empty() {
                return false;
            }
            let Some(origin) = self.plane_point(from.0, from.1, width, height, z) else { return false };
            self.modeller.grid_drag = Some(GridDrag { indices, before: self.modeller.model.grids().to_vec(), origin: origin.truncate() });
        }
        let Some(drag) = self.modeller.grid_drag.clone() else { return false };
        if let Some(at) = self.plane_point(to.0, to.1, width, height, z) {
            let by = ((at.truncate() - drag.origin) / DRAG_ROUND).round() * DRAG_ROUND;
            for &index in &drag.indices {
                let Some(line) = drag.before.get(index).cloned() else { continue };
                let _ = self.modeller.model.set_grid(index, bim_core::GridDef { start: line.start + by, end: line.end + by, ..line });
            }
            self.show_modelled();
            self.pick_modelled_grids(&drag.indices);
        }
        if done {
            self.modeller.grid_drag = None;
            self.modeller.touched = true;
        }
        true
    }

    /// The middle of what the turn under way takes: of its grid lines, else of its elements'
    /// places. The centre a typed angle turns about when none was clicked.
    fn turn_centre(&self, t: &crate::app::transform::Transform) -> Option<DVec2> {
        let mut points: Vec<DVec2> = t.grids.iter().filter_map(|&i| self.modeller.model.grids().get(i)).flat_map(|g| [g.start, g.end]).collect();
        if points.is_empty() {
            let grid = self.modeller.model.grid;
            points = t.elements.iter().filter_map(|&id| self.modeller.model.get(id)?.shape_in_project(&grid).placement()).map(|p| p.truncate()).collect();
        }
        (!points.is_empty()).then(|| points.iter().copied().sum::<DVec2>() / points.len() as f64)
    }

    /// Enter in the angle field of Rotate: what the turn takes turns by the angle typed (degrees,
    /// anticlockwise; a comma or a point) about the centre clicked, else about its own middle.
    /// An angle that is no number leaves everything as it is.
    pub(in crate::app) fn turn_by_typed(&mut self) {
        use crate::app::transform::{Map, TransformKind};
        let Some(t) = self.modeller.transform.clone().filter(|t| t.kind == TransformKind::Rotate) else { return };
        let Ok(degrees) = self.modeller.turn_typed.trim().trim_end_matches('°').replace(',', ".").parse::<f64>() else { return };
        let Some(about) = t.points.first().map(|p| p.truncate()).or_else(|| self.turn_centre(&t)) else { return };
        if !degrees.is_finite() {
            return;
        }
        self.modeller.transform = None;
        self.modeller.turn_typed.clear();
        self.transform_picked(&t, &Map::Turn { about, angle: degrees.to_radians() });
    }

    /// The angle field of Rotate, at the top of the view while a turn is under way.
    pub(in crate::app) fn turn_overlay(&self, p: crate::theme::Palette, l: Language) -> Option<iced::Element<'_, crate::app::Message>> {
        use crate::app::Message;
        use iced::widget::{button, column, container, row, text, text_input};
        use iced::Fill;
        let t = self.modeller.transform.as_ref().filter(|t| t.kind == crate::app::transform::TransformKind::Rotate)?;
        let typed = &self.modeller.turn_typed;
        let valid = typed.trim().trim_end_matches('°').replace(',', ".").parse::<f64>().is_ok_and(f64::is_finite);
        let field = text_input("90", typed).on_input(Message::TurnTyped).on_submit(Message::TurnApply).size(12).padding([3, 6]).width(90);
        let turn = button(text(l.text("Draaien", "Turn")).size(12)).padding([3, 10]).style(move |_, s| p.ribbon_button(valid, s)).on_press_maybe(valid.then_some(Message::TurnApply));
        let about = if t.points.is_empty() { l.text("om het midden van de selectie", "about the middle of the selection") } else { l.text("om het geklikte draaipunt", "about the centre clicked") };
        let card = container(
            column![
                text(l.text("Hoek in graden (tegen de klok in)", "Angle in degrees (anticlockwise)")).size(12).color(p.text_secondary),
                row![field, turn].spacing(6),
                text(about).size(11).color(p.text_muted),
            ]
            .spacing(6),
        )
        .padding(8)
        .width(CARD)
        .style(move |_| p.panel());
        Some(container(card).padding(iced::Padding { top: 56.0, ..iced::Padding::ZERO }).center_x(Fill).into())
    }

    /// The plus past the end of the picked grid line, and the series field it opens.
    pub(in crate::app) fn grid_series_overlay(&self, p: crate::theme::Palette, l: Language) -> Option<iced::Element<'_, crate::app::Message>> {
        use crate::app::Message;
        use iced::widget::{button, column, container, row, stack, text, text_input};
        use iced::{Center, Fill, Padding};
        let line = self.modelled_grid_lines().into_iter().nth(self.picked_modelled_grid()?)?;
        let (width, height) = (self.viewport_size.0.max(1.0), self.viewport_size.1.max(1.0));
        let (a, b) = (glam::Vec3::from_array(line.a).as_dvec3(), glam::Vec3::from_array(line.b).as_dvec3());
        let (a, b) = (self.on_screen(a, width, height)?, self.on_screen(b, width, height)?);
        // past the end and its bubble, along the line on screen
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx.hypot(dy).max(1.0);
        let (x, y) = (b.0 + dx / length * PLUS_PAST, b.1 + dy / length * PLUS_PAST);
        let at = |w: f32, h: f32| Padding { left: (x - PLUS / 2.0).clamp(0.0, (width - w).max(0.0)), top: (y - PLUS / 2.0).clamp(0.0, (height - h).max(0.0)), ..Padding::ZERO };
        let plus = button(text("+").size(16).color(iced::Color::WHITE).align_x(Center).align_y(Center).width(Fill).height(Fill))
            .width(PLUS)
            .height(PLUS)
            .padding(0)
            .style(move |_, _| iced::widget::button::Style {
                background: Some(p.accent.into()),
                border: iced::Border { radius: (PLUS / 2.0).into(), width: 1.0, color: iced::Color::WHITE },
                ..Default::default()
            })
            .on_press(Message::GridSeries(SeriesMessage::Toggle));
        let mut layers = stack![container(plus).padding(at(PLUS, PLUS))].width(Fill).height(Fill);
        if let Some(typed) = &self.modeller.grid_series {
            let valid = spacings(typed).is_some();
            let field = text_input("5000 3000 3x5400", typed)
                .on_input(|t| Message::GridSeries(SeriesMessage::Typed(t)))
                .on_submit(Message::GridSeries(SeriesMessage::Create))
                .size(12)
                .padding([3, 6]);
            let create = button(text(l.text("Maken", "Create")).size(12)).padding([3, 10]).style(move |_, s| p.ribbon_button(valid, s)).on_press_maybe(valid.then_some(Message::GridSeries(SeriesMessage::Create)));
            let cancel = button(text(l.text("Annuleren", "Cancel")).size(12)).padding([3, 10]).style(move |_, s| p.ribbon_button(false, s)).on_press(Message::GridSeries(SeriesMessage::Toggle));
            let card = container(
                column![
                    text(l.text("Reeks stramienen: afstanden in mm", "Series of grid lines: spacings in mm")).size(12).color(p.text_secondary),
                    field,
                    row![create, cancel].spacing(6),
                ]
                .spacing(6),
            )
            .padding(8)
            .width(CARD)
            .style(move |_| p.panel());
            let mut place = at(CARD, 96.0);
            place.left = (x + PLUS).clamp(0.0, (width - CARD).max(0.0));
            layers = layers.push(container(card).padding(place));
        }
        Some(layers.into())
    }

    /// Switches the Grid tool on or off; a tool that places elements goes off.
    pub(in crate::app) fn toggle_grid_tool(&mut self) {
        let on = !self.modeller.grid_tool;
        self.set_model_tool(None);
        self.modeller.grid_tool = on;
        if on {
            self.measure.tool = None;
            self.plane_picking = false;
        }
    }

    /// Esc while the Grid tool is on: drop the point clicked, else switch the tool off.
    pub(in crate::app) fn escape_grid_tool(&mut self) {
        self.modeller.refusal = None;
        if self.modeller.points.is_empty() {
            self.modeller.grid_tool = false;
            self.modeller.cursor = None;
        } else {
            self.modeller.points.clear();
        }
    }

    /// The cursor rests at (x, y) while the Grid tool is on: the preview follows it. False when
    /// the tool is off.
    pub(in crate::app) fn grid_hover(&mut self, x: f32, y: f32, width: f32, height: f32) -> bool {
        if !self.modeller.grid_tool {
            return false;
        }
        let (width, height) = (width.max(1.0), height.max(1.0));
        let (z, _, _) = self.work_plane();
        let at = self.plane_point(x, y, width, height, z).map(|p| self.snapped_as(p, x, y, width, height));
        self.modeller.cursor = at.map(|(p, _)| p);
        self.modeller.snap_mark = at.and_then(|(_, kind)| kind);
        true
    }

    /// A click in the viewport while the Grid tool is on: the line's start, then its end.
    pub(in crate::app) fn grid_click(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let (z, _, _) = self.work_plane();
        let Some(p) = self.plane_point(x, y, width, height, z) else { return };
        let p = self.snapped(p, x, y, width, height);
        self.modeller.refusal = None;
        let Some(first) = self.modeller.points.first().copied() else {
            self.modeller.points.push(p);
            return;
        };
        self.modeller.points.clear();
        match self.add_grid_line(first.truncate(), p.truncate()) {
            Ok(_) => {
                self.modeller.grid_tool = false;
                self.modeller.cursor = None;
            }
            Err(refusal) => self.modeller.refusal = Some(refusal),
        }
    }

    /// Adds a grid line from `start` to `end` (scene coordinates) to the modelled project, shows
    /// it and picks it; returns its index among the project's grid lines.
    pub(in crate::app) fn add_grid_line(&mut self, start: DVec2, end: DVec2) -> Result<usize, bim_core::Refusal> {
        let index = self.modeller.model.add_grid(start, end)?;
        self.show_modelled();
        let shown = self.shown_grid(index);
        self.pick_grid(shown);
        Ok(index)
    }

    /// The grid lines of the modelled project as the viewport draws them: at the lowest level.
    pub(in crate::app) fn modelled_grid_lines(&self) -> Vec<GridLine> {
        let z = self.document.levels().iter().map(|l| l.elevation).fold(f64::INFINITY, f64::min);
        let z = if z.is_finite() { z as f32 } else { 0.0 };
        self.modeller.model.grids().iter().map(|g| GridLine { name: g.name.clone(), a: [g.start.x as f32, g.start.y as f32, z], b: [g.end.x as f32, g.end.y as f32, z] }).collect()
    }

    /// Which of the grid lines shown modelled grid line `index` is: the scene may hold those of
    /// opened files too.
    fn shown_grid(&self, index: usize) -> Option<usize> {
        let line = self.modelled_grid_lines().into_iter().nth(index)?;
        self.model.as_ref()?.project.grids.iter().position(|g| *g == line)
    }

    /// Which modelled grid line the grid line picked in the viewport is; `None` when none is
    /// picked or it is one of an opened file.
    pub(in crate::app) fn picked_modelled_grid(&self) -> Option<usize> {
        let picked = self.model.as_ref()?.project.grids.get(self.picked_grid?)?;
        self.modelled_grid_lines().iter().position(|g| g == picked)
    }

    /// Which modelled grid lines the grid lines picked in the viewport are; those of an opened
    /// file are left out.
    pub(crate) fn picked_modelled_grids(&self) -> Vec<usize> {
        let Some(model) = self.model.as_ref() else { return Vec::new() };
        let modelled = self.modelled_grid_lines();
        self.grids_picked().into_iter().filter_map(|g| model.project.grids.get(g)).filter_map(|picked| modelled.iter().position(|g| g == picked)).collect()
    }

    /// Removes the picked grid lines of the modelled project. False when none was picked.
    pub(in crate::app) fn delete_picked_grid(&mut self) -> bool {
        let mut indices = self.picked_modelled_grids();
        if indices.is_empty() {
            return false;
        }
        // from the last on, so the others keep their place
        indices.sort_unstable();
        indices.dedup();
        for index in indices.into_iter().rev() {
            self.modeller.model.remove_grid(index);
        }
        self.set_picked_grids(Vec::new());
        self.show_modelled();
        true
    }

    /// The points of the grid lines a click snaps to, on the work plane: their ends and where
    /// they cross.
    pub(in crate::app) fn grid_snap_points(&self) -> Vec<DVec3> {
        let grids = self.modeller.model.grids();
        if grids.is_empty() {
            return Vec::new();
        }
        let (z, _, _) = self.work_plane();
        let mut points: Vec<DVec3> = grids.iter().flat_map(|g| [g.start.extend(z), g.end.extend(z)]).collect();
        for (i, g) in grids.iter().enumerate() {
            points.extend(grids[i + 1..].iter().filter_map(|other| g.crossing(other)).map(|p| p.extend(z)));
        }
        points
    }

    /// What to click next with the Grid tool, for the status line; `None` when the tool is off.
    pub(in crate::app) fn grid_hint(&self, l: Language) -> Option<String> {
        if !self.modeller.grid_tool {
            return None;
        }
        let hint = if self.modeller.points.is_empty() {
            l.text("Klik het beginpunt · Esc: stoppen", "Click the start point · Esc: stop")
        } else {
            l.text("Klik het eindpunt · Esc: opnieuw", "Click the end point · Esc: start again")
        };
        let tool = l.text("Stramien", "Grid");
        Some(match self.modeller.refusal {
            Some(refusal) => format!("{tool}: {} · {hint}", l.translate(refusal.reason())),
            None => format!("{tool}: {hint}"),
        })
    }

    /// The preview of the grid line being drawn: from the point clicked to the cursor's point.
    /// `None` when the tool is off or the cursor is away.
    pub(in crate::app) fn grid_overlay(&self, ink: Color) -> Option<PlacingOverlay> {
        if !self.modeller.grid_tool {
            return None;
        }
        let cursor = self.modeller.cursor?;
        let points: Vec<glam::Vec3> = self.modeller.points.iter().map(|p| p.as_vec3()).collect();
        // the line as a face of two corners: its outline is the line
        let faces = points.first().map(|first| vec![vec![vec![*first, cursor.as_vec3()]]]).unwrap_or_default();
        Some(PlacingOverlay { faces, points, cursor: cursor.as_vec3(), view_proj: self.camera.view_proj(self.aspect()) * self.model_matrix(), ink, mark: self.snap_mark(), readout: self.tool_readout() })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_series_typed_beside_a_picked_grid_line_adds_parallel_lines_with_the_next_names() {
        use super::super::tests::modelling;
        use super::{spacings, SeriesMessage};
        use crate::app::Message;
        use glam::DVec2;
        assert_eq!(spacings("5000 3000 3x5400"), Some(vec![5.0, 3.0, 5.4, 5.4, 5.4]));
        assert_eq!(spacings(" 2X1200,5 "), Some(vec![1.2005, 1.2005]));
        for wrong in ["", "abc", "0", "-5", "3x", "x5", "0x5"] {
            assert_eq!(spacings(wrong), None, "{wrong}");
        }
        let mut app = modelling();
        // a line up the plan is numbered: its series goes to the right
        app.add_grid_line(DVec2::new(0.0, 0.0), DVec2::new(0.0, 8.0)).unwrap();
        assert!(app.grid_series_overlay(crate::theme::ThemeKind::Light.palette(), openaec_ui::Language::English).is_some());
        let _ = app.update(Message::GridSeries(SeriesMessage::Toggle));
        let _ = app.update(Message::GridSeries(SeriesMessage::Typed("5000 2x3000".into())));
        let _ = app.update(Message::GridSeries(SeriesMessage::Create));
        let lines: Vec<(String, f64)> = app.modeller.model.grids().iter().map(|g| (g.name.clone(), g.start.x)).collect();
        assert_eq!(lines, [("1".to_string(), 0.0), ("2".to_string(), 5.0), ("3".to_string(), 8.0), ("4".to_string(), 11.0)]);
        assert!(app.modeller.grid_series.is_none() && app.picked_modelled_grid() == Some(0), "the field closes, the line stays picked");
        // a line across the plan is lettered: its series goes up
        app.add_grid_line(DVec2::new(12.0, 0.0), DVec2::new(-1.0, 0.0)).unwrap();
        assert!(app.add_grid_series(&[4.0]));
        let across: Vec<(String, f64)> = app.modeller.model.grids().iter().skip(4).map(|g| (g.name.clone(), g.start.y)).collect();
        assert_eq!(across, [("A".to_string(), 0.0), ("B".to_string(), 4.0)]);
        // a text that is no series adds nothing
        let _ = app.update(Message::GridSeries(SeriesMessage::Typed("abc".into())));
        let _ = app.update(Message::GridSeries(SeriesMessage::Create));
        assert_eq!(app.modeller.model.grids().len(), 6);
    }

    use super::super::tests::{click, modelling};
    use super::super::ModelTool;
    use crate::app::Message;
    use bim_core::Shape;
    use glam::DVec2;

    fn names(app: &crate::app::App) -> Vec<String> {
        app.model.as_ref().unwrap().project.grids.iter().map(|g| g.name.clone()).collect()
    }

    #[test]
    fn copy_and_move_take_the_picked_grid_line_when_no_element_is_selected() {
        use crate::app::transform::TransformKind;
        let mut app = modelling();
        app.add_grid_line(DVec2::new(2.0, -1.0), DVec2::new(2.0, 8.0)).unwrap();
        app.show_modelled();
        app.pick_grid(Some(0));
        // CO: from one click to the next, the line stays and a parallel one gets the next number
        let _ = app.update(Message::Transform(TransformKind::Copy));
        // (a click near a grid line's end snaps to it, so the clicks are on the ends)
        click(&mut app, 2.0, -1.0);
        click(&mut app, 7.4, -1.0);
        assert_eq!(names(&app), ["1", "2"]);
        let grids = app.modeller.model.grids();
        assert_eq!((grids[0].start, grids[1].start, grids[1].end), (DVec2::new(2.0, -1.0), DVec2::new(7.4, -1.0), DVec2::new(7.4, 8.0)));
        assert_eq!(app.picked_grid, Some(1), "the copy is picked, to copy on");
        // MV: the picked line itself goes, and keeps its name
        let _ = app.update(Message::Transform(TransformKind::Move));
        click(&mut app, 7.4, -1.0);
        click(&mut app, 9.0, -1.0);
        assert_eq!(names(&app), ["1", "2"]);
        assert_eq!(app.modeller.model.grids()[1].start, DVec2::new(9.0, -1.0));
        // a turn takes it too: by the angle typed, about its own middle
        let _ = app.update(Message::Transform(TransformKind::Rotate));
        let _ = app.update(Message::TurnTyped("90".into()));
        let _ = app.update(Message::TurnApply);
        let turned = &app.modeller.model.grids()[1];
        assert!(app.modeller.transform.is_none());
        assert!((turned.start - DVec2::new(13.5, 3.5)).length() < 1e-9 && (turned.end - DVec2::new(4.5, 3.5)).length() < 1e-9, "{turned:?}");
        assert_eq!(app.picked_grid, Some(1), "it stays picked");
    }

    #[test]
    fn a_box_picks_several_grid_lines_which_are_dragged_turned_and_deleted_together() {
        use crate::viewport::Navigate;
        let mut app = modelling();
        app.modeller.in_modeller = true;
        for x in [2.0, 5.0, 8.0] {
            app.add_grid_line(DVec2::new(x, -1.0), DVec2::new(x, 8.0)).unwrap();
        }
        let (w, h) = app.viewport_size;
        let at = |app: &crate::app::App, x: f64, y: f64| app.on_screen(glam::DVec3::new(x, y, 0.0), w, h).unwrap();
        // a box drawn to the right over the first two, whole: they are picked, the third is not
        let (a, b) = (at(&app, 1.0, 9.0), at(&app, 6.0, -2.0));
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let _ = app.update(Message::Navigate(Navigate::Box { x0, y0, x1, y1, width: w, height: h, done: true, add: false }));
        assert_eq!(app.picked_modelled_grids(), [0, 1]);
        // a drag that starts on one of them moves both, as one step
        let (from, to) = (at(&app, 2.0, 3.0), at(&app, 3.5, 3.0));
        let _ = app.update(Message::Navigate(Navigate::Box { x0: from.0, y0: from.1, x1: to.0, y1: to.1, width: w, height: h, done: false, add: false }));
        let _ = app.update(Message::Navigate(Navigate::Box { x0: from.0, y0: from.1, x1: to.0, y1: to.1, width: w, height: h, done: true, add: false }));
        let xs: Vec<f64> = app.modeller.model.grids().iter().map(|g| g.start.x).collect();
        assert!((xs[0] - 3.5).abs() < 0.011 && (xs[1] - 6.5).abs() < 0.011 && xs[2] == 8.0, "{xs:?}");
        assert!(app.box_select.is_none() && app.modeller.grid_drag.is_none());
        assert_eq!(app.picked_modelled_grids(), [0, 1], "they stay picked");
        let _ = app.update(Message::Undo);
        assert_eq!(app.modeller.model.grids()[0].start.x, 2.0, "one step back");
        let _ = app.update(Message::Redo);
        // Delete takes both
        app.set_picked_grids(vec![0, 1]);
        let _ = app.update(Message::DeletePicked);
        assert_eq!(names(&app), ["3"]);
    }

    #[test]
    fn two_clicks_draw_a_grid_line_that_names_itself_and_is_picked() {
        let mut app = modelling();
        let _ = app.update(Message::GridTool);
        assert!(app.modeller.grid_tool && app.grid_hint(openaec_ui::Language::English).is_some());
        click(&mut app, 2.0, -1.0);
        // nearly straight up: put in line with the first point
        click(&mut app, 2.05, 8.0);
        assert!(!app.modeller.grid_tool, "the tool goes off");
        let grid = &app.modeller.model.grids()[0];
        assert_eq!((grid.name.as_str(), grid.start, grid.end), ("1", DVec2::new(2.0, -1.0), DVec2::new(2.0, 8.0)));
        assert_eq!(names(&app), ["1"]);
        assert_eq!(app.picked_grid, Some(0));
        // one across it: lettered
        let _ = app.update(Message::GridTool);
        click(&mut app, -1.0, 3.0);
        click(&mut app, 9.0, 3.0);
        assert_eq!(names(&app), ["1", "A"]);
        // a click of a modelling tool snaps to where they cross
        app.set_model_tool(Some((ModelTool::Column, "Column")));
        click(&mut app, 2.2, 3.2);
        let column = app.modeller.model.iter().next().unwrap().shape_in_project(&app.modeller.model.grid);
        let Shape::Linear(l) = column else { panic!() };
        assert_eq!(l.start.truncate(), DVec2::new(2.0, 3.0));
        app.set_model_tool(None);
    }

    #[test]
    fn the_grid_lines_of_a_project_come_back_when_it_is_opened_again() {
        use std::sync::Arc;
        let mut app = modelling();
        app.add_grid_line(DVec2::new(2.0, -1.0), DVec2::new(2.0, 8.0)).unwrap();
        app.add_grid_line(DVec2::new(-1.0, 3.25), DVec2::new(9.0, 3.25)).unwrap();
        let written = app.modeller.model.grids().to_vec();
        let scene = app.model.clone().unwrap();
        let bytes = crate::export::to_bytes(&scene, &app.document, "house.ifcx").expect("IFCX");
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("openaec::grid") && text.contains("IfcGridAxis"));
        let source = crate::loaders::Source::Bytes { name: "house.ifcx".into(), data: Arc::new(bytes) };
        let loaded = crate::loaders::load_with(&source, &crate::loaders::Progress::default()).expect("read back");
        let mut back = crate::app::App::default();
        back.set_model(Arc::new(loaded));
        assert_eq!(back.modeller.model.grids(), written, "every grid line comes back");
        // shown once, and the project's own: the next one takes the next name
        assert_eq!(names(&back), ["1", "A"]);
        back.add_grid_line(DVec2::new(6.0, -1.0), DVec2::new(6.0, 8.0)).unwrap();
        assert_eq!(names(&back), ["1", "A", "2"]);
    }

    #[test]
    fn a_grid_line_is_taken_back_removed_and_refused_without_length() {
        let mut app = modelling();
        let _ = app.update(Message::GridTool);
        click(&mut app, 0.0, 0.0);
        click(&mut app, 0.0, 6.0);
        let _ = app.update(Message::GridTool);
        click(&mut app, 4.0, 0.0);
        click(&mut app, 4.0, 6.0);
        assert_eq!(names(&app), ["1", "2"]);
        // undo takes the last one back, redo gives it again
        let _ = app.update(Message::Undo);
        assert_eq!(names(&app), ["1"]);
        let _ = app.update(Message::Redo);
        assert_eq!(names(&app), ["1", "2"]);
        // Delete removes the picked one
        app.pick_grid(Some(0));
        let _ = app.update(Message::DeletePicked);
        assert_eq!(names(&app), ["2"]);
        assert_eq!(app.picked_grid, None);
        // a line without length is refused and the tool stays on; Esc drops the point, then the tool
        let _ = app.update(Message::GridTool);
        click(&mut app, 8.0, 8.0);
        click(&mut app, 8.0, 8.0);
        assert_eq!(app.modeller.refusal, Some(bim_core::Refusal::ZeroLength));
        assert!(app.modeller.grid_tool);
        click(&mut app, 1.0, 1.0);
        let _ = app.update(Message::Escape);
        assert!(app.modeller.grid_tool && app.modeller.points.is_empty());
        let _ = app.update(Message::Escape);
        assert!(!app.modeller.grid_tool);
    }
}
