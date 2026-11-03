//! Detroit at night from the river, as one piece of original pixel art.
//!
//! The skyline's shapes are traced by eye from a 1965 view, before the
//! Renaissance Center, and Mariners' Church stands on the shore in front of it.
//! The art is drawn here in palette colors. No photo is used at runtime.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::ui::palette::Pal;
use crate::ui::{VIEW_H, VIEW_W, at, rect, z};

/// Where the buildings stand, and where the river begins.
const BASE: i32 = 106;
pub const RIVER_Y: i32 = 112;

/// One block of a building: left, width, height above `BASE`, the color of its
/// walls and windows, and what share of its windows are lit.
struct Block(i32, i32, i32, Pal, Pal, u32);

const SKYLINE: [Block; 25] = [
    // The low hall and tall slab at the left of the view.
    Block(0, 27, 40, Pal::Black, Pal::Amber, 35),
    Block(0, 57, 23, Pal::Night, Pal::Sky, 15),
    Block(49, 16, 55, Pal::Black, Pal::Sky, 30),
    Block(65, 35, 25, Pal::Black, Pal::Amber, 45),
    // The stepped tower with the peaked top.
    Block(78, 27, 44, Pal::Black, Pal::Amber, 30),
    Block(84, 15, 50, Pal::Black, Pal::Amber, 25),
    Block(111, 12, 45, Pal::Black, Pal::Amber, 25),
    // The tallest tower, in setbacks up to its red light.
    Block(123, 29, 53, Pal::Black, Pal::Sand, 40),
    Block(127, 16, 65, Pal::Black, Pal::Sand, 35),
    Block(129, 12, 74, Pal::Black, Pal::Sand, 30),
    Block(132, 6, 82, Pal::Black, Pal::Sand, 20),
    Block(152, 12, 41, Pal::Black, Pal::Amber, 30),
    Block(140, 21, 23, Pal::Night, Pal::Sky, 70),
    // The dark tower behind the bright slab.
    Block(164, 13, 74, Pal::Black, Pal::Amber, 15),
    // The bright slab, floodlit at every window.
    Block(171, 25, 65, Pal::Slate, Pal::Foam, 100),
    Block(196, 5, 44, Pal::Black, Pal::Red, 40),
    Block(201, 10, 47, Pal::Black, Pal::Amber, 35),
    Block(211, 11, 25, Pal::Night, Pal::Amber, 80),
    Block(222, 9, 48, Pal::Black, Pal::Amber, 35),
    // The grid-fronted tower, with its mast.
    Block(231, 29, 55, Pal::Slate, Pal::White, 75),
    Block(260, 4, 25, Pal::Black, Pal::Amber, 30),
    // The long low building, and the dark box in front of it.
    Block(264, 42, 33, Pal::Slate, Pal::Foam, 70),
    Block(266, 28, 20, Pal::Black, Pal::Black, 0),
    Block(306, 14, 18, Pal::Black, Pal::Amber, 25),
    Block(294, 12, 12, Pal::Night, Pal::Amber, 40),
];

const TOWER_LIGHT_X: i32 = 135;
const MAST_X: i32 = 251;
/// The lit sign on top of the bright slab: left, width, height.
const SIGN: (i32, i32, i32) = (178, 12, 9);

/// Church position. Its tower is drawn about 1.5x true scale against the
/// skyline, for being on the near shore and so that it can be recognized.
const CHURCH_X: i32 = 205;
const CHURCH_BASE: i32 = 110;
const TOWER_W: i32 = 8;
const TOWER_H: i32 = 22;
const LINK_W: i32 = 3;
const NAVE_W: i32 = 17;
const GABLE_H: i32 = 15;
const STONE: Pal = Pal::Sand;
const SHADOW: Pal = Pal::Slate;
/// A reflection stretches on the water: this many rows for each 4 church rows.
const REFLECTION_STRETCH: i32 = 5;
/// Rows of water the church's reflection covers, down from the river's edge.
const REFLECTION_ROWS: i32 = 30;
/// Rows at the river's edge that show the reflection almost whole.
const REFLECTION_SOLID_ROWS: i32 = 4;
/// Share of the reflection's pixels still lit at its far end.
const REFLECTION_FAR_SHARE: i32 = 25;
/// Sideways ripple, in pixels, by row.
const RIPPLE: [i32; 8] = [0, 1, 1, 0, 0, -1, -1, 0];
/// The usual 4x4 ordered-dither matrix.
const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

struct Canvas {
    pixels: Vec<Pal>,
}

