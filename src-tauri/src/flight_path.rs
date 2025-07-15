use geo::{
    algorithm::{self, area::Area, MinimumRotatedRect}, coordinate_position::CoordPos, Coord, CoordinatePosition, Intersects, LineString, Polygon
};
use kml::{types::Point, Kml, KmlWriter};
use proj::Proj;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File};
use std::{f32::INFINITY, io::Write};

#[derive(Serialize, Deserialize)]
pub struct Drone {
    pub model: String,
    pub fov: f64,
    pub altitude: f64,
    pub overlap: f64,
    pub speed: f64,
}

#[derive(Serialize, Deserialize)]
pub struct FlightPlanResult {
    pub waypoints: Vec<[f64; 2]>,
    pub search_area: f64,
    pub est_flight_time: f64,
}

#[tauri::command]
pub async fn generate_flightpath(coords: Vec<[f64; 2]>, drone: Drone) -> FlightPlanResult {
    let points: Vec<Coord> = coords.iter().map(|c| Coord::from((c[0], c[1]))).collect();
    let polygon = Polygon::new(LineString::from(points.clone()), vec![]);
    let mbr = MinimumRotatedRect::minimum_rotated_rect(&polygon).unwrap();
    let mbr_coords = mbr.exterior().coords().collect::<Vec<_>>();

    let coverage = get_ground_coverage(&drone);
    let angle = get_lawnmower_angle(&mbr_coords);
    let spacing = coverage * (100.0 - drone.overlap) / 100.0;

    let waypoints = get_waypoints(&polygon, &mbr, &angle, &spacing);

    //FlightPlanResult {
    //    waypoints,
    //    search_area,
    //    est_flight_time,
    //}

    FlightPlanResult {
        waypoints,
        search_area: 0.0,
        est_flight_time: 0.0,
    }
}

