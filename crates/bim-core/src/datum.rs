//! Grid lines: the straight lines a project is set out on. A grid line is a datum, not an
//! element: it has a name and two ends in plan (project coordinates, metres) and no body.
//!
//! A new grid line names itself. A line that runs more north-south than east-west is numbered
//! (1, 2, 3 …); any other is lettered (A, B … Z, then A2, B2 …). The name is the one after the
//! last of its kind in use, so a name given away is not given again while a later one stands.

use crate::model::Model;
use crate::{Refusal, LINEAR_TOLERANCE};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A grid line: its name and its ends in plan, in project coordinates.
///
/// A locked (pinned) grid line keeps its name and its ends, and cannot be removed, until it is
/// unlocked: a grid line is what a project is set out on, and moving one by a slip of the hand
/// moves what hangs on it. Files from before the lock have none locked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridDef {
    pub name: String,
    pub start: DVec2,
    pub end: DVec2,
    #[serde(default)]
    pub locked: bool,
}

impl GridDef {
    /// Whether it runs more north-south than east-west: such lines are numbered.
    pub fn numbered(start: DVec2, end: DVec2) -> bool {
        let d = end - start;
        d.y.abs() > d.x.abs()
    }

    /// Where it crosses `other`, within the length of both.
    pub fn crossing(&self, other: &GridDef) -> Option<DVec2> {
        let (r, s) = (self.end - self.start, other.end - other.start);
        let denom = r.perp_dot(s);
        if denom.abs() < 1e-12 * r.length() * s.length() {
            return None;
        }
        let d = other.start - self.start;
        let (t, u) = (d.perp_dot(s) / denom, d.perp_dot(r) / denom);
        ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| self.start + r * t)
    }
}

/// The place of a lettered name in A, B … Z, A2, B2 …; `None` for any other name.
fn letter_index(name: &str) -> Option<u64> {
    let mut chars = name.chars();
    let letter = chars.next().filter(char::is_ascii_uppercase)?;
    let round = match chars.as_str() {
        "" => 1,
        digits => digits.parse::<u64>().ok().filter(|r| *r >= 2)?,
    };
    Some((round - 1) * 26 + (letter as u64 - 'A' as u64))
}

/// The lettered name at place `index` of A, B … Z, A2, B2 …
fn letter_name(index: u64) -> String {
    let letter = (b'A' + (index % 26) as u8) as char;
    match index / 26 {
        0 => letter.to_string(),
        round => format!("{letter}{}", round + 1),
    }
}

impl Model {
    /// The grid lines, in the order they were made.
    pub fn grids(&self) -> &[GridDef] {
        &self.grids
    }

    /// The name a new grid line from `start` to `end` gets: the number after the highest in use
    /// for one that runs north-south, else the letter after the last in use.
    pub fn next_grid_name(&self, start: DVec2, end: DVec2) -> String {
        if GridDef::numbered(start, end) {
            let last = self.grids.iter().filter_map(|g| g.name.parse::<u64>().ok()).max().unwrap_or(0);
            return (last + 1).to_string();
        }
        let next = self.grids.iter().filter_map(|g| letter_index(&g.name)).max().map_or(0, |i| i + 1);
        letter_name(next)
    }

    /// Adds a grid line from `start` to `end` under the next name of its kind; returns its
    /// index. A line without length is refused.
    pub fn add_grid(&mut self, start: DVec2, end: DVec2) -> Result<usize, Refusal> {
        let name = self.next_grid_name(start, end);
        self.insert_grid(GridDef { name, start, end, locked: false })
    }

    /// Adds grid line `grid` as it is (reading a file); returns its index. A line without length
    /// is refused.
    pub fn insert_grid(&mut self, grid: GridDef) -> Result<usize, Refusal> {
        if (grid.end - grid.start).length() <= LINEAR_TOLERANCE || !grid.start.is_finite() || !grid.end.is_finite() {
            return Err(Refusal::ZeroLength);
        }
        self.grids.push(grid);
        Ok(self.grids.len() - 1)
    }