impl Canvas {
    fn new() -> Self {
        Self {
            pixels: vec![Pal::Night; (VIEW_W * VIEW_H) as usize],
        }
    }

    fn set(&mut self, x: i32, y: i32, color: Pal) {
        if (0..VIEW_W as i32).contains(&x) && (0..VIEW_H as i32).contains(&y) {
            self.pixels[y as usize * VIEW_W as usize + x as usize] = color;
        }
    }

    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, color: Pal) {
        for row in y..y + h {
            for column in x..x + w {
                self.set(column, row, color);
            }
        }
    }

    fn get(&self, x: i32, y: i32) -> Pal {
        self.pixels[y as usize * VIEW_W as usize + x as usize]
    }
}

/// A stable pseudo-random roll from 0 to 99 for a spot on the canvas.
fn roll(x: i32, y: i32) -> u32 {
    let mixed = (x as u32).wrapping_mul(31) ^ (y as u32).wrapping_mul(17);
    mixed.wrapping_mul(2_654_435_761) >> 16 & 0xFFFF
}

fn window_lit(x: i32, y: i32, share: u32) -> bool {
    roll(x, y) % 100 < share
}

fn draw_sky(canvas: &mut Canvas) {
    // The glow of the city, dithered up into the night.
    for y in 76..BASE {
        for x in 0..VIEW_W as i32 {
            let step = (y - 76) / 8;
            let on = match step {
                0 => x % 4 == 0 && y % 2 == 0,
                1 => (x + y) % 2 == 0,
                _ => true,
            };
            if on {
                canvas.set(x, y, Pal::Deep);
            }
        }
    }
}

fn draw_skyline(canvas: &mut Canvas) {
    for Block(left, width, height, wall, window, share) in &SKYLINE {
        let top = BASE - height;
        canvas.fill(*left, top, *width, *height, *wall);
        let (step_x, step_y) = if *share >= 70 { (2, 2) } else { (3, 3) };
        let mut y = top + 1;
        while y < BASE - 1 {
            let mut x = left + 1;
            while x < left + width - 1 {
                if window_lit(x, y, *share) {
                    canvas.set(x, y, *window);
                }
                x += step_x;
            }
            y += step_y;
        }
    }
    // The tallest tower's mast and red light.
    canvas.fill(TOWER_LIGHT_X - 1, BASE - 85, 2, 3, Pal::Black);
    canvas.fill(TOWER_LIGHT_X - 1, BASE - 89, 2, 4, Pal::Red);
    // The pointed top of the dark tower.
    for step in 0..6 {
        canvas.fill(
            170 - (5 - step) / 2,
            BASE - 74 - 5 + step,
            1 + (5 - step),
            1,
            Pal::Black,
        );
    }
    canvas.fill(MAST_X, BASE - 74, 1, 19, Pal::Black);
    canvas.set(MAST_X, BASE - 75, Pal::Red);
    let (x, w, h) = SIGN;
    canvas.fill(x, BASE - 74, w, h, Pal::Sky);
}

fn draw_shore(canvas: &mut Canvas) {
    canvas.fill(0, BASE, VIEW_W as i32, RIVER_Y - BASE, Pal::Black);
    let mut x = 2;
    while x < VIEW_W as i32 {
        if window_lit(x, BASE, 55) {
            let color = if roll(x, 1).is_multiple_of(3) {
                Pal::Sky
            } else {
                Pal::White
            };
            canvas.set(x, BASE + 1, color);
        }
        x += 4;
    }
}

/// Mariners' Church: a square tower with two belfry slits, a low link, and a
/// gabled front with a rose window, an arched door and a cross on the peak.
fn draw_church(canvas: &mut Canvas) {
    let base = CHURCH_BASE;
    let tower_top = base - TOWER_H;

    canvas.fill(CHURCH_X, tower_top, TOWER_W, TOWER_H, STONE);
    canvas.fill(CHURCH_X + TOWER_W - 2, tower_top, 2, TOWER_H, SHADOW);
    for x in [0, 2, 4, 6] {
        canvas.set(CHURCH_X + x, tower_top - 1, STONE);
    }
    canvas.fill(CHURCH_X, tower_top + 8, TOWER_W, 1, SHADOW);
    for (x, y, h) in belfry_slits() {
        canvas.fill(x, y, 1, h, Pal::Black);
    }
    canvas.fill(CHURCH_X + 3, base - 6, 1, 4, Pal::Black);

    let link = CHURCH_X + TOWER_W;
    canvas.fill(link, base - 6, LINK_W, 6, SHADOW);

    let nave = link + LINK_W;
    let center = nave + NAVE_W / 2;
    for row in 0..GABLE_H {
        let half = (row + 1).min(NAVE_W / 2);
        canvas.fill(center - half, base - GABLE_H + row, half * 2 + 1, 1, STONE);
    }
    canvas.fill(center - 1, base - GABLE_H - 3, 1, 1, Pal::Gray);
    canvas.fill(center, base - GABLE_H - 4, 1, 4, Pal::Gray);
    canvas.fill(center + 1, base - GABLE_H - 3, 1, 1, Pal::Gray);
    // The turret at the left of the front.
    canvas.fill(nave, base - 11, 2, 11, SHADOW);
    // Rose window and door, lit from inside.
    canvas.fill(center - 1, base - 11, 3, 3, Pal::Sky);
    canvas.set(center, base - 10, Pal::White);
    canvas.fill(center - 1, base - 4, 3, 4, Pal::Black);
    canvas.fill(center - 3, base, 7, 1, Pal::Gray);
}

