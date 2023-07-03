//! Built-in layouts.
use crate::{
    builtin::layout::messages::{ExpandMain, IncMain, Mirror, Rotate, ShrinkMain},
    core::layout::{Layout, Message},
    pure::{geometry::Rect, Stack},
    Xid,
};

pub mod messages;
pub mod transformers;

#[derive(Debug, Clone, Copy)]
enum StackPosition {
    Side,
    Bottom,
}

/// A simple [Layout] with main and secondary regions.
///
/// - `MainAndStack::side` give a main region to the left and remaining clients to the right.
/// - `MainAndStack::bottom` give a main region to the top and remaining clients to the bottom.
///
/// The ratio between the main and secondary stack regions can be adjusted by sending [ShrinkMain]
/// and [ExpandMain] messages to this layout. The number of clients in the main area can be
/// increased or decreased by sending an [IncMain] message. To flip between the side and bottom
/// behaviours you can send a [Rotate] message.
///
/// ```text
/// ..................................
/// .                  .             .
/// .                  .             .
/// .                  .             .
/// .                  ...............
/// .                  .             .
/// .                  .             .
/// .                  .             .
/// .                  ...............
/// .                  .             .
/// .                  .             .
/// .                  .             .
/// ..................................
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MainAndStack {
    pos: StackPosition,
    max_main: u32,
    ratio: f32,
    ratio_step: f32,
    mirrored: bool,
}

impl MainAndStack {
    /// Create a new default [MainAndStack] [Layout] as a trait object ready to be added to your
    /// [LayoutStack][crate::core::layout::LayoutStack].
    pub fn boxed_default() -> Box<dyn Layout> {
        Box::<Self>::default()
    }

    /// Create a new [MainAndStack] [Layout] with the main area on the left and remaining windows
    /// stacked to the right.
    pub fn side(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::side_unboxed(max_main, ratio, ratio_step, false))
    }

    /// Create a new [MainAndStack] [Layout] with the main area on the right and remaining windows
    /// stacked to the left.
    pub fn side_mirrored(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::side_unboxed(max_main, ratio, ratio_step, true))
    }

    /// Create a new [MainAndStack] [Layout] with the main area and remaining windows
    /// stacked to the side.
    pub fn side_unboxed(max_main: u32, ratio: f32, ratio_step: f32, mirrored: bool) -> Self {
        Self {
            pos: StackPosition::Side,
            max_main,
            ratio,
            ratio_step,
            mirrored,
        }
    }

    /// Create a new [MainAndStack] [Layout] with the main area on the top and remaining windows
    /// stacked on the bottom.
    pub fn bottom(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::bottom_unboxed(max_main, ratio, ratio_step, false))
    }

    /// Create a new [MainAndStack] [Layout] with the main area on the bottom and remaining windows
    /// stacked on the top.
    pub fn top(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::bottom_unboxed(max_main, ratio, ratio_step, true))
    }

    /// Create a new [MainAndStack] [Layout] with a main area and the remaining windows
    /// stacked either on the top or the bottom.
    pub fn bottom_unboxed(max_main: u32, ratio: f32, ratio_step: f32, mirrored: bool) -> Self {
        Self {
            pos: StackPosition::Bottom,
            max_main,
            ratio,
            ratio_step,
            mirrored,
        }
    }

    fn split(&self, d: u32) -> u32 {
        let ratio = if self.mirrored {
            1.0 - self.ratio
        } else {
            self.ratio
        };

        ((d as f32) * ratio) as u32
    }

    // In each of these four cases we no longer have a split point giving
    // us two independent stacks.
    fn all_windows_in_single_stack(&self, n: u32) -> bool {
        n <= self.max_main || self.max_main == 0 || self.ratio == 1.0 || self.ratio == 0.0
    }

    fn layout_side(&self, s: &Stack<Xid>, r: Rect) -> Vec<(Xid, Rect)> {
        let n = s.len() as u32;

        if self.all_windows_in_single_stack(n) {
            r.as_rows(n).iter().zip(s).map(|(r, c)| (*c, *r)).collect()
        } else {
            // We have two stacks so split the screen in two and then build a stack for each
            let split = self.split(r.w);
            let (mut main, mut stack) = r.split_at_width(split).expect("split point to be valid");
            if self.mirrored {
                (main, stack) = (stack, main);
            }

            main.as_rows(self.max_main)
                .into_iter()
                .chain(stack.as_rows(n.saturating_sub(self.max_main)))
                .zip(s)
                .map(|(r, c)| (*c, r))
                .collect()
        }
    }

    fn layout_bottom(&self, s: &Stack<Xid>, r: Rect) -> Vec<(Xid, Rect)> {
        let n = s.len() as u32;

        if self.all_windows_in_single_stack(n) {
            r.as_columns(n)
                .iter()
                .zip(s)
                .map(|(r, c)| (*c, *r))
                .collect()
        } else {
            let split = self.split(r.h);
            let (mut main, mut stack) = r.split_at_height(split).expect("split point to be valid");
            if self.mirrored {
                (main, stack) = (stack, main);
            }

            main.as_columns(self.max_main)
                .into_iter()
                .chain(stack.as_columns(n.saturating_sub(self.max_main)))
                .zip(s)
                .map(|(r, c)| (*c, r))
                .collect()
        }
    }
}

