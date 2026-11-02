//! The chart: an original pixel map rasterized from the outline in `chart.ron`.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::route::LatLon;
use crate::content::{Chart, Content};
use crate::states::GameState;
use crate::ui::palette::Pal;

pub const MAP_W: u32 = 224;
pub const MAP_H: u32 = 118;
const WEST: f32 = -92.45;
const EAST: f32 = -84.25;
const NORTH: f32 = 49.10;
const SOUTH: f32 = 46.25;

/// Pixel position on the map of a lat/lon.
pub fn project(point: LatLon) -> Vec2 {
    Vec2::new(
        (point.lon - WEST) / (EAST - WEST) * MAP_W as f32,
        (NORTH - point.lat) / (NORTH - SOUTH) * MAP_H as f32,
    )
}

fn unproject(x: f32, y: f32) -> (f32, f32) {
    (
        NORTH - y / MAP_H as f32 * (NORTH - SOUTH),
        WEST + x / MAP_W as f32 * (EAST - WEST),
    )
}

/// Even-odd test for whether a (lat, lon) lies inside a polygon.
pub fn in_polygon(polygon: &[(f32, f32)], point: (f32, f32)) -> bool {
    let (lat, lon) = point;
    let mut inside = false;
    let mut previous = polygon.last().copied().unwrap_or((0.0, 0.0));
    for &current in polygon {
        let crosses = (current.0 > lat) != (previous.0 > lat);
        if crosses {
            let t = (lat - current.0) / (previous.0 - current.0);
            if lon < current.1 + t * (previous.1 - current.1) {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

/// True where the map shows open water.
pub fn is_water(chart: &Chart, point: (f32, f32)) -> bool {
    in_polygon(&chart.lake, point) && !chart.islands.iter().any(|island| in_polygon(island, point))
}

/// RGBA8 pixels of the map: land, water, and a sand line along the coast.
fn map_pixels(chart: &Chart) -> Vec<u8> {
    let (width, height) = (MAP_W as usize, MAP_H as usize);
    let water: Vec<bool> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| is_water(chart, unproject(x as f32 + 0.5, y as f32 + 0.5)))
        .collect();
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let here = water[y * width + x];
            let neighbor_water = |dx: isize, dy: isize| {
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                nx >= 0
                    && ny >= 0
                    && (nx as usize) < width
                    && (ny as usize) < height
                    && water[ny as usize * width + nx as usize]
            };
            let color = if here {
                Pal::Lake
            } else if neighbor_water(1, 0)
                || neighbor_water(-1, 0)
                || neighbor_water(0, 1)
                || neighbor_water(0, -1)
            {
                Pal::Sand
            } else {
                Pal::Shore
            };
            let [r, g, b] = color.rgb();
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    pixels
}

#[derive(Resource, Clone)]
pub struct MapImage(pub Handle<Image>);

fn build_map(mut commands: Commands, content: Res<Content>, mut images: ResMut<Assets<Image>>) {
    let image = Image::new(
        Extent3d {
            width: MAP_W,
            height: MAP_H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        map_pixels(&content.chart),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.insert_resource(MapImage(images.add(image)));
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(GameState::Boot), build_map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::real_content;

    #[test]
    fn polygon_test_handles_a_square() {
        let square = [(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0)];
        assert!(in_polygon(&square, (1.0, 1.0)));
        assert!(!in_polygon(&square, (3.0, 1.0)));
    }

    #[test]
    fn known_places_are_on_the_right_side_of_the_coast() {
        let chart = real_content().chart;
        let open_lake = (47.6, -87.0);
        let wreck = (46.998, -85.11);
        let isle_royale = (48.0, -88.8);
        let wisconsin = (46.0, -90.0);
        assert!(is_water(&chart, open_lake));
        assert!(is_water(&chart, wreck));
        assert!(!is_water(&chart, isle_royale));
        assert!(!is_water(&chart, wisconsin));
    }

    #[test]
    fn every_route_leg_stays_on_the_map_and_mostly_on_water() {
        let chart = real_content().chart;
        for route in &chart.routes {
            let inner = &route.points[1..route.points.len() - 1];
            for point in inner {
                assert!(
                    is_water(&chart, (point.lat, point.lon)),
                    "{} is on land",
                    point.name
                );
            }
            for point in &route.points {
                let at = project(LatLon::from(point));
                assert!(at.x >= 0.0 && at.x < MAP_W as f32, "{}", point.name);
                assert!(at.y >= 0.0 && at.y < MAP_H as f32, "{}", point.name);
            }
        }
    }

    #[test]
    fn the_map_only_uses_palette_colors() {
        let chart = real_content().chart;
        let pixels = map_pixels(&chart);
        let (chunks, _) = pixels.as_chunks::<4>();
        for pixel in chunks {
            let rgb = [pixel[0], pixel[1], pixel[2]];
            assert_eq!(Pal::nearest(rgb).rgb(), rgb);
        }
    }
}