/// (x, y, height) of the two slits in the belfry. The bell lights them.
fn belfry_slits() -> [(i32, i32, i32); 2] {
    let top = CHURCH_BASE - TOWER_H + 2;
    [(CHURCH_X + 2, top, 4), (CHURCH_X + 5, top, 4)]
}

fn draw_river(canvas: &mut Canvas) {
    canvas.fill(
        0,
        RIVER_Y,
        VIEW_W as i32,
        VIEW_H as i32 - RIVER_Y,
        Pal::Deep,
    );
    // The city's lights, broken up on the water.
    for x in 0..VIEW_W as i32 {
        let above = (0..BASE)
            .filter(|y| {
                matches!(
                    canvas.get(x, *y),
                    Pal::Foam | Pal::White | Pal::Amber | Pal::Sand
                )
            })
            .count() as i32;
        let reach = (above * 2).min(46);
        let color = if above > 12 { Pal::Foam } else { Pal::Sky };
        let mut y = RIVER_Y;
        while y < RIVER_Y + reach {
            if window_lit(x, y, 45) {
                canvas.set(x, y, color);
            }
            y += 2;
        }
    }
    let mut y = RIVER_Y + 4;
    while y < VIEW_H as i32 {
        let mut x = roll(y, 3) as i32 % 40;
        while x < VIEW_W as i32 {
            if canvas.get(x, y) == Pal::Deep {
                canvas.fill(x, y, 4 + roll(x, y) as i32 % 12, 1, Pal::Lake);
            }
            x += 60 + roll(x, y) as i32 % 60;
        }
        y += 9;
    }
}

/// Share of the reflection's pixels lit, by rows of water from the river's edge.
fn reflection_share(rows: i32) -> i32 {
    let fading = (REFLECTION_ROWS - REFLECTION_SOLID_ROWS).max(1);
    let past = (rows - REFLECTION_SOLID_ROWS).clamp(0, fading);
    100 - (100 - REFLECTION_FAR_SHARE) * past / fading
}

/// Whether ordered dithering keeps a pixel at a share from 0 to 100.
fn dither_on(x: i32, y: i32, share: i32) -> bool {
    let threshold = BAYER[y.rem_euclid(4) as usize][x.rem_euclid(4) as usize];
    threshold * 100 < share * 16
}

/// Sideways ripple at some rows of water from the river's edge.
fn ripple(rows: i32) -> i32 {
    if rows < REFLECTION_SOLID_ROWS {
        return 0;
    }
    RIPPLE[rows.rem_euclid(RIPPLE.len() as i32) as usize]
}

/// Where in the river a point of the church lands: every (x, y) of water that
/// shows it, and whether dithering lets that pixel through.
fn reflection_of(mut visit: impl FnMut(i32, i32, i32, i32, bool)) {
    const MARGIN: i32 = 6;
    let right = CHURCH_X + TOWER_W + LINK_W + NAVE_W;
    for rows in 0..REFLECTION_ROWS {
        let y = RIVER_Y + rows;
        let source_y = CHURCH_BASE - rows * 4 / REFLECTION_STRETCH;
        let shift = ripple(rows);
        let share = reflection_share(rows);
        for x in CHURCH_X - MARGIN..right + MARGIN {
            visit(x, y, x - shift, source_y, dither_on(x, y, share));
        }
    }
}

