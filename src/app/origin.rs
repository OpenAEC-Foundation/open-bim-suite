//! The zero point of each model: a mark with its three axes over the 3D view, always in sight, and
//! in the right pane, for the model picked in the explorer, where the model is moved to and where
//! its zero point lies in it.
//!
//! Moving a model moves all of it, its zero point along: the models are combined again with the
//! move added to the model's place ([`Model::combine_moved`]), so world coordinates stay those of
//! the file. Moving the zero point leaves the model where it is: only the mark moves, and the
//! place is kept with the model while it is open. Neither is written to a file yet.

use super::{App, Message as AppMessage, ModelEntry};
use crate::model::Model;
use crate::theme::Palette;
use glam::{DVec3, Mat4, Vec3};
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::widget::text::Wrapping;
use iced::widget::{column, row, text, text_input};
use iced::{alignment, mouse, Center, Color, Element, Length, Point, Rectangle, Task, Vector};
use openaec_ui::Language;
use std::sync::Arc;

/// What a typed place moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moves {
    /// The whole model, its zero point along.
    Model,
    /// The zero point alone, within the model.
    Zero,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// A coordinate typed for the picked model: what it moves, the axis (0 x, 1 y, 2 z), the text.
    Typed(Moves, usize, String),
    /// Use what is typed (Enter).
    Done(Moves),
    /// The turn typed for the picked model (degrees).
    Turn(String),
    /// Use the turn typed (Enter).
    Turned,
    /// The map of a model (by its id) came, as RGBA rows, or why not.
    Map(u64, Result<Arc<Vec<u8>>, String>),
}

/// The map of where a model lies.
#[derive(Debug, Clone, Default)]
pub enum Map {
    /// Not asked for.
    #[default]
    None,
    Loading,
    Ready(iced::widget::image::Handle),
    Failed(String),
}

/// Where a model was moved to and where its zero point lies in it (metres), with what is typed
/// for each until it is used.
#[derive(Debug, Clone, Default)]
pub struct Placed {
    pub moved: DVec3,
    /// How far the model is turned about its zero point (degrees, anticlockwise seen from above).
    pub turned: f64,
    pub zero: DVec3,
    typed: [[Option<String>; 3]; 2],
    typed_turn: Option<String>,
    pub map: Map,
}

/// The move and turn of a model that lay two of its points (`from`, in the scene) on two
/// others (`to`): turned so the line between its points runs as the line between the others,
/// and moved so their middles meet. `by` and `turned` are its move and turn now, `pivot` where
/// its zero point lies in the scene (the turn is about it). None when a pair's points lie on
/// top of each other in plan.
pub(crate) fn aligned_by_pairs(by: DVec3, turned: f64, pivot: DVec3, from: [DVec3; 2], to: [DVec3; 2]) -> Option<(DVec3, f64)> {
    let (along_from, along_to) = (from[1] - from[0], to[1] - to[0]);
    if along_from.truncate().length() < 1e-3 || along_to.truncate().length() < 1e-3 {
        return None;
    }
    let turn = along_to.y.atan2(along_to.x) - along_from.y.atan2(along_from.x);
    let (sin, cos) = turn.sin_cos();
    let rotate = |v: DVec3| DVec3::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos, v.z);
    let (a, b) = ((from[0] + from[1]) / 2.0, (to[0] + to[1]) / 2.0);
    // the scene is R(t)(p - c) + c + s + by, with c + s the pivot less the move: turned on, the
    // middle of the model's points lands on the middle of the others
    let moved = rotate(pivot - a) + b - pivot + by;
    let mut degrees = (turned + turn.to_degrees()) % 360.0;
    if degrees > 180.0 {
        degrees -= 360.0;
    } else if degrees <= -180.0 {
        degrees += 360.0;
    }
    Some((moved, degrees))
}