    /// Changes grid line `index`; a line without length, or an index out of range, is refused.
    /// A locked line keeps its name and ends: a change of them is refused (`Locked`); only the
    /// lock itself can change (see [`Model::lock_grid`]).
    pub fn set_grid(&mut self, index: usize, grid: GridDef) -> Result<(), Refusal> {
        if (grid.end - grid.start).length() <= LINEAR_TOLERANCE || !grid.start.is_finite() || !grid.end.is_finite() {
            return Err(Refusal::ZeroLength);
        }
        let old = self.grids.get_mut(index).ok_or(Refusal::Missing)?;
        if old.locked && (old.name != grid.name || old.start != grid.start || old.end != grid.end) {
            return Err(Refusal::Locked);
        }
        *old = grid;
        Ok(())
    }

    /// Locks or unlocks grid line `index`.
    pub fn lock_grid(&mut self, index: usize, locked: bool) -> Result<(), Refusal> {
        self.grids.get_mut(index).ok_or(Refusal::Missing)?.locked = locked;
        Ok(())
    }

    /// Whether grid line `index` is locked; a missing one is not.
    pub fn grid_locked(&self, index: usize) -> bool {
        self.grids.get(index).is_some_and(|g| g.locked)
    }

    /// Removes grid line `index`; returns it. A locked line stays, and so does a missing one.
    pub fn remove_grid(&mut self, index: usize) -> Option<GridDef> {
        (index < self.grids.len() && !self.grids[index].locked).then(|| self.grids.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::TileGrid;

    fn p(x: f64, y: f64) -> DVec2 {
        DVec2::new(x, y)
    }

    #[test]
    fn a_new_grid_line_names_itself_by_the_way_it_runs() {
        let mut m = Model::new(TileGrid::default());
        // north-south: numbered; east-west and diagonal: lettered
        let names = |m: &Model| m.grids().iter().map(|g| g.name.clone()).collect::<Vec<_>>();
        m.add_grid(p(0.0, 0.0), p(0.0, 10.0)).unwrap();
        m.add_grid(p(5.0, 10.0), p(5.0, 0.0)).unwrap();
        m.add_grid(p(0.0, 0.0), p(10.0, 0.0)).unwrap();
        m.add_grid(p(0.0, 4.0), p(10.0, 4.0)).unwrap();
        m.add_grid(p(0.0, 0.0), p(10.0, 10.0)).unwrap();
        assert_eq!(names(&m), ["1", "2", "A", "B", "C"]);
        // a name given away is not given again while a later one stands
        m.remove_grid(0).unwrap();
        m.remove_grid(1).unwrap();
        m.add_grid(p(9.0, 0.0), p(9.0, 10.0)).unwrap();
        m.add_grid(p(0.0, 9.0), p(10.0, 9.0)).unwrap();
        assert_eq!(names(&m), ["2", "B", "C", "3", "D"]);
        // after Z comes A2
        m.insert_grid(GridDef { name: "Z".into(), start: p(0.0, 20.0), end: p(10.0, 20.0), locked: false }).unwrap();
        assert_eq!(m.next_grid_name(p(0.0, 0.0), p(1.0, 0.0)), "A2");
        m.insert_grid(GridDef { name: "Z2".into(), start: p(0.0, 21.0), end: p(10.0, 21.0), locked: false }).unwrap();
        assert_eq!(m.next_grid_name(p(0.0, 0.0), p(1.0, 0.0)), "A3");
        // names of one's own do not count
        m.insert_grid(GridDef { name: "Gevel".into(), start: p(0.0, 22.0), end: p(10.0, 22.0), locked: false }).unwrap();
        assert_eq!(m.next_grid_name(p(0.0, 0.0), p(1.0, 0.0)), "A3");
    }

    #[test]
    fn a_grid_line_needs_a_length_and_crosses_others_within_both() {
        let mut m = Model::new(TileGrid::default());
        assert_eq!(m.add_grid(p(1.0, 1.0), p(1.0, 1.0)), Err(Refusal::ZeroLength));
        let one = m.add_grid(p(2.0, -1.0), p(2.0, 6.0)).unwrap();
        let a = m.add_grid(p(-1.0, 3.0), p(8.0, 3.0)).unwrap();
        let short = m.add_grid(p(4.0, 5.0), p(8.0, 5.0)).unwrap();
        let g = m.grids();
        assert_eq!(g[one].crossing(&g[a]), Some(p(2.0, 3.0)));
        assert_eq!(g[one].crossing(&g[short]), None, "it ends before the other");
        assert_eq!(g[a].crossing(&g[short]), None, "parallel");
        // moved, renamed, removed
        m.set_grid(one, GridDef { name: "X".into(), start: p(3.0, -1.0), end: p(3.0, 6.0), locked: false }).unwrap();
        assert_eq!(m.grids()[one].name, "X");
        assert_eq!(m.set_grid(9, GridDef { name: "Y".into(), start: p(0.0, 0.0), end: p(1.0, 0.0), locked: false }), Err(Refusal::Missing));
        assert_eq!(m.remove_grid(one).map(|g| g.name), Some("X".into()));
        assert_eq!(m.grids().len(), 2);
    }

    #[test]
    fn a_model_keeps_its_grid_lines_when_written_and_read() {
        let mut m = Model::new(TileGrid::default());
        m.add_grid(p(0.0, 0.0), p(0.0, 10.0)).unwrap();
        let read: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(read, m);
        // a model written before grid lines were kept has none
        let old: Model = serde_json::from_str(&serde_json::to_string(&Model::new(TileGrid::default())).unwrap().replace(",\"grids\":[]", "")).unwrap();
        assert!(old.grids().is_empty());
    }

    #[test]
    fn a_locked_grid_line_keeps_its_name_and_ends_and_cannot_be_removed() {
        let mut m = Model::new(TileGrid::default());
        let a = m.add_grid(p(0.0, 0.0), p(0.0, 10.0)).unwrap();
        let b = m.add_grid(p(5.0, 0.0), p(5.0, 10.0)).unwrap();
        assert!(!m.grid_locked(a), "a new one is free");
        m.lock_grid(a, true).unwrap();
        assert!(m.grid_locked(a) && !m.grid_locked(b) && !m.grid_locked(9));
        let moved = GridDef { name: "1".into(), start: p(1.0, 0.0), end: p(1.0, 10.0), locked: true };
        assert_eq!(m.set_grid(a, moved.clone()), Err(Refusal::Locked));
        // a longer line, a new name and a free one with other ends are refused all the same
        assert_eq!(m.set_grid(a, GridDef { end: p(0.0, 12.0), ..m.grids()[a].clone() }), Err(Refusal::Locked));
        assert_eq!(m.set_grid(a, GridDef { name: "X".into(), ..m.grids()[a].clone() }), Err(Refusal::Locked));
        assert_eq!(m.set_grid(a, GridDef { locked: false, ..moved }), Err(Refusal::Locked), "unlocking is its own step");
        assert_eq!((m.grids()[a].start, m.grids()[a].end), (p(0.0, 0.0), p(0.0, 10.0)));
        assert!(m.remove_grid(a).is_none() && m.grids().len() == 2);
        // the other is free
        m.set_grid(b, GridDef { end: p(5.0, 12.0), ..m.grids()[b].clone() }).unwrap();
        // unlocked, it changes and goes
        m.lock_grid(a, false).unwrap();
        m.set_grid(a, GridDef { end: p(0.0, 12.0), ..m.grids()[a].clone() }).unwrap();
        assert_eq!(m.remove_grid(a).map(|g| g.name), Some("1".into()));
        assert_eq!(m.lock_grid(9, true), Err(Refusal::Missing));
    }

    #[test]
    fn the_lock_is_written_and_a_file_from_before_it_has_none_locked() {
        let mut m = Model::new(TileGrid::default());
        let a = m.add_grid(p(0.0, 0.0), p(0.0, 10.0)).unwrap();
        m.lock_grid(a, true).unwrap();
        let read: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert!(read.grid_locked(0));
        let line: GridDef = serde_json::from_str(r#"{"name":"A","start":[0.0,0.0],"end":[1.0,0.0]}"#).unwrap();
        assert!(!line.locked);
    }
}
