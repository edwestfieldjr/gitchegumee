//! Great-circle-free chart arithmetic: positions and distances along a route.

use crate::content::{RouteDef, Waypoint};

const NM_PER_DEGREE: f32 = 60.0;
/// Longitude scale at the latitude of Lake Superior.
const LON_SCALE: f32 = 0.68;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32,
}

impl From<&Waypoint> for LatLon {
    fn from(waypoint: &Waypoint) -> Self {
        Self {
            lat: waypoint.lat,
            lon: waypoint.lon,
        }
    }
}

impl LatLon {
    pub fn distance_nm(self, other: LatLon) -> f32 {
        let north = (other.lat - self.lat) * NM_PER_DEGREE;
        let east = (other.lon - self.lon) * NM_PER_DEGREE * LON_SCALE;
        (north * north + east * east).sqrt()
    }

    pub fn bearing_to(self, other: LatLon) -> f32 {
        let north = other.lat - self.lat;
        let east = (other.lon - self.lon) * LON_SCALE;
        east.atan2(north).to_degrees().rem_euclid(360.0)
    }

    fn lerp(self, other: LatLon, t: f32) -> LatLon {
        LatLon {
            lat: self.lat + (other.lat - self.lat) * t,
            lon: self.lon + (other.lon - self.lon) * t,
        }
    }
}

pub fn path_length(path: &[LatLon]) -> f32 {
    path.windows(2).map(|leg| leg[0].distance_nm(leg[1])).sum()
}

/// The leg holding `nm`, and the fraction of the way along it.
fn locate(path: &[LatLon], nm: f32) -> Option<(usize, f32)> {
    let mut remaining = nm.max(0.0);
    let last = path.len().checked_sub(2)?;
    for (index, leg) in path.windows(2).enumerate() {
        let length = leg[0].distance_nm(leg[1]);
        if remaining <= length || index == last {
            let t = if length > 0.0 {
                (remaining / length).min(1.0)
            } else {
                0.0
            };
            return Some((index, t));
        }
        remaining -= length;
    }
    None
}

pub fn path_point(path: &[LatLon], nm: f32) -> LatLon {
    match locate(path, nm) {
        Some((index, t)) => path[index].lerp(path[index + 1], t),
        None => path
            .first()
            .copied()
            .unwrap_or(LatLon { lat: 0.0, lon: 0.0 }),
    }
}

pub fn path_heading(path: &[LatLon], nm: f32) -> f32 {
    locate(path, nm).map_or(0.0, |(index, _)| path[index].bearing_to(path[index + 1]))
}

/// A path from where the ship is now, joining `route` at its next point east.
pub fn reroute(from: LatLon, route: &RouteDef) -> Vec<LatLon> {
    let mut path = vec![from];
    path.extend(
        route
            .points
            .iter()
            .map(LatLon::from)
            .filter(|point| point.lon > from.lon),
    );
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::RouteId;
    use crate::content::tests::real_content;

    fn point(lat: f32, lon: f32) -> LatLon {
        LatLon { lat, lon }
    }

    #[test]
    fn one_degree_of_latitude_is_sixty_miles() {
        assert_eq!(point(47.0, -90.0).distance_nm(point(48.0, -90.0)), 60.0);
    }

    #[test]
    fn walks_along_the_path() {
        let path = [point(47.0, -90.0), point(48.0, -90.0), point(49.0, -90.0)];
        assert_eq!(path_length(&path), 120.0);
        assert_eq!(path_point(&path, 30.0), point(47.5, -90.0));
        assert_eq!(path_point(&path, 90.0), point(48.5, -90.0));
        assert_eq!(path_point(&path, 500.0), point(49.0, -90.0));
    }

    #[test]
    fn heading_follows_the_leg() {
        let path = [point(47.0, -90.0), point(48.0, -90.0), point(48.0, -89.0)];
        assert_eq!(path_heading(&path, 10.0), 0.0);
        assert_eq!(path_heading(&path, 70.0), 90.0);
    }

    #[test]
    fn routes_have_plausible_lengths() {
        let content = real_content();
        let length = |id| {
            let route = content.chart.route(id).unwrap();
            let path: Vec<LatLon> = route.points.iter().map(LatLon::from).collect();
            path_length(&path)
        };
        let (standard, north) = (length(RouteId::Standard), length(RouteId::North));
        assert!((280.0..360.0).contains(&standard), "{standard}");
        assert!(north > standard, "{north} vs {standard}");
    }

    #[test]
    fn reroute_only_goes_forward() {
        let content = real_content();
        let route = content.chart.route(RouteId::North).unwrap();
        let path = reroute(point(47.5, -88.5), route);
        assert_eq!(path[0], point(47.5, -88.5));
        assert!(path[1..].iter().all(|p| p.lon > -88.5));
    }
}