impl App {
    /// The remote's `place_model`: where each open model is placed (`list`), a model's move,
    /// turn and zero point set (`set`), a model aligned by two pairs of points (`align`), or a
    /// new revision of its file put in its place (`replace`). A model by its place in the list
    /// or its file name.
    pub(crate) fn place_model_command(&mut self, a: &serde_json::Value) -> Result<serde_json::Value, String> {
        use serde_json::json;
        let list = |app: &App| -> serde_json::Value {
            json!(app.models.iter().enumerate().map(|(k, e)| json!({
                "index": k, "file": e.model.file_name(), "visible": e.visible,
                "moved": e.placed.moved.to_array(), "turned": e.placed.turned, "zero": e.placed.zero.to_array(),
            })).collect::<Vec<_>>())
        };
        let action = a.get("action").and_then(|v| v.as_str()).unwrap_or("list");
        if action == "list" {
            return Ok(json!({ "models": list(self) }));
        }
        if self.models.is_empty() {
            // one model shown alone is not yet among the models: it becomes the first
            if let Some(shown) = self.model.clone() {
                self.models.push(ModelEntry::new(shown, 0));
            }
        }
        let i = match a.get("model") {
            Some(serde_json::Value::Number(n)) => n.as_u64().map(|n| n as usize).filter(|&n| n < self.models.len()).ok_or_else(|| format!("no model {n}; there are {}", self.models.len()))?,
            Some(serde_json::Value::String(name)) => self.models.iter().position(|e| e.model.file_name().eq_ignore_ascii_case(name) || e.model.path.to_string_lossy().eq_ignore_ascii_case(name)).ok_or_else(|| format!("no model named {name}"))?,
            _ if self.models.len() == 1 => 0,
            _ => return Err("give the model: its index or file name (place_model list shows them)".into()),
        };
        let point = |v: Option<&serde_json::Value>| -> Option<DVec3> {
            let p: Vec<f64> = v?.as_array()?.iter().filter_map(|x| x.as_f64()).collect();
            (p.len() == 3).then(|| DVec3::new(p[0], p[1], p[2]))
        };
        let said = match action {
            "set" => {
                let entry = &mut self.models[i];
                if let Some(moved) = point(a.get("moved")) {
                    entry.placed.moved = moved;
                }
                if let Some(turned) = a.get("turned").and_then(|v| v.as_f64()) {
                    entry.placed.turned = turned;
                }
                if let Some(zero) = point(a.get("zero")) {
                    entry.placed.zero = zero;
                }
                let name = entry.model.file_name();
                self.replace_models();
                format!("{name} placed")
            }
            "align" => {
                let pairs = a.get("pairs").and_then(|v| v.as_array()).filter(|p| p.len() == 2).ok_or("give two pairs: [{from: [x,y,z], to: [x,y,z]}, {from, to}]")?;
                let pair = |p: &serde_json::Value| -> Result<(Vec3, Vec3), String> {
                    let (from, to) = (point(p.get("from")), point(p.get("to")));
                    Ok((from.ok_or("a pair needs from: [x, y, z]")?.as_vec3(), to.ok_or("a pair needs to: [x, y, z]")?.as_vec3()))
                };
                let (first, second) = (pair(&pairs[0])?, pair(&pairs[1])?);
                self.align_model(i, first, second)?
            }
            #[cfg(not(target_arch = "wasm32"))]
            "replace" => {
                let path = a.get("path").and_then(|v| v.as_str()).ok_or("give the path of the new revision")?;
                let model = crate::loaders::load(std::path::Path::new(path))?;
                self.replace_model(i, Arc::new(model));
                match &self.status {
                    super::Status::Loaded(said) => said.clone(),
                    other => format!("{other:?}"),
                }
            }
            other => return Err(format!("place_model: unknown action {other} (list, set, align, replace)")),
        };
        Ok(json!({ "said": said, "models": list(self) }))
    }
}

/// A number as typed: a comma counts as a decimal point.
fn number(typed: &str) -> Option<f64> {
    typed.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite() && v.abs() < 1.0e7)
}

