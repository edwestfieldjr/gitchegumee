//! Original pixel art for shoal water: broken surf around the edge, then water
//! shoaling from deep to pale, then sand and rock at the heart of it.
//!
//! The art covers most of the hazard's collision box. Its outline is uneven and
//! its corners round off, but only by a few pixels, so a near miss stays a fair call.

use super::debris::roll;
use crate::ui::palette::Pal;

/// Spacing of the coarse grid behind the shoal's uneven outline.
const NOISE_CELL: i32 = 9;
/// How far the bands wander from a smooth outline, as a share of the depth scale.
const NOISE_REACH: f32 = 0.16;
/// Share of the bands' wander that the outermost rim gets.
const RIM_WANDER: f32 = 0.5;
/// Higher squares off the outline, leaving less of the box uncovered.
const OUTLINE_POWER: f32 = 4.5;
/// The usual 4x4 ordered-dither matrix.
const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
/// How far in from the edge each band begins, from 0 at the edge to 1 at the heart.
const SHALLOW_AT: f32 = 0.05;
const PALE_AT: f32 = 0.24;
const SAND_AT: f32 = 0.5;

/// Whether ordered dithering keeps a pixel at a share from 0 to 1.
fn dither(x: i32, y: i32, share: f32) -> bool {
    let threshold = BAYER[y.rem_euclid(4) as usize][x.rem_euclid(4) as usize];
    (threshold as f32 + 0.5) / 16.0 < share
}

/// Smooth noise from -1 to 1, stable for a seed.
fn wander(x: i32, y: i32, seed: u32) -> f32 {
    let corner = |cx: i32, cy: i32| (roll(cx, cy, seed) % 1000) as f32 / 500.0 - 1.0;
    let (cell_x, cell_y) = (x.div_euclid(NOISE_CELL), y.div_euclid(NOISE_CELL));
    let tx = x.rem_euclid(NOISE_CELL) as f32 / NOISE_CELL as f32;
    let ty = y.rem_euclid(NOISE_CELL) as f32 / NOISE_CELL as f32;
    let top = corner(cell_x, cell_y) * (1.0 - tx) + corner(cell_x + 1, cell_y) * tx;
    let bottom = corner(cell_x, cell_y + 1) * (1.0 - tx) + corner(cell_x + 1, cell_y + 1) * tx;
    top * (1.0 - ty) + bottom * ty
}

/// How far a pixel is inside the shoal: below 0 is outside it, and 1 is the heart.
fn depth_in(x: i32, y: i32, w: i32, h: i32, seed: u32) -> f32 {
    let across = ((x as f32 + 0.5) / w as f32 * 2.0 - 1.0).abs();
    let down = ((y as f32 + 0.5) / h as f32 * 2.0 - 1.0).abs();
    let outline = (across.powf(OUTLINE_POWER) + down.powf(OUTLINE_POWER)).powf(1.0 / OUTLINE_POWER);
    // The rim wanders less than the bands inside it, to stay close to the box.
    let inward = (1.0 - outline).clamp(0.0, 1.0);
    let reach = NOISE_REACH * (RIM_WANDER + inward * 3.0).min(1.0);
    1.0 - outline + wander(x, y, seed) * reach
}

/// The color of one pixel, or `None` for open water.
fn shoal_pixel(x: i32, y: i32, w: i32, h: i32, seed: u32) -> Option<Pal> {
    let depth = depth_in(x, y, w, h, seed);
    if depth < 0.0 {
        return None;
    }
    let speck = roll(x, y, seed ^ 0x51).is_multiple_of(9);
    Some(if depth < SHALLOW_AT {
        // Surf, broken up where it runs against the reef.
        if speck {
            Pal::White
        } else if roll(x / 2, y / 2, seed).is_multiple_of(3) {
            Pal::Lake
        } else {
            Pal::Foam
        }
    } else if depth < PALE_AT {
        let share = (depth - SHALLOW_AT) / (PALE_AT - SHALLOW_AT);
        if dither(x, y, share) {
            Pal::Sky
        } else {
            Pal::Lake
        }
    } else if depth < SAND_AT {
        let share = (depth - PALE_AT) / (SAND_AT - PALE_AT);
        if dither(x, y, share * share) {
            Pal::Sand
        } else if speck {
            Pal::Foam
        } else {
            Pal::Sky
        }
    } else if roll(x / 3, y / 2, seed ^ 0x2A).is_multiple_of(7) {
        // Rock breaking the sand, lit along the top and dark at the foot.
        if y.rem_euclid(2) == 0 {
            Pal::Slate
        } else {
            Pal::Black
        }
    } else if speck {
        Pal::Gray
    } else {
        Pal::Sand
    })
}