/// The church on the water. Inside its outline the city's scattered lights give
/// way to the reflection, dithered thinner toward the viewer.
fn draw_reflection(canvas: &mut Canvas) {
    // Only the church on a bare canvas, so its outline can be read back. The
    // church itself never uses the canvas's starting color.
    let mut church = Canvas::new();
    draw_church(&mut church);
    reflection_of(|x, y, source_x, source_y, on| {
        let color = church.get(source_x, source_y);
        if color == Pal::Night {
            return;
        }
        canvas.set(x, y, if on { reflected(color) } else { Pal::Deep });
    });
}

/// The color a part of the church takes on the water.
fn reflected(color: Pal) -> Pal {
    match color {
        Pal::Black => Pal::Night,
        Pal::Gray => Pal::Slate,
        other => other,
    }
}

fn scene_canvas() -> Canvas {
    let mut canvas = Canvas::new();
    draw_sky(&mut canvas);
    draw_skyline(&mut canvas);
    draw_shore(&mut canvas);
    draw_church(&mut canvas);
    draw_river(&mut canvas);
    draw_reflection(&mut canvas);
    canvas
}

fn scene_image() -> Image {
    let data = scene_canvas()
        .pixels
        .into_iter()
        .flat_map(|color| {
            let [r, g, b] = color.rgb();
            [r, g, b, 255]
        })
        .collect();
    Image::new(
        Extent3d {
            width: VIEW_W,
            height: VIEW_H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Resource, Clone)]
pub struct DetroitImage(pub Handle<Image>);

pub fn build_detroit_image(mut images: ResMut<Assets<Image>>, mut commands: Commands) {
    commands.insert_resource(DetroitImage(images.add(scene_image())));
}

/// The whole scene as a background sprite.
pub fn scene_sprite(image: &DetroitImage) -> impl Bundle {
    (
        Sprite::from_image(image.0.clone()),
        at(VIEW_W as f32 / 2.0, VIEW_H as f32 / 2.0, z::BACKGROUND),
    )
}

/// The pixels of the river that show the lit belfry slits.
fn belfry_reflection() -> Vec<(i32, i32)> {
    let slits = belfry_slits();
    let mut pixels = Vec::new();
    reflection_of(|x, y, source_x, source_y, on| {
        let in_slit = slits.iter().any(|(slit_x, slit_y, height)| {
            source_x == *slit_x && (*slit_y..slit_y + height).contains(&source_y)
        });
        if on && in_slit {
            pixels.push((x, y));
        }
    });
    pixels
}

/// Sprites over the belfry slits and their reflection. They show only while the
/// bell rings.
pub fn belfry_light() -> Vec<impl Bundle> {
    let slits = belfry_slits().into_iter();
    let water = belfry_reflection().into_iter().map(|(x, y)| (x, y, 1));
    slits
        .chain(water)
        .map(|(x, y, h)| rect(x as f32, y as f32, 1.0, h as f32, Pal::Yellow, z::FIELD))
        .collect()
}

const LETTER_H: usize = 5;
const LETTER_GAP: f32 = 2.0;
const LETTER_PIXEL: f32 = 2.0;

/// Chunky letters for the one word this scene needs, a little smaller than the
/// double-size font and still on whole pixels.
fn big_letter(letter: char) -> [&'static str; LETTER_H] {
    match letter {
        'D' => ["1110", "1001", "1001", "1001", "1110"],
        'E' => ["1111", "1000", "1110", "1000", "1111"],
        'T' => ["111", "010", "010", "010", "010"],
        'R' => ["1110", "1001", "1110", "1010", "1001"],
        'O' => ["0110", "1001", "1001", "1001", "0110"],
        _ => ["111", "010", "010", "010", "111"],
    }
}

/// The pixels of the caption as (x, y), centered across the view.
pub fn caption_pixels(word: &str, y: f32) -> Vec<(f32, f32)> {
    let letter_width = |letter: char| big_letter(letter)[0].len() as f32 * LETTER_PIXEL;
    let total: f32 = word.chars().map(letter_width).sum::<f32>()
        + LETTER_GAP * (word.chars().count() as f32 - 1.0);
    let mut x = ((VIEW_W as f32 - total) / 2.0).floor();
    let mut pixels = Vec::new();
    for letter in word.chars() {
        for (row, line) in big_letter(letter).iter().enumerate() {
            for (column, bit) in line.chars().enumerate() {
                if bit == '1' {
                    pixels.push((
                        x + column as f32 * LETTER_PIXEL,
                        y + row as f32 * LETTER_PIXEL,
                    ));
                }
            }
        }
        x += letter_width(letter) + LETTER_GAP;
    }
    pixels
}

/// One pixel of the caption.
pub fn caption_pixel(x: f32, y: f32) -> impl Bundle {
    rect(x, y, LETTER_PIXEL, LETTER_PIXEL, Pal::Foam, z::TEXT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skyline_fits_the_view() {
        for Block(left, width, height, ..) in &SKYLINE {
            assert!(*left >= 0 && left + width <= VIEW_W as i32);
            assert!(BASE - height > 0);
        }
    }

    #[test]
    fn the_tallest_building_is_the_one_with_the_red_light() {
        let top = |x: i32| {
            (BASE - 90..BASE).find(|y| {
                let canvas = scene_canvas();
                canvas.get(x, *y) != Pal::Night && canvas.get(x, *y) != Pal::Deep
            })
        };
        let tower = top(TOWER_LIGHT_X).unwrap();
        let canvas = scene_canvas();
        assert_eq!(canvas.get(TOWER_LIGHT_X, tower), Pal::Red);
        for Block(_, _, height, ..) in &SKYLINE {
            assert!(*height <= 82);
        }
    }

    #[test]
    fn the_church_stands_on_the_shore_right_of_center() {
        const { assert!(CHURCH_X > VIEW_W as i32 / 2) };
        const { assert!(CHURCH_BASE > BASE && CHURCH_BASE < RIVER_Y) };
    }

    #[test]
    fn the_church_is_much_smaller_than_the_tallest_tower() {
        const { assert!(TOWER_H * 3 < 82) };
    }

    #[test]
    fn the_scene_uses_only_palette_colors_by_construction() {
        let canvas = scene_canvas();
        assert_eq!(canvas.pixels.len(), (VIEW_W * VIEW_H) as usize);
        assert!(canvas.pixels.contains(&Pal::Red));
        assert!(canvas.pixels.contains(&Pal::Foam));
    }

    #[test]
    fn belfry_slits_are_on_the_tower_and_are_dark_until_lit() {
        let canvas = scene_canvas();
        for (x, y, height) in belfry_slits() {
            assert!((CHURCH_X..CHURCH_X + TOWER_W).contains(&x));
            for row in y..y + height {
                assert_eq!(canvas.get(x, row), Pal::Black);
            }
        }
    }

    #[test]
    fn the_reflection_stays_in_the_water_and_reaches_the_top_of_the_tower() {
        reflection_of(|_, y, _, _, _| assert!(y >= RIVER_Y && y < VIEW_H as i32));
        let tower_top = CHURCH_BASE - TOWER_H - 1;
        let farthest = CHURCH_BASE - (REFLECTION_ROWS - 1) * 4 / REFLECTION_STRETCH;
        assert!(farthest <= tower_top);
    }

    #[test]
    fn the_reflection_thins_with_distance_from_the_shore() {
        let lit = |rows: i32| {
            let share = reflection_share(rows);
            (0..16)
                .flat_map(|y| (0..16).map(move |x| (x, y)))
                .filter(|(x, y)| dither_on(*x, *y, share))
                .count()
        };
        assert_eq!(lit(0), 256);
        assert!(lit(REFLECTION_ROWS - 1) < lit(REFLECTION_ROWS / 2));
        assert!(lit(REFLECTION_ROWS / 2) < lit(REFLECTION_SOLID_ROWS + 1));
        assert!(lit(REFLECTION_ROWS - 1) > 0);
    }

    #[test]
    fn the_reflection_is_the_church_on_the_water() {
        let canvas = scene_canvas();
        let stone_in_water = (RIVER_Y..RIVER_Y + REFLECTION_ROWS)
            .flat_map(|y| (CHURCH_X - 6..CHURCH_X + 40).map(move |x| (x, y)))
            .filter(|(x, y)| canvas.get(*x, *y) == STONE)
            .count();
        assert!(stone_in_water > 150, "{stone_in_water}");
    }

    #[test]
    fn the_lit_belfry_shows_on_the_water() {
        let pixels = belfry_reflection();
        assert!(!pixels.is_empty());
        assert!(pixels.iter().all(|(_, y)| *y >= RIVER_Y));
    }

    #[test]
    fn the_caption_is_smaller_than_double_size_text_and_centered() {
        let pixels = caption_pixels("DETROIT", 4.0);
        let left = pixels.iter().map(|p| p.0).fold(f32::MAX, f32::min);
        let right = pixels.iter().map(|p| p.0).fold(f32::MIN, f32::max) + LETTER_PIXEL;
        assert!((left + right - VIEW_W as f32).abs() <= 2.0);
        // The double-size font stands 14 pixels tall.
        let height = LETTER_H as f32 * LETTER_PIXEL;
        assert!(height < 14.0);
        assert!(height >= 10.0);
    }
}