/// A coordinate as shown: millimetres kept, no trailing zeros.
fn shown(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

impl App {
    pub(super) fn update_origin(&mut self, message: Message) -> Task<AppMessage> {
        if let Message::Map(id, result) = message {
            if let Some(entry) = self.models.iter_mut().find(|entry| entry.model.id == id) {
                entry.placed.map = match result {
                    Ok(pixels) => Map::Ready(iced::widget::image::Handle::from_rgba(super::origin_map::SIZE.0, super::origin_map::SIZE.1, pixels.as_ref().clone())),
                    Err(why) => Map::Failed(why),
                };
            }
            return Task::none();
        }
        self.typed_origin(message);
        Task::none()
    }

    /// Where the file of a model says it lies: latitude and longitude (degrees), of its map
    /// conversion when it has one, else of its site.
    pub(super) fn located(entry: &ModelEntry) -> Option<(f64, f64)> {
        let info = &entry.model.project.info;
        let value = ["Georeference latitude, longitude", "Site latitude, longitude"].iter().find_map(|key| info.iter().find(|(k, _)| k == key))?;
        let (lat, lon) = value.1.split_once(',')?;
        Some((lat.trim().parse().ok()?, lon.trim().parse().ok()?))
    }

    /// Asks for the map of the picked model, once, when its file says where it lies.
    pub(super) fn load_map(&mut self) -> Task<AppMessage> {
        let Some(entry) = self.picked_model.and_then(|i| self.models.get_mut(i)) else { return Task::none() };
        let (Some((lat, lon)), Map::None) = (Self::located(entry), &entry.placed.map) else { return Task::none() };
        entry.placed.map = Map::Loading;
        map_task(entry.model.id, lat, lon)
    }

    fn typed_origin(&mut self, message: Message) {
        let Some(entry) = self.picked_model.and_then(|i| self.models.get_mut(i)) else { return };
        match message {
            Message::Map(..) => {}
            Message::Turn(typed) => entry.placed.typed_turn = Some(typed),
            Message::Turned => {
                let Some(next) = entry.placed.typed_turn.take().as_deref().and_then(number).map(|degrees| degrees.rem_euclid(360.0)) else { return };
                if next != entry.placed.turned {
                    entry.placed.turned = next;
                    self.replace_models();
                }
            }
            Message::Typed(moves, axis, typed) => {
                if let Some(slot) = entry.placed.typed[moves as usize].get_mut(axis) {
                    *slot = Some(typed);
                }
            }
            Message::Done(moves) => {
                let typed = std::mem::take(&mut entry.placed.typed[moves as usize]);
                let current = if moves == Moves::Model { entry.placed.moved } else { entry.placed.zero };
                let next = DVec3::from_array(std::array::from_fn(|k| typed[k].as_deref().and_then(number).unwrap_or(current[k])));
                if next == current {
                    return;
                }
                if moves == Moves::Zero {
                    entry.placed.zero = next;
                    return;
                }
                entry.placed.moved = next;
                self.replace_models();
            }
        }
    }

    /// Combines the models again, each at the place it was moved to; one model that was not
    /// moved shows as it was loaded.
    pub(super) fn replace_models(&mut self) {
        let picked = self.picked_model;
        let sources: Vec<Arc<Model>> = self.models.iter().map(|entry| entry.model.clone()).collect();
        let moves: Vec<DVec3> = self.models.iter().map(|entry| entry.placed.moved).collect();
        let turns: Vec<f64> = self.models.iter().map(|entry| entry.placed.turned).collect();
        let scene = match sources.as_slice() {
            [one] if moves[0] == DVec3::ZERO && turns[0] == 0.0 => one.clone(),
            _ => Arc::new(Model::combine_placed(&sources, &moves, &turns)),
        };
        self.set_scene(scene);
        self.restore_model_visibility();
        self.picked_model = picked;
        self.side_tab = super::SideTab::Properties;
    }

    /// Where the zero point of each shown model lies in the 3D view, with the model's name and
    /// whether it is the picked one.
    pub(super) fn zero_points(&self) -> Vec<(Vec3, String, bool)> {
        let Some(scene) = &self.model else { return Vec::new() };
        let mut first = 0usize;
        let mut out = Vec::new();
        for (i, entry) in self.models.iter().enumerate() {
            // a model's own zero is drawn where the scene shifted it to
            let shift = scene.origins.get(first).map_or(DVec3::ZERO, |o| o.shift);
            first += entry.model.origins.len().max(1);
            // a map layer fetched in the GIS tab lies in the project's own frame: no zero of its own
            if entry.visible && !self.is_map_layer(&entry.model) {
                out.push(((shift + entry.placed.zero).as_vec3(), entry.model.file_name(), self.picked_model == Some(i)));
            }
        }
        out
    }

    /// The rows of the right pane for the picked model: where it is moved to and where its zero
    /// point lies in it, each typed in and used on Enter.
    pub(super) fn origin_section(&self, p: Palette, l: Language, entry: &ModelEntry) -> Element<'_, AppMessage> {
        let fields = |moves: Moves, at: DVec3| {
            let mut fields = row![].spacing(4).width(Length::FillPortion(3));
            for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
                let value = entry.placed.typed[moves as usize][axis].clone().unwrap_or_else(|| shown(at[axis]));
                fields = fields.push(
                    text_input(name, &value)
                        .on_input(move |typed| AppMessage::Origin(Message::Typed(moves, axis, typed)))
                        .on_submit(AppMessage::Origin(Message::Done(moves)))
                        .size(12)
                        .padding([2, 4]),
                );
            }
            fields
        };
        let line = |label: &str, moves: Moves, at: DVec3| {
            row![text(label.to_string()).size(12).color(p.text_secondary).wrapping(Wrapping::WordOrGlyph).width(Length::FillPortion(2)), fields(moves, at)]
                .spacing(8)
                .align_y(Center)
        };
        let mut rows = column![
            line(l.text("Model verplaatst (m)", "Model moved (m)"), Moves::Model, entry.placed.moved),
            row![
                text(l.text("Model gedraaid (°)", "Model turned (°)").to_string()).size(12).color(p.text_secondary).wrapping(Wrapping::WordOrGlyph).width(Length::FillPortion(2)),
                row![openaec_ui::design::a11y::named(
                    text_input("°", &entry.placed.typed_turn.clone().unwrap_or_else(|| shown(entry.placed.turned))).on_input(|typed| AppMessage::Origin(Message::Turn(typed))).on_submit(AppMessage::Origin(Message::Turned)).size(12).padding([2, 4]),
                    openaec_ui::design::a11y::Role::Label,
                    l.text("Model gedraaid (°)", "Model turned (°)"),
                )]
                .width(Length::FillPortion(3)),
            ]
            .spacing(8)
            .align_y(Center),
            line(l.text("Nulpunt in model (m)", "Zero point in model (m)"), Moves::Zero, entry.placed.zero),
            text(l.text("X, Y, Z in meters, de draaiing in graden linksom om het nulpunt; Enter past toe.", "X, Y, Z in metres, the turn in degrees anticlockwise about the zero point; Enter applies.").to_string()).size(11).color(p.text_muted),
        ]
        .spacing(4);
        // moving by two points picked in the view: a point of the model, then where it goes; or
        // aligning by two pairs of them, which turns it as well
        let moving = self.moving_model.is_some() && !self.align_pairs;
        let aligning = self.moving_model.is_some() && self.align_pairs;
        let button = |label: &str, on: bool, message: AppMessage| iced::widget::button(text(label.to_string()).size(12)).padding([3, 10]).style(move |_, status| p.ribbon_button(on, status)).on_press(message);
        rows = rows.push(
            row![
                button(if moving { l.text("Verplaatsen…", "Moving…") } else { l.text("Verplaats met twee punten", "Move by two points") }, moving, AppMessage::MoveModelByPoints),
                button(if aligning { l.text("Uitlijnen…", "Aligning…") } else { l.text("Uitlijnen met twee paren", "Align by two pairs") }, aligning, AppMessage::AlignModelByPairs),
            ]
            .spacing(4)
            .wrap(),
        );
        if moving {
            rows = rows.push(text(l.text("Klik een punt van het model en daarna waar het heen moet (bijv. de kruising van twee stramienen). Esc breekt af.", "Click a point of the model, then where it should go (such as where two grids cross). Esc stops.").to_string()).size(11).color(p.text_muted).wrapping(Wrapping::WordOrGlyph));
        }
        if aligning {
            rows = rows.push(text(l.text(
                "Klik een punt van het model en waar het heen moet, en dan een tweede punt en waar dat heen moet (bijv. twee stramienkruisingen). Het model draait en schuift zo dat beide paren op elkaar vallen. Esc breekt af.",
                "Click a point of the model and where it should go, then a second point and where that should go (such as two grid crossings). The model turns and moves so both pairs meet. Esc stops.",
            ).to_string()).size(11).color(p.text_muted).wrapping(Wrapping::WordOrGlyph));
        }
        // a new revision of the file in its place, moved and turned as this one
        rows = rows.push(
            row![button(l.text("Nieuwe revisie…", "New revision…"), false, AppMessage::ReplaceModelDialog)]
                .spacing(4),
        )
        .push(text(l.text("Vervangt het bestand door een nieuwe versie; plaats en draaiing blijven.", "Replaces the file by a new version; its place and turn stay.").to_string()).size(11).color(p.text_muted).wrapping(Wrapping::WordOrGlyph));
        // the map of where its file says it lies
        let note = |words: String| text(words).size(11).color(p.text_muted).wrapping(Wrapping::WordOrGlyph);
        match &entry.placed.map {
            Map::None => {}
            Map::Loading => rows = rows.push(note(l.text("Kaart wordt opgehaald…", "Fetching the map…").to_string())),
            Map::Ready(picture) => {
                rows = rows
                    .push(iced::widget::image(picture.clone()).width(Length::Fill))
                    .push(note("© OpenStreetMap contributors".to_string()));
            }
            Map::Failed(why) => rows = rows.push(note(format!("{}: {why}", l.text("Geen kaart", "No map")))),
        }
        rows.into()
    }
}

