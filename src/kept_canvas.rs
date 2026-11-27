//! A canvas's drawing kept from frame to frame until what it shows changes.
//!
//! A canvas draws itself again at every frame of the window, also when only the pointer moved
//! over something else. What takes time to draw (the hatches of a floor plan's cut, the view
//! cube) is drawn through a [`Kept`] in the canvas's state, with a key made of everything the
//! drawing follows from: the same key gives the drawing of the frame before.

//!
//! A drawing that is only moved or zoomed is not made again either: a [`moved`] layer keeps it
//! as it was drawn and has the graphics card shift and scale it, as long as it covers what shows.

use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::Widget;
use iced::widget::canvas::{Cache, Frame, Geometry};
use iced::{mouse, Color, Element, Length, Rectangle, Renderer, Size, Theme, Transformation, Vector};
use std::cell::Cell;
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub struct Kept {
    cache: Cache,
    key: Cell<Option<u64>>,
}

impl Kept {
    /// The drawing `draw` makes, made again only when `key` differs from the frame before (or the
    /// canvas has another size).
    pub fn draw(&self, renderer: &iced::Renderer, size: Size, key: u64, draw: impl FnOnce(&mut Frame)) -> Geometry {
        if self.key.get() != Some(key) {
            self.cache.clear();
            self.key.set(Some(key));
        }
        self.cache.draw(renderer, size, draw)
    }
}

/// A key of what `parts` say (numbers by their bits, the rest as it is written out).
pub fn key(numbers: &[f32], rest: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for n in numbers {
        n.to_bits().hash(&mut hasher);
    }
    rest.hash(&mut hasher);
    hasher.finish()
}

/// The size a text of `pixels` on a drawing is written in: the nearest of a ladder of sizes at
/// most a few hundredths apart. A drawing that is zoomed has its texts in another size at every
/// step, and each letter of each new size is drawn from its outline; off the ladder that takes
/// half the time of a frame, on it the sizes come back and are had already.
pub fn text_size(pixels: f32) -> iced::Pixels {
    let step = match pixels {
        p if p < 16.0 => 0.5,
        p if p < 40.0 => 1.0,
        p if p < 100.0 => 2.0,
        _ => 4.0,
    };
    iced::Pixels(((pixels / step).round() * step).max(0.5))
}

/// A drawing a [`moved`] layer shows: it lies on the screen by a scale and a shift, and is drawn
/// anew only when something else about it changes.
pub trait Drawing {
    /// The name of the layer in the frame log.
    fn name(&self) -> &'static str;
    /// Everything the drawing follows from, but where it lies: its [`Drawing::lie`].
    fn key(&self, size: Size) -> u64;
    /// Where it lies in a layer of `size`: the pixels of one unit of the drawing, and the pixel
    /// of its origin. `None` for one that cannot be shifted and scaled (seen in perspective).
    fn lie(&self, size: Size) -> Option<[f32; 3]>;
    /// Draws it as it lies now. The frame reaches a layer's size beyond the layer on every side:
    /// what falls there is drawn too, and shows when the drawing is moved.
    fn draw(&self, frame: &mut Frame, size: Size);
}

/// How far beyond the layer a drawing is kept, in layers.
const MARGIN: f32 = 1.0;
/// How much larger or smaller than drawn a kept drawing may show: its lines are as much wider
/// or thinner until it is drawn again.
const ZOOM: f32 = 1.6;

#[derive(Default)]
struct MovedState {
    cache: Cache,
    /// The key, the lie and the layer's size the kept drawing was drawn with.
    drawn: Cell<Option<(u64, [f32; 3], Size)>>,
    /// The lie of the frame before.
    last: Cell<Option<[f32; 3]>>,
}

/// How a drawing kept as it lay at `drawn` shows at `now` in a layer of `size`: its scale and
/// shift, or `None` when it must be drawn again: it would be too large or small, it does not
/// cover the layer any more, or the view has come to rest at another size (`rests`).
fn kept_as(drawn: [f32; 3], now: [f32; 3], size: Size, rests: bool) -> Option<(f32, Vector)> {
    let k = now[0] / drawn[0];
    if !k.is_finite() || !(1.0 / ZOOM..=ZOOM).contains(&k) || (rests && (k - 1.0).abs() > 1e-4) {
        return None;
    }
    let shift = Vector::new(now[1] - k * drawn[1], now[2] - k * drawn[2]);
    // the part of the kept drawing that shows, in the pixels it was drawn in
    let (left, top) = (-shift.x / k, -shift.y / k);
    let (right, bottom) = ((size.width - shift.x) / k, (size.height - shift.y) / k);
    let (mx, my) = (size.width * MARGIN, size.height * MARGIN);
    (left >= -mx && top >= -my && right <= size.width + mx && bottom <= size.height + my).then_some((k, shift))
}

