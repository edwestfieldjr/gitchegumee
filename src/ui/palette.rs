//! The game's 16-color palette. Every color on screen comes from here.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pal {
    Black,
    Night,
    Deep,
    Lake,
    Sky,
    Foam,
    White,
    Gray,
    Slate,
    Rust,
    Red,
    Amber,
    Yellow,
    Sand,
    Pine,
    Shore,
}

impl Pal {
    pub const ALL: [Pal; 16] = [
        Pal::Black,
        Pal::Night,
        Pal::Deep,
        Pal::Lake,
        Pal::Sky,
        Pal::Foam,
        Pal::White,
        Pal::Gray,
        Pal::Slate,
        Pal::Rust,
        Pal::Red,
        Pal::Amber,
        Pal::Yellow,
        Pal::Sand,
        Pal::Pine,
        Pal::Shore,
    ];

    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Pal::Black => [0x00, 0x00, 0x00],
            Pal::Night => [0x0a, 0x16, 0x38],
            Pal::Deep => [0x10, 0x2c, 0x70],
            Pal::Lake => [0x00, 0x40, 0xa0],
            Pal::Sky => [0x50, 0x98, 0xdc],
            Pal::Foam => [0xc0, 0xe0, 0xf0],
            Pal::White => [0xf0, 0xf0, 0xe8],
            Pal::Gray => [0xa0, 0xa0, 0xa0],
            Pal::Slate => [0x50, 0x50, 0x50],
            Pal::Rust => [0x80, 0x18, 0x10],
            Pal::Red => [0xc8, 0x30, 0x20],
            Pal::Amber => [0xe0, 0x80, 0x00],
            Pal::Yellow => [0xf0, 0xd0, 0x40],
            Pal::Sand => [0xc0, 0xa0, 0x60],
            Pal::Pine => [0x30, 0x70, 0x40],
            Pal::Shore => [0x18, 0x38, 0x28],
        }
    }

    pub fn color(self) -> Color {
        let [r, g, b] = self.rgb();
        Color::srgb_u8(r, g, b)
    }

    /// The palette entry closest to an arbitrary color.
    pub fn nearest(rgb: [u8; 3]) -> Pal {
        Pal::ALL
            .into_iter()
            .min_by_key(|entry| {
                entry
                    .rgb()
                    .iter()
                    .zip(rgb)
                    .map(|(a, b)| (i32::from(*a) - i32::from(b)).pow(2))
                    .sum::<i32>()
            })
            .unwrap_or(Pal::Black)
    }
}

/// Snap RGBA8 pixels to the palette. Pixels are either opaque or fully clear,
/// so sprite art cannot introduce colors the palette does not have.
pub fn remap_rgba(pixels: &mut [u8]) {
    let (chunks, _) = pixels.as_chunks_mut::<4>();
    for pixel in chunks {
        *pixel = if pixel[3] < 128 {
            [0, 0, 0, 0]
        } else {
            let [r, g, b] = Pal::nearest([pixel[0], pixel[1], pixel[2]]).rgb();
            [r, g, b, 255]
        };
    }
}

/// Turn RGBA8 art into a white shape, to be tinted with one palette color.
pub fn silhouette_rgba(pixels: &mut [u8]) {
    let (chunks, _) = pixels.as_chunks_mut::<4>();
    for pixel in chunks {
        *pixel = if pixel[3] < 128 {
            [0, 0, 0, 0]
        } else {
            [255, 255, 255, 255]
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_entries_are_distinct() {
        for (i, a) in Pal::ALL.iter().enumerate() {
            for b in &Pal::ALL[i + 1..] {
                assert_ne!(a.rgb(), b.rgb());
            }
        }
    }

    #[test]
    fn silhouette_keeps_only_the_shape() {
        let mut pixels = [9, 9, 9, 0, 128, 24, 16, 255];
        silhouette_rgba(&mut pixels);
        assert_eq!(pixels, [0, 0, 0, 0, 255, 255, 255, 255]);
    }

    #[test]
    fn exact_colors_map_to_themselves() {
        for entry in Pal::ALL {
            assert_eq!(Pal::nearest(entry.rgb()), entry);
        }
    }

    #[test]
    fn remap_clears_transparency_and_snaps_color() {
        let mut pixels = [1, 2, 3, 40, 250, 250, 245, 255];
        remap_rgba(&mut pixels);
        assert_eq!(&pixels[..4], &[0, 0, 0, 0]);
        let [r, g, b] = Pal::White.rgb();
        assert_eq!(&pixels[4..], &[r, g, b, 255]);
    }
}