/// Fetches the map of a place on a worker thread.
#[cfg(not(target_arch = "wasm32"))]
fn map_task(id: u64, lat: f64, lon: f64) -> Task<AppMessage> {
    Task::perform(
        async move {
            let (tx, rx) = iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(super::origin_map::fetch(lat, lon).map(Arc::new));
            });
            rx.await.unwrap_or_else(|_| Err("the map was not fetched".into()))
        },
        move |result| AppMessage::Origin(Message::Map(id, result)),
    )
}

/// The browser build shows no map.
#[cfg(target_arch = "wasm32")]
fn map_task(_: u64, _: f64, _: f64) -> Task<AppMessage> {
    Task::none()
}

/// The zero points over the 3D view: a ring with the three axes (x red, y green, z blue), each a
/// fixed length on screen, and the model's name. Drawn only: the viewport below handles the pointer.
pub struct ZeroPoints {
    pub marks: Vec<(Vec3, String, bool)>,
    /// World to clip space of the current camera.
    pub view_proj: Mat4,
    pub ink: Color,
    pub fill: Color,
    pub accent: Color,
}

/// The length of an axis on screen (logical pixels).
const AXIS: f32 = 44.0;

impl<M> canvas::Program<M> for ZeroPoints {
    type State = ();

    fn draw(&self, _: &(), renderer: &iced::Renderer, _: &iced::Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let _timed = crate::frame_log::canvas("ZeroPoints");
        let mut frame = Frame::new(renderer, bounds.size());
        let project = |world: Vec3| {
            let clip = self.view_proj * world.extend(1.0);
            (clip.w > 1e-6).then(|| {
                let ndc = clip.truncate() / clip.w;
                Point::new((ndc.x + 1.0) * 0.5 * bounds.width, (1.0 - ndc.y) * 0.5 * bounds.height)
            })
        };
        for (at, name, picked) in &self.marks {
            let Some(centre) = project(*at) else { continue };
            let axes = [(Vec3::X, Color::from_rgb(0.86, 0.20, 0.18), "X"), (Vec3::Y, Color::from_rgb(0.18, 0.62, 0.22), "Y"), (Vec3::Z, Color::from_rgb(0.16, 0.38, 0.86), "Z")];
            for (axis, color, letter) in axes {
                // the direction the axis runs on screen, drawn a fixed length
                let Some(tip) = project(*at + axis) else { continue };
                let along = Vector::new(tip.x - centre.x, tip.y - centre.y);
                let length = (along.x * along.x + along.y * along.y).sqrt();
                if length < 1e-3 {
                    continue;
                }
                let end = Point::new(centre.x + along.x / length * AXIS, centre.y + along.y / length * AXIS);
                frame.stroke(&Path::line(centre, end), Stroke::default().with_color(color).with_width(2.5));
                frame.fill_text(canvas::Text {
                    content: letter.into(),
                    position: Point::new(end.x + along.x / length * 8.0, end.y + along.y / length * 8.0),
                    size: iced::Pixels(11.0),
                    color,
                    align_x: alignment::Horizontal::Center.into(),
                    align_y: alignment::Vertical::Center,
                    font: super::FONT_BOLD,
                    ..canvas::Text::default()
                });
            }
            let ring = Path::circle(centre, if *picked { 6.5 } else { 5.0 });
            frame.fill(&ring, self.fill);
            frame.stroke(&ring, Stroke::default().with_color(if *picked { self.accent } else { self.ink }).with_width(if *picked { 2.5 } else { 1.5 }));
            frame.fill(&Path::circle(centre, 1.5), self.ink);
            frame.fill_text(canvas::Text {
                content: name.clone(),
                position: Point::new(centre.x + 10.0, centre.y + 10.0),
                size: iced::Pixels(11.0),
                color: self.ink,
                font: super::FONT,
                ..canvas::Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_pairs_of_points_turn_and_move_a_model_onto_them() {
        // the model's scene: R(t)(p - c) + c + s + by, here with its pivot (c + s + by) given
        let place = |p: DVec3, by: DVec3, turned: f64, pivot: DVec3| {
            let (sin, cos) = turned.to_radians().sin_cos();
            let c = pivot - by; // c + s
            let q = p - c;
            DVec3::new(q.x * cos - q.y * sin, q.x * sin + q.y * cos, q.z) + c + by
        };
        let (by, turned, c) = (DVec3::new(3.0, -1.0, 0.5), 20.0, DVec3::new(7.0, 2.0, 0.0));
        let pivot = c + by;
        // two points of the model, as the scene shows them now
        let (p0, p1) = (DVec3::new(1.0, 1.0, 0.0), DVec3::new(6.0, 4.0, 1.0));
        let (a0, a1) = (place(p0, by, turned, pivot), place(p1, by, turned, pivot));
        // where they should go: turned a further 35 degrees and shifted, a metre up
        let (sin, cos) = 35f64.to_radians().sin_cos();
        let go = |a: DVec3| DVec3::new(a.x * cos - a.y * sin + 40.0, a.x * sin + a.y * cos - 12.0, a.z + 1.0);
        let (b0, b1) = (go(a0), go(a1));
        let (moved, t) = aligned_by_pairs(by, turned, pivot, [a0, a1], [b0, b1]).unwrap();
        assert!((t - 55.0).abs() < 1e-9, "{t}");
        let pivot_after = c + moved;
        assert!(place(p0, moved, t, pivot_after).distance(b0) < 1e-9);
        assert!(place(p1, moved, t, pivot_after).distance(b1) < 1e-9);
        // points on top of each other in plan give no turn
        assert!(aligned_by_pairs(by, turned, pivot, [a0, a0 + DVec3::Z], [b0, b1]).is_none());
    }

    #[test]
    fn a_typed_number_reads_with_a_comma_or_a_point() {
        assert_eq!(number(" 1,5 "), Some(1.5));
        assert_eq!(number("-2.25"), Some(-2.25));
        assert_eq!(number("x"), None);
        assert_eq!(shown(1.5), "1.5");
        assert_eq!(shown(-0.0), "0");
        assert_eq!(shown(12.0), "12");
    }
}
