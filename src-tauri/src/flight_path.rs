use gdal::Dataset;
use geo::Area;
use geo::{
    algorithm::MinimumRotatedRect, coordinate_position::CoordPos, Coord, CoordinatePosition,
    LineString, Polygon,
};
use kml::{Kml, KmlWriter};
use proj::Proj;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{collections::HashMap, fs::File};

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
    let vrt_path = String::from("../data/elevation.vrt");

    let coverage = get_ground_coverage(&drone);
    let angle = get_lawnmower_angle(&mbr_coords);
    let spacing = coverage * (100.0 - drone.overlap) / 100.0;

    let initial_waypoints = get_waypoints(&polygon, &mbr, &angle, &spacing);
    let waypoints = adjust_waypoints_for_slope(&initial_waypoints, &vrt_path, drone.altitude);

    let search_area = calculate_search_area(&polygon);
    let est_flight_time = calculate_flight_time(&waypoints, drone.speed);


    let _ = write_flightpath_kml(&waypoints, &drone);

    //FlightPlanResult {
    //    waypoints,
    //    search_area,
    //    est_flight_time,
    //}

    FlightPlanResult {
        waypoints,
        search_area,
        est_flight_time,
    }
}

/// Calculates the search area of the polygon in square kilometers
fn calculate_search_area(polygon: &Polygon) -> f64 {
    // Convert polygon coordinates to meters (NZTM projection)
    let coords_meters = get_coord_meters(&polygon.exterior().coords().collect::<Vec<_>>());
    let polygon_meters = Polygon::new(LineString::from(coords_meters), vec![]);

    // Calculate area using the geo crate's Area trait
    polygon_meters.unsigned_area() / 1_000_000.0
}

fn calculate_flight_time(waypoints: &[[f64; 2]], speed_ms: f64) -> f64 {
    if waypoints.len() < 2 {
        return 0.0;
    }

    let mut total_distance = 0.0;
    let to_nztm =
        Proj::new_known_crs("EPSG:4326", "EPSG:2193", None).expect("Failed to create projection");

    for i in 0..waypoints.len() - 1 {
        let current = waypoints[i];
        let next = waypoints[i + 1];

        // Convert both points to meters
        let (x1, y1) = to_nztm
            .convert((current[0], current[1]))
            .expect("Cannot convert current waypoint to NZTM");
        let (x2, y2) = to_nztm
            .convert((next[0], next[1]))
            .expect("Cannot convert next waypoint to NZTM");

        // Calculate distance between waypoints in meters
        let distance = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
        total_distance += distance;
    }

    // Convert time from seconds to minutes
    (total_distance / speed_ms) / 60.0
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


fn adjust_waypoints_for_slope(waypoints: &[[f64; 2]], vrt_path: &str, altitude: f64) -> Vec<[f64; 2]> {
    let dataset = match Dataset::open(vrt_path) {
        Ok(ds) => ds,
        Err(_) => return waypoints.to_vec(),
    };

    let rasterband = match dataset.rasterband(1) {
        Ok(band) => band,
        Err(_) => return waypoints.to_vec(),
    };

    let geotransform = match dataset.geo_transform() {
        Ok(gt) => gt,
        Err(_) => return waypoints.to_vec(),
    };

    let to_nztm = match Proj::new_known_crs("EPSG:4326", "EPSG:2193", None) {
        Ok(proj) => proj,
        Err(_) => return waypoints.to_vec(),
    };

    let to_wgs84 = match Proj::new_known_crs("EPSG:2193", "EPSG:4326", None) {
        Ok(proj) => proj,
        Err(_) => return waypoints.to_vec(),
    };

    let raster_size = dataset.raster_size();
    let mut adjusted_waypoints = Vec::new();

    for waypoint in waypoints {
        let (x, y) = match to_nztm.convert((waypoint[0], waypoint[1])) {
            Ok(coords) => coords,
            Err(_) => {
                adjusted_waypoints.push(*waypoint);
                continue;
            }
        };

        // Calculate slope using finite differences
        let pixel_size = geotransform[1].abs(); // assuming square pixels
        let sample_distance = pixel_size * 2.0; // sample 2 pixels away

        // Get elevations in 4 directions
        let elevations = [
            get_elevation_at_point(&rasterband, &geotransform, raster_size, x + sample_distance, y),
            get_elevation_at_point(&rasterband, &geotransform, raster_size, x - sample_distance, y),
            get_elevation_at_point(&rasterband, &geotransform, raster_size, x, y + sample_distance),
            get_elevation_at_point(&rasterband, &geotransform, raster_size, x, y - sample_distance),
        ];

        // Calculate gradients
        if let [Some(e_east), Some(e_west), Some(e_north), Some(e_south)] = elevations {
            let dx = (e_east - e_west) / (2.0 * sample_distance);
            let dy = (e_north - e_south) / (2.0 * sample_distance);
            
            // Calculate slope direction (downhill)
            let slope_angle = dy.atan2(dx);
            
            // Perpendicular direction to slope (90 degrees from downhill)
            let perp_angle = slope_angle + std::f64::consts::PI / 2.0;
            
            // Shift waypoint by altitude distance perpendicular to slope
            let shift_x = altitude * perp_angle.cos();
            let shift_y = altitude * perp_angle.sin();
            
            let new_x = x + shift_x;
            let new_y = y + shift_y;
            
            // Convert back to lat/lon
            if let Ok((lon, lat)) = to_wgs84.convert((new_x, new_y)) {
                adjusted_waypoints.push([lon, lat]);
                println!("Waypoint adjusted: slope_angle={:.2}°, shift=({:.1}m, {:.1}m)", 
                         slope_angle.to_degrees(), shift_x, shift_y);
            } else {
                adjusted_waypoints.push(*waypoint);
            }
        } else {
            adjusted_waypoints.push(*waypoint);
        }
    }

    adjusted_waypoints
}

fn get_elevation_at_point(
    rasterband: &gdal::raster::RasterBand,
    geotransform: &[f64; 6],
    raster_size: (usize, usize),
    x: f64,
    y: f64,
) -> Option<f64> {
    let pixel_x = ((x - geotransform[0]) / geotransform[1]).floor() as isize;
    let pixel_y = ((y - geotransform[3]) / geotransform[5]).floor() as isize;

    if pixel_x < 0 || pixel_y < 0 || pixel_x >= raster_size.0 as isize || pixel_y >= raster_size.1 as isize {
        return None;
    }

    let mut buffer = [0.0f32; 1];
    match rasterband.read_into_slice::<f32>((pixel_x, pixel_y), (1, 1), (1, 1), &mut buffer, None) {
        Ok(_) => {
            let elevation = buffer[0] as f64;
            if (elevation - (-32767.0)).abs() < 0.1 {
                None
            } else {
                Some(elevation)
            }
        }
        Err(_) => None,
    }
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

    let filename = format!("../output/flightpath_{}.kml", drone.model);
    let mut file = File::create(&filename)?;

    file.write_all(&buf)?;

    Ok(())
}
