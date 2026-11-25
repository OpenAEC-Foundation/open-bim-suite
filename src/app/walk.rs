//! Walking through the model in the 3D view with W, A, S and D, as in a game: forward, to the
//! left, back and to the right as the camera looks (level), Q and E down and up. The camera
//! moves as long as a key is held, smoothly, every frame; with Shift it runs.
//!
//! Outside the Modeller the keys walk whenever the 3D view shows; in the Modeller, whose letters
//! are its tools, while the right button looks around (`shell::Studio`, `Msg::Key`).
use super::{App, Message};
use glam::Vec3;
use web_time::Instant;

/// Walking speed in metres per second, and running with Shift.
const WALK: f32 = 3.0;
const RUN: f32 = 12.0;
/// A frame that comes late (the window was busy) moves no further than this long a step.
const LONGEST_STEP: f32 = 0.1;

/// The keys that walk, held now, and when the camera last moved.
#[derive(Debug, Default, Clone)]
pub(crate) struct Walking {
    /// W, S, D, A, E, Q: forward, back, right, left, up, down.
    held: [bool; 6],
    fast: bool,
    last: Option<Instant>,
}

/// The place of a walking key in [`Walking::held`].
pub(crate) fn walk_key(c: char) -> Option<usize> {
    "wsdaeq".find(c.to_ascii_lowercase())
}

impl Walking {
    pub(crate) fn any(&self) -> bool {
        self.held.iter().any(|&h| h)
    }

    /// (forward, right, up), each −1, 0 or 1.
    fn direction(&self) -> (f32, f32, f32) {
        let axis = |plus: usize, minus: usize| self.held[plus] as i8 as f32 - self.held[minus] as i8 as f32;
        (axis(0, 1), axis(2, 3), axis(4, 5))
    }
}

impl App {
    /// The 3D view shows a model to walk through (no sheet, view or drawing over it).
    pub(crate) fn can_walk(&self) -> bool {
        self.model.is_some() && self.shown_view().is_none() && self.active_sheet.is_none() && self.active_doc_sheet.is_none()
    }

    /// A walking key is held.
    pub(crate) fn is_walking(&self) -> bool {
        self.walking.any()
    }

    /// Looking around with the right button.
    pub(crate) fn looking(&self) -> bool {
        self.nav.held.is_some()
    }

    /// A walking key went down or up.
    pub(super) fn walk_key(&mut self, key: char, down: bool, fast: bool) {
        let Some(k) = walk_key(key) else { return };
        if down && !self.can_walk() {
            return;
        }
        let was = self.walking.any();
        self.walking.held[k] = down;
        if down {
            self.walking.fast = fast;
        }
        if down && !was {
            // the first key: the camera moves from this frame on, in perspective as one walks
            self.anim = None;
            self.camera.ortho = false;
            self.walking.last = Some(Instant::now());
        }
        if !self.walking.any() {
            self.walking.last = None;
        }
    }

    /// Every key let go (the window lost the keyboard).
    pub(super) fn walk_stop(&mut self) {
        self.walking = Walking::default();
    }

    /// A frame while keys are held: the camera moves on by the time since the last one.
    pub(super) fn walk_frame(&mut self, now: Instant) {
        let Some(last) = self.walking.last else { return };
        if !self.can_walk() {
            self.walk_stop();
            return;
        }
        let seconds = now.saturating_duration_since(last).as_secs_f32().min(LONGEST_STEP);
        self.walking.last = Some(now);
        let speed = if self.walking.fast { RUN } else { WALK };
        let (forward, right, up) = self.walking.direction();
        self.walk_by(forward, right, up, speed * seconds);
    }

    /// Moves the eye and what it looks at together by `distance`: level as the camera looks
    /// (`forward`), to its right, or straight up; the view keeps its direction.
    pub(super) fn walk_by(&mut self, forward: f32, right: f32, up: f32, distance: f32) {
        let (sin, cos) = self.camera.yaw.sin_cos();
        let ahead = Vec3::new(-cos, -sin, 0.0);
        let aside = Vec3::new(-sin, cos, 0.0);
        let way = ahead * forward + aside * right + Vec3::Z * up;
        if way != Vec3::ZERO {
            self.camera.target += way.normalize() * distance;
        }
    }
}

impl Message {
    /// A walking key from the keyboard, W A S D Q E, pressed or let go.
    pub(crate) fn walk(key: char, down: bool, fast: bool) -> Option<Message> {
        walk_key(key).map(|_| Message::WalkKey { key: key.to_ascii_lowercase(), down, fast })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn app_with_model() -> App {
        let mut app = App::default();
        app.set_model(std::sync::Arc::new(crate::model::Model::new(std::path::PathBuf::from("empty.obj"), "OBJ", Vec::new())));
        app
    }

    #[test]
    fn a_held_key_moves_the_camera_every_frame_by_the_time() {
        let mut app = app_with_model();
        app.camera.yaw = 0.0;
        let start = app.camera.target;
        let _ = app.update(Message::WalkKey { key: 'w', down: true, fast: false });
        let t0 = app.walking.last.unwrap();
        app.walk_frame(t0 + Duration::from_millis(50));
        app.walk_frame(t0 + Duration::from_millis(100));
        let moved = app.camera.target - start;
        // yaw 0 looks along −X: 0.1 s at walking speed
        assert!((moved.x + WALK * 0.1).abs() < 1e-4, "{moved:?}");
        assert!(moved.y.abs() < 1e-5 && moved.z.abs() < 1e-5);
        // let go: it stands still
        let _ = app.update(Message::WalkKey { key: 'w', down: false, fast: false });
        let here = app.camera.target;
        app.walk_frame(t0 + Duration::from_millis(200));
        assert_eq!(app.camera.target, here);
    }

    #[test]
    fn two_keys_walk_diagonally_no_faster_and_shift_runs() {
        let mut app = app_with_model();
        let start = app.camera.target;
        let _ = app.update(Message::WalkKey { key: 'w', down: true, fast: true });
        let _ = app.update(Message::WalkKey { key: 'd', down: true, fast: true });
        let t0 = app.walking.last.unwrap();
        app.walk_frame(t0 + Duration::from_millis(100));
        let moved = (app.camera.target - start).length();
        assert!((moved - RUN * 0.1).abs() < 1e-4, "{moved}");
    }

    #[test]
    fn a_late_frame_moves_no_further_than_a_short_step() {
        let mut app = app_with_model();
        let start = app.camera.target;
        let _ = app.update(Message::WalkKey { key: 'e', down: true, fast: false });
        let t0 = app.walking.last.unwrap();
        app.walk_frame(t0 + Duration::from_secs(3));
        assert!(((app.camera.target - start).z - WALK * LONGEST_STEP).abs() < 1e-4);
    }

    #[test]
    fn without_a_model_or_on_a_sheet_nothing_walks() {
        let mut app = App::default();
        let _ = app.update(Message::WalkKey { key: 'w', down: true, fast: false });
        assert!(!app.walking.any());
        assert_eq!(walk_key('x'), None);
        assert_eq!(walk_key('A'), Some(3));
    }
}