impl Default for MainAndStack {
    fn default() -> Self {
        Self {
            pos: StackPosition::Side,
            max_main: 1,
            ratio: 0.6,
            ratio_step: 0.1,
            mirrored: false,
        }
    }
}

impl Layout for MainAndStack {
    fn name(&self) -> String {
        match (self.pos, self.mirrored) {
            (StackPosition::Side, false) => "Side".to_owned(),
            (StackPosition::Side, true) => "Mirror".to_owned(),
            (StackPosition::Bottom, false) => "Bottom".to_owned(),
            (StackPosition::Bottom, true) => "Top".to_owned(),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(*self)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        let positions = match self.pos {
            StackPosition::Side => self.layout_side(s, r),
            StackPosition::Bottom => self.layout_bottom(s, r),
        };

        (None, positions)
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        if let Some(&ExpandMain) = m.downcast_ref() {
            self.ratio += self.ratio_step;
            if self.ratio > 1.0 {
                self.ratio = 1.0;
            }
        } else if let Some(&ShrinkMain) = m.downcast_ref() {
            self.ratio -= self.ratio_step;
            if self.ratio < 0.0 {
                self.ratio = 0.0;
            }
        } else if let Some(&IncMain(n)) = m.downcast_ref() {
            if n < 0 {
                self.max_main = self.max_main.saturating_sub((-n) as u32);
            } else {
                self.max_main += n as u32;
            }
        } else if let Some(&Mirror) = m.downcast_ref() {
            self.mirrored = !self.mirrored;
        } else if let Some(&Rotate) = m.downcast_ref() {
            self.pos = match self.pos {
                StackPosition::Side => StackPosition::Bottom,
                StackPosition::Bottom => StackPosition::Side,
            };
        }

        None
    }
}

/// A simple monolce layout that gives the maximum available space to the currently
/// focused client and unmaps all other windows.
///
/// ```text
/// ..................................
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// .                                .
/// ..................................
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Monocle;

impl Monocle {
    /// Create a new [Monocle] [Layout] as a boxed trait object
    pub fn boxed() -> Box<dyn Layout> {
        Box::new(Monocle)
    }
}

impl Layout for Monocle {
    fn name(&self) -> String {
        "Mono".to_owned()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Self::boxed()
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        (None, vec![(s.focus, r)])
    }

    fn handle_message(&mut self, _: &Message) -> Option<Box<dyn Layout>> {
        None
    }
}

/// A simple grid layout that places windows in the smallest nxn grid that will
/// contain all window present on the workspace.
///
/// ```text
/// ..................................
/// .          .          .          .
/// .          .          .          .
/// .          .          .          .
/// ..................................
/// .          .          .          .
/// .          .          .          .
/// .          .          .          .
/// ..................................
/// .          .          .          .
/// .          .          .          .
/// .          .          .          .
/// ..................................
/// ```
///
/// ### NOTE
/// This will leave unused screen space if there are not a square number of
/// windows present in the workspace being laid out:
/// ```text
/// ..................................
/// .          .          .          .
/// .          .          .          .
/// .          .          .          .
/// ..................................
/// .          .          .          .
/// .          .          .          .
/// .          .          .          .
/// ..................................
/// .          .          .
/// .          .          .
/// .          .          .
/// .......................
/// ```
#[derive(Debug, Default, Copy, Clone)]
pub struct Grid;

impl Grid {
    /// Create a new [Grid] [Layout] as a boxed trait object
    pub fn boxed() -> Box<dyn Layout> {
        Box::new(Grid)
    }
}

impl Layout for Grid {
    fn name(&self) -> String {
        "Grid".to_string()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Self::boxed()
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        let n = s.len();
        let n_cols = (1..).find(|&i| (i * i) >= n).unwrap_or(1);
        let n_rows = if n_cols * (n_cols - 1) >= n {
            n_cols - 1
        } else {
            n_cols
        };

        let rects = r
            .as_rows(n_rows as u32)
            .into_iter()
            .flat_map(|row| row.as_columns(n_cols as u32));

        let positions = s.iter().zip(rects).map(|(&id, r)| (id, r)).collect();

        (None, positions)
    }

    fn handle_message(&mut self, _: &Message) -> Option<Box<dyn Layout>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        builtin::layout::{messages::IncMain, *},
        core::layout::IntoMessage,
    };

    #[test]
    fn message_handling() {
        let mut l = MainAndStack::side_unboxed(1, 0.6, 0.1, false);

        l.handle_message(&IncMain(2).into_message());

        assert_eq!(l.max_main, 3);
    }
}
