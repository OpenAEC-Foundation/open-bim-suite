//! A canvas's drawing kept from frame to frame until what it shows changes.
//!
//! A canvas draws itself again at every frame of the window, also when only the pointer moved
//! over something else. What takes time to draw (the hatches of a floor plan's cut, the view
//! cube) is drawn through a [`Kept`] in the canvas's state, with a key made of everything the
//! drawing follows from: the same key gives the drawing of the frame before.

use iced::widget::canvas::{Cache, Frame, Geometry};
use iced::Size;
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
}