/// Returns a grid of waypoints that cover the entire search area using a lawnmower pattern
fn get_waypoints(polygon: &Polygon, mbr: &Polygon, angle: &f64, spacing: &f64) -> Vec<[f64; 2]> {
    let mut waypoints = Vec::new();
    let mbr_coords = mbr.exterior().coords().collect::<Vec<_>>();
    let mbr_coords_meters = get_coord_meters(&mbr_coords);

    // Convert the search area polygon to meters
    let search_coords_meters = get_coord_meters(&polygon.exterior().coords().collect::<Vec<_>>());
    let search_polygon_meters = Polygon::new(LineString::from(search_coords_meters), vec![]);

    // Find the bounds of the MBR
    let min_x = mbr_coords_meters
        .iter()
        .map(|c| c.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = mbr_coords_meters
        .iter()
        .map(|c| c.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = mbr_coords_meters
        .iter()
        .map(|c| c.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = mbr_coords_meters
        .iter()
        .map(|c| c.y)
        .fold(f64::NEG_INFINITY, f64::max);

    // Calculate perpendicular direction for line spacing
    let perp_angle = angle + std::f64::consts::PI / 2.0;
    let line_dx = perp_angle.cos();
    let line_dy = perp_angle.sin();

    // Calculate flight line direction
    let flight_dx = angle.cos();
    let flight_dy = angle.sin();

    // Calculate the number of parallel lines needed
    let width = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
    let num_lines = (width / spacing).ceil() as i32;

    // Generate waypoints for each flight line
    let mut line_index = 0;
    for i in -(num_lines / 2)..=(num_lines / 2) {
        let offset_dist = i as f64 * spacing;

        // Calculate the center point of the MBR
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Calculate the starting point of this flight line
        let line_start_x = center_x + offset_dist * line_dx;
        let line_start_y = center_y + offset_dist * line_dy;

        // Generate points along this flight line
        let mut line_waypoints = Vec::new();
        let line_length = width * 2.0; // Make sure we cover the entire area
        let num_points = (line_length / (spacing / 4.0)) as i32; // Higher resolution along the line

        for j in -(num_points / 2)..=(num_points / 2) {
            let point_dist = j as f64 * (spacing / 4.0);
            let point_x = line_start_x + point_dist * flight_dx;
            let point_y = line_start_y + point_dist * flight_dy;

            let point = Coord {
                x: point_x,
                y: point_y,
            };

            // Check if this point is within the search area
            if search_polygon_meters.coordinate_position(&point) == CoordPos::Inside
                || search_polygon_meters.coordinate_position(&point) == CoordPos::OnBoundary
            {
                line_waypoints.push(point);
            }
        }

        // Add waypoints from this line (alternate direction for lawnmower pattern)
        if !line_waypoints.is_empty() {
            if line_index % 2 == 0 {
                waypoints.extend(line_waypoints);
            } else {
                waypoints.extend(line_waypoints.into_iter().rev());
            }
            line_index += 1;
        }
    }

    // Convert waypoints back to lat/lon
    let mut waypoints_latlon = Vec::new();
    let to_wgs84 =
        Proj::new_known_crs("EPSG:2193", "EPSG:4326", None).expect("Failed to create projection");

    for coord in waypoints {
        let (x, y) = to_wgs84
            .convert((coord.x, coord.y))
            .expect("Cannot convert coords to wsg84");
        waypoints_latlon.push([x, y]);
    }

    waypoints_latlon
}

/// Returns the ground coverage in meters of a photo taken from the drone
fn get_ground_coverage(drone: &Drone) -> f64 {
    let fov_rad = drone.fov.to_radians();
    2.0 * drone.altitude * (fov_rad / 2.0).tan()
}

/// Convert Vec of coords in lat, lon to meters
fn get_coord_meters(coords: &[&Coord]) -> Vec<Coord> {
    let mut converted = Vec::new();
    let to_nztm =
        Proj::new_known_crs("EPSG:4326", "EPSG:2193", None).expect("Failed to create projection");
    for coord in coords {
        let (x, y) = to_nztm
            .convert((coord.x, coord.y))
            .expect("Cannot convert coords to nztm");

        converted.push(Coord { x, y });
    }
    converted
}

/// Returns the optimal angle of the lawnmover pattern based on the minimum rotated
/// rectangle of the search area.
fn get_lawnmower_angle(mbr_coords: &[&Coord]) -> f64 {
    let to_nztm =
        Proj::new_known_crs("EPSG:4326", "EPSG:2193", None).expect("Failed to create projection");

    let mut max_dist = 0.0;
    let mut longest_len_dx = 0.0;
    let mut longest_len_dy = 0.0;

    for i in 0..mbr_coords.len() - 1 {
        let (x1, y1) = to_nztm
            .convert((mbr_coords[i].x, mbr_coords[i].y))
            .expect("Cannot convert coords to nztm");
        let (x2, y2) = to_nztm
            .convert((mbr_coords[i + 1].x, mbr_coords[i + 1].y))
            .expect("Cannot convert coords to nztm");

        let dx = x2 - x1;
        let dy = y2 - y1;

        let dist = (dx * dx + dy * dy).sqrt();

        if dist > max_dist {
            max_dist = dist;
            longest_len_dx = dx;
            longest_len_dy = dy;
        }
    }

    longest_len_dy.atan2(longest_len_dx)
}

/// Creates a KML file containing the flight information for the drone
fn write_flightpath_kml(waypoints: &[[f64; 2]], drone: &Drone) -> std::io::Result<()> {
    let mut elements = Vec::new();

    for (i, waypoint) in waypoints.iter().enumerate() {
        let point = kml::types::Point {
            coord: kml::types::Coord {
                x: waypoint[0], // longitude
                y: waypoint[1], // latitude
                z: None,
            },
            extrude: false,
            altitude_mode: kml::types::AltitudeMode::RelativeToGround,
            attrs: HashMap::new(),
        };

        let placemark = kml::types::Placemark {
            name: Some(format!("Waypoint {}", i + 1)),
            description: Some(format!("Lat: {}, Lon: {}", waypoint[1], waypoint[0])),
            geometry: Some(kml::types::Geometry::Point(point)),
            ..Default::default()
        };

        elements.push(Kml::Placemark(placemark));
    }

    let document = Kml::KmlDocument(kml::types::KmlDocument {
        version: kml::types::KmlVersion::Unknown,
        attrs: HashMap::new(),
        elements,
    });

    let mut buf = Vec::new();
    let mut writer = KmlWriter::from_writer(&mut buf);
    writer.write(&document).unwrap();

    let filename = format!("flightpath_{}.kml", drone.model);
    let mut file = File::create(&filename)?;

    file.write_all(&buf)?;

    Ok(())
}