/// A shoal's pixels in rows. The seed fixes every detail, so the same run always
/// draws the same shoal.
pub fn shoal_art(w: i32, h: i32, seed: u32) -> Vec<Option<Pal>> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| shoal_pixel(x, y, w, h, seed))
        .collect()
}

/// A shoal as RGBA8 pixels for an image, clear where there is open water.
pub fn shoal_rgba(w: i32, h: i32, seed: u32) -> Vec<u8> {
    shoal_art(w, h, seed)
        .into_iter()
        .flat_map(|pixel| match pixel {
            Some(color) => {
                let [r, g, b] = color.rgb();
                [r, g, b, 255]
            }
            None => [0, 0, 0, 0],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest and largest shoals the helm asks for, and some between.
    const SIZES: [(i32, i32); 4] = [(30, 24), (70, 48), (30, 48), (70, 24)];

    #[test]
    fn the_art_is_one_pixel_for_each_of_the_box() {
        for (w, h) in SIZES {
            assert_eq!(shoal_art(w, h, 7).len(), (w * h) as usize);
            assert_eq!(shoal_rgba(w, h, 7).len(), (w * h * 4) as usize);
        }
    }

    /// The most pixels the outline may draw in from the box at the middle of a side.
    const SIDE_ROOM: i32 = 4;

    #[test]
    fn the_shoal_comes_close_to_every_side_of_its_box() {
        for (w, h) in SIZES {
            for seed in 0..50 {
                let art = shoal_art(w, h, seed * 91);
                let at = |x: i32, y: i32| art[(y * w + x) as usize];
                let near = |mut spots: std::ops::Range<i32>, spot: &dyn Fn(i32) -> Option<Pal>| {
                    spots.any(|step| spot(step).is_some())
                };
                let room = 0..SIDE_ROOM;
                assert!(near(room.clone(), &|i| at(i, h / 2)), "{w}x{h} #{seed}");
                assert!(
                    near(room.clone(), &|i| at(w - 1 - i, h / 2)),
                    "{w}x{h} #{seed}"
                );
                assert!(near(room.clone(), &|i| at(w / 2, i)), "{w}x{h} #{seed}");
                assert!(near(room, &|i| at(w / 2, h - 1 - i)), "{w}x{h} #{seed}");
            }
        }
    }

    #[test]
    fn the_shoal_covers_most_of_the_box_it_can_be_hit_in() {
        for (w, h) in SIZES {
            for seed in 0..50 {
                let art = shoal_art(w, h, seed * 91);
                let covered = art.iter().filter(|pixel| pixel.is_some()).count();
                assert!(covered * 100 >= art.len() * 80, "{w}x{h} #{seed}");
            }
        }
    }

    #[test]
    fn surf_rings_the_edge_and_sand_lies_at_the_heart() {
        for (w, h) in SIZES {
            let art = shoal_art(w, h, 11);
            let at = |x: i32, y: i32| art[(y * w + x) as usize];
            let rim = (0..w).find_map(|x| at(x, h / 2));
            assert!(matches!(rim, Some(Pal::Foam | Pal::White | Pal::Lake)));
            assert!(matches!(
                at(w / 2, h / 2),
                Some(Pal::Sand | Pal::Slate | Pal::Black | Pal::Gray)
            ));
        }
    }

    #[test]
    fn the_same_seed_draws_the_same_shoal_and_others_differ() {
        assert_eq!(shoal_art(50, 30, 3), shoal_art(50, 30, 3));
        assert_ne!(shoal_art(50, 30, 3), shoal_art(50, 30, 4));
    }
}