/// A layer that shows `drawing`, on `paper` if any, and takes no part in what the pointer does.
pub fn moved<'a, M: 'a>(drawing: impl Drawing + 'a, paper: Option<Color>) -> Element<'a, M> {
    Element::new(Moved { drawing, paper })
}

struct Moved<D> {
    drawing: D,
    paper: Option<Color>,
}

impl<M, D: Drawing> Widget<M, Theme, Renderer> for Moved<D> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<MovedState>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(MovedState::default())
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fill)
    }
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, _theme: &Theme, _style: &renderer::Style, layout: Layout<'_>, _cursor: mouse::Cursor, viewport: &Rectangle) {
        let _timed = crate::frame_log::canvas(self.drawing.name());
        let bounds = layout.bounds();
        let Some(clip) = bounds.intersection(viewport) else { return };
        let size = bounds.size();
        let state = tree.state.downcast_ref::<MovedState>();
        let (key, lie) = (self.drawing.key(size), self.drawing.lie(size));
        let kept = match (state.drawn.get(), lie) {
            (Some((k, drawn, s)), Some(now)) if k == key && s == size => kept_as(drawn, now, size, state.last.get() == lie || crate::frame_log::plain()),
            // one that cannot be moved shows as drawn while nothing changed at all
            (Some((k, _, s)), None) if k == key && s == size => Some((1.0, Vector::ZERO)),
            _ => None,
        };
        state.last.set(lie);
        let (k, shift) = kept.unwrap_or_else(|| {
            state.cache.clear();
            state.drawn.set(Some((key, lie.unwrap_or([1.0, 0.0, 0.0]), size)));
            (1.0, Vector::ZERO)
        });
        let margin = Vector::new(size.width * MARGIN, size.height * MARGIN);
        let geometry: Geometry = state.cache.draw(renderer, Size::new(size.width + 2.0 * margin.x, size.height + 2.0 * margin.y), |frame| {
            frame.translate(margin);
            self.drawing.draw(frame, size);
        });
        renderer.with_layer(clip, |renderer| {
            if let Some(paper) = self.paper {
                renderer.fill_quad(renderer::Quad { bounds, ..Default::default() }, paper);
            }
            let place = Transformation::translate(bounds.x + shift.x - k * margin.x, bounds.y + shift.y - k * margin.y) * Transformation::scale(k);
            renderer.with_transformation(place, |renderer| renderer.draw_geometry(geometry));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_differs_when_a_number_or_the_rest_does() {
        assert_eq!(key(&[1.0, 2.0], "a"), key(&[1.0, 2.0], "a"));
        assert_ne!(key(&[1.0, 2.0], "a"), key(&[1.0, 2.5], "a"));
        assert_ne!(key(&[1.0, 2.0], "a"), key(&[1.0, 2.0], "b"));
        assert_ne!(key(&[0.0], ""), key(&[-0.0], ""), "by their bits");
    }

    #[test]
    fn a_text_is_written_in_a_size_of_the_ladder_near_its_own() {
        assert_eq!(text_size(11.8).0, 12.0);
        assert_eq!(text_size(11.7).0, 11.5);
        assert_eq!(text_size(23.4).0, 23.0);
        assert_eq!(text_size(57.2).0, 58.0);
        assert_eq!(text_size(203.0).0, 204.0);
        assert_eq!(text_size(0.1).0, 0.5, "never none");
        for size in [3.0f32, 9.3, 15.9, 16.2, 39.9, 41.0, 99.0, 130.0] {
            assert!((text_size(size).0 / size - 1.0).abs() < 0.09, "{size}");
        }
    }

    #[test]
    fn a_kept_drawing_is_shifted_and_scaled_while_it_covers_the_layer() {
        let size = Size::new(800.0, 600.0);
        let drawn = [10.0, 400.0, 300.0];
        // not moved: as drawn
        assert_eq!(kept_as(drawn, drawn, size, true), Some((1.0, Vector::ZERO)));
        // panned: shifted as far, also at rest
        assert_eq!(kept_as(drawn, [10.0, 460.0, 280.0], size, true), Some((1.0, Vector::new(60.0, -20.0))));
        // zoomed in about the middle of the layer: scaled, the middle stays where it is
        let (k, shift) = kept_as(drawn, [12.0, 400.0, 300.0], size, false).unwrap();
        assert!((k - 1.2).abs() < 1e-6 && (shift.x + 80.0).abs() < 1e-3 && (shift.y + 60.0).abs() < 1e-3, "{k} {shift:?}");
        // and drawn again once the view rests there, in its own line widths
        assert_eq!(kept_as(drawn, [12.0, 400.0, 300.0], size, true), None);
        // too far in or out, or panned beyond what was drawn: again
        assert_eq!(kept_as(drawn, [17.0, 400.0, 300.0], size, false), None);
        assert_eq!(kept_as(drawn, [6.0, 400.0, 300.0], size, false), None);
        assert_eq!(kept_as(drawn, [10.0, 400.0 + 801.0, 300.0], size, false), None);
        assert!(kept_as(drawn, [10.0, 400.0 + 799.0, 300.0], size, false).is_some());
        assert_eq!(kept_as([0.0, 0.0, 0.0], drawn, size, false), None, "nothing to scale from");
    }
}
