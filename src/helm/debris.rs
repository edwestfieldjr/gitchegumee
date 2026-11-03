//! Original pixel art for what floats on Lake Superior in a gale: pulpwood logs,
//! tangled pairs of them, whole trunks with their root wads, and broken timber.
//!
//! The wood stays inside the hazard's collision box, so what looks like a hit is
//! one. Only the wash of foam around it spills over the edge.

use crate::ui::palette::Pal;

/// One horizontal run of pixels, relative to the hazard's top left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub color: Pal,
}

/// A stable pseudo-random number for a spot on a piece of debris.
pub(super) fn roll(x: i32, y: i32, seed: u32) -> u32 {
    let mixed = (x as u32).wrapping_mul(73) ^ (y as u32).wrapping_mul(179) ^ seed;
    mixed.wrapping_mul(2_654_435_761) >> 16 & 0xFFFF
}

struct Grid {
    w: i32,
    h: i32,
    cells: Vec<Option<Pal>>,
}

impl Grid {
    fn new(w: i32, h: i32) -> Self {
        Self {
            w,
            h,
            cells: vec![None; (w * h) as usize],
        }
    }

    fn set(&mut self, x: i32, y: i32, color: Option<Pal>) {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            self.cells[(y * self.w + x) as usize] = color;
        }
    }

    fn paint(&mut self, x: i32, y: i32, color: Pal) {
        self.set(x, y, Some(color));
    }

    fn get(&self, x: i32, y: i32) -> Option<Pal> {
        self.cells[(y * self.w + x) as usize]
    }

    /// Merge the grid's rows into runs, which saves a sprite for each pixel.
    fn spans(&self) -> Vec<Span> {
        let mut spans = Vec::new();
        for y in 0..self.h {
            let mut x = 0;
            while x < self.w {
                let Some(color) = self.get(x, y) else {
                    x += 1;
                    continue;
                };
                let start = x;
                while x < self.w && self.get(x, y) == Some(color) {
                    x += 1;
                }
                spans.push(Span {
                    x: start,
                    y,
                    w: x - start,
                    color,
                });
            }
        }
        spans
    }
}

/// A round waterlogged log, lit along the top, dark at the waterline, with flecks
/// of reddish bark and a pale sawn end on the right.
fn draw_log(grid: &mut Grid, left: i32, top: i32, w: i32, h: i32, seed: u32) {
    let (right, bottom) = (left + w - 1, top + h - 1);
    for y in top..=bottom {
        for x in left..=right {
            let color = if y == top {
                Pal::Sand
            } else if y == bottom {
                Pal::Black
            } else if roll(x, y, seed).is_multiple_of(4) {
                Pal::Rust
            } else {
                Pal::Slate
            };
            grid.paint(x, y, color);
        }
    }
    for y in top..=bottom {
        grid.paint(right, y, Pal::Amber);
    }
    if h >= 3 {
        grid.paint(right, top + h / 2, Pal::Yellow);
    }
    // Round off the corners.
    grid.set(left, top, None);
    grid.set(left, bottom, None);
    grid.set(right, top, None);
    grid.set(right, bottom, None);
}

/// Two short logs that have drifted together.
fn draw_tangle(grid: &mut Grid, w: i32, h: i32, seed: u32) {
    let upper_h = h / 2;
    let lower_h = h - upper_h;
    let overlap = (w * 2 / 3).max(4);
    draw_log(grid, 0, 0, overlap, upper_h.max(2), seed);
    draw_log(
        grid,
        w - overlap,
        h - lower_h,
        overlap,
        lower_h,
        seed ^ 0x55,
    );
}

/// A whole trunk, torn from a shore: a tangled root wad at the left and the
/// stubs of snapped branches along the top.
fn draw_trunk(grid: &mut Grid, w: i32, h: i32, seed: u32) {
    const ROOT_W: i32 = 3;
    draw_log(grid, ROOT_W - 1, 1, w - ROOT_W + 1, h - 1, seed);
    for y in 0..h {
        for x in 0..ROOT_W {
            let color = match roll(x, y, seed) % 4 {
                0 => None,
                1 => Some(Pal::Slate),
                2 => Some(Pal::Sand),
                _ => Some(Pal::Black),
            };
            grid.set(x, y, color);
        }
    }
    let mut x = ROOT_W + 2 + (roll(1, 1, seed) % 3) as i32;
    while x < w - 2 {
        grid.paint(x, 0, Pal::Slate);
        x += 3 + (roll(x, 2, seed) % 4) as i32;
    }
}

/// Weathered timber, snapped in two, with splintered ends and rusted fastenings.
fn draw_timber(grid: &mut Grid, w: i32, h: i32, seed: u32) {
    let top = (h - 2) / 2;
    let break_at = w / 3 + (roll(2, 2, seed) % (w as u32 / 3).max(1)) as i32;
    for x in 0..w {
        // The far part sits a pixel lower, where it has broken and swung away.
        let drop = i32::from(x > break_at && h > 2);
        grid.paint(x, top + drop, Pal::Gray);
        grid.paint(x, top + drop + 1, Pal::Slate);
    }
    grid.set(break_at, top, None);
    grid.set(break_at, top + 1, None);
    for x in [0, w - 1] {
        for y in 0..h {
            if roll(x, y, seed).is_multiple_of(2) {
                grid.set(x, y, None);
            }
        }
    }
    let mut x = 2 + (roll(3, 3, seed) % 3) as i32;
    while x < w - 2 {
        if grid.get(x, top).is_some() {
            grid.paint(x, top, Pal::Rust);
        }
        x += 5;
    }
}

/// White water where the sea breaks against the wood. It is only a wash, so it
/// may spill a pixel past the collision box.
fn foam(w: i32, h: i32, seed: u32) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut x = (roll(4, 4, seed) % 3) as i32;
    while x < w {
        let long = 1 + (roll(x, 5, seed) % 3) as i32;
        spans.push(Span {
            x,
            y: h,
            w: long.min(w - x),
            color: Pal::Foam,
        });
        x += long + 2 + (roll(x, 6, seed) % 4) as i32;
    }
    for (x, y) in [(-1, h / 2), (w, h / 2 + (seed % 2) as i32)] {
        spans.push(Span {
            x,
            y,
            w: 1,
            color: Pal::White,
        });
    }
    spans
}

/// The art for a piece of debris of a size. The seed picks what it is and every
/// detail of it, so the same run always draws the same debris.
pub fn debris_art(w: i32, h: i32, seed: u32) -> Vec<Span> {
    let mut grid = Grid::new(w, h);
    let choice = roll(7, 7, seed) % 4;
    match choice {
        1 if h >= 4 => draw_tangle(&mut grid, w, h, seed),
        2 if h >= 4 => draw_trunk(&mut grid, w, h, seed),
        3 => draw_timber(&mut grid, w, h, seed),
        _ => draw_log(&mut grid, 0, 0, w, h, seed),
    }
    let mut spans = grid.spans();
    spans.extend(foam(w, h, seed));
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_wood(span: &Span) -> bool {
        !matches!(span.color, Pal::Foam | Pal::White)
    }

    /// Every size and many seeds the helm can ask for.
    fn every_piece() -> impl Iterator<Item = (i32, i32, u32)> {
        (10..=18)
            .flat_map(|w| (3..=5).flat_map(move |h| (0..200).map(move |seed| (w, h, seed * 37))))
    }

    #[test]
    fn the_wood_stays_inside_the_collision_box() {
        for (w, h, seed) in every_piece() {
            for span in debris_art(w, h, seed).iter().filter(|s| is_wood(s)) {
                assert!(span.x >= 0 && span.x + span.w <= w, "{w}x{h} #{seed}");
                assert!(span.y >= 0 && span.y < h, "{w}x{h} #{seed}");
            }
        }
    }

    #[test]
    fn the_wood_nearly_fills_the_width_it_can_be_hit_across() {
        for (w, h, seed) in every_piece() {
            let art = debris_art(w, h, seed);
            let wood = art.iter().filter(|span| is_wood(span));
            let left = wood.clone().map(|span| span.x).min().unwrap();
            let right = wood.map(|span| span.x + span.w).max().unwrap();
            assert!(left <= 1 && right >= w - 1, "{w}x{h} #{seed}");
        }
    }

    #[test]
    fn the_same_seed_draws_the_same_piece() {
        assert_eq!(debris_art(16, 5, 42), debris_art(16, 5, 42));
    }

    #[test]
    fn the_pieces_vary() {
        let colors = |seed| debris_art(18, 5, seed);
        assert!((1..20).any(|seed| colors(seed) != colors(0)));
    }

    #[test]
    fn every_kind_of_debris_turns_up() {
        let pieces: Vec<Vec<Span>> = (0..200).map(|seed| debris_art(18, 5, seed * 37)).collect();
        let has = |color| {
            pieces
                .iter()
                .any(|piece| piece.iter().any(|span| span.color == color))
        };
        assert!(has(Pal::Amber), "no sawn log ends");
        assert!(has(Pal::Gray), "no timber");
    }

    #[test]
    fn the_shortest_debris_still_draws() {
        assert!(debris_art(10, 3, 3).iter().any(is_wood));
    }
}
