use crate::writer::write_wqml;
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

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct CoverageRect {
    pub coords: [[f64; 2]; 5],
    pub center: [f64; 2],
}

#[derive(Serialize, Deserialize)]
pub struct FlightPlanResult {
    pub waypoints: Vec<Waypoint>,
    pub search_area: f64,
    pub est_flight_time: f64,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Waypoint {
    pub coverage_rect: CoverageRect,
    pub position: [f64; 2],
    pub bearing: f64,
    pub altitude: f64,
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

    let waypoints =
        get_waypoints_with_slope_adjustment(&polygon, &mbr, &angle, &spacing, &vrt_path, &drone);
    //let _ = write_flightpath_kml(&waypoints, &drone);
    write_wqml(&waypoints, &drone).await;
    let search_area = calculate_search_area(&polygon);
    let est_flight_time = calculate_flight_time(&waypoints, drone.speed);

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

fn calculate_flight_time(waypoints: &[Waypoint], speed_ms: f64) -> f64 {
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
            .convert((current.position[0], current.position[1]))
            .expect("Cannot convert current waypoint to NZTM");
        let (x2, y2) = to_nztm
            .convert((next.position[0], next.position[1]))
            .expect("Cannot convert next waypoint to NZTM");

        // Calculate distance between waypoints in meters
        let distance = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
        total_distance += distance;
    }

    // Convert time from seconds to minutes
    (total_distance / speed_ms) / 60.0
}

/// Calculate the slope magnitude at a given point
fn calculate_slope_at_point(
    point: Coord,
    rasterband: &gdal::raster::RasterBand,
    geotransform: &[f64; 6],
    raster_size: (usize, usize),
) -> f64 {
    let pixel_size = geotransform[1].abs(); // assuming square pixels
    let sample_distance = pixel_size * 2.0; // sample 2 pixels away

    // Get elevations in 4 directions
    let elevations = [
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            point.x + sample_distance,
            point.y,
        ),
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            point.x - sample_distance,
            point.y,
        ),
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            point.x,
            point.y + sample_distance,
        ),
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            point.x,
            point.y - sample_distance,
        ),
    ];

    // Calculate gradients
    if let [Some(e_east), Some(e_west), Some(e_north), Some(e_south)] = elevations {
        let dx = (e_east - e_west) / (2.0 * sample_distance);
        let dy = (e_north - e_south) / (2.0 * sample_distance);

        // Calculate slope magnitude (in radians)
        (dx.powi(2) + dy.powi(2)).sqrt().atan()
    } else {
        0.0 // Return 0 slope if elevation data is unavailable
    }
}

/// Returns the coverage rectangle representing the area that the photo
/// from that waypoint creates. Used for rendering the coverage rectangles on the frontend
fn generate_coverage_rect(
    waypoint: &Coord,
    slope_magnitude: &f64,
    angle: &f64,
    drone: &Drone,
) -> CoverageRect {
    // TODO adjust photo height based on slope angle
    let to_wgs84 =
        Proj::new_known_crs("EPSG:2193", "EPSG:4326", None).expect("Failed to create projection");

    let base_coverage = get_ground_coverage(drone);
    let slope_adjusted_coverage = base_coverage / slope_magnitude.cos().max(0.1);
    let hw = slope_adjusted_coverage / 2.0;

    let local_corners = [
        [-hw, hw],  // top-left
        [-hw, -hw], // bottom-left
        [hw, -hw],  // bottom-right
        [hw, hw],   // top-right
    ];

    // rotate and translate
    let rotated_corners: Vec<[f64; 2]> = local_corners
        .iter()
        .map(|[x, y]| {
            let xr = x * angle.cos() - y * angle.sin();
            let yr = x * angle.sin() + y * angle.cos();
            [waypoint.x + xr, waypoint.y + yr]
        })
        .collect();

    // project to WGS84
    let wgs84_coords: Vec<[f64; 2]> = rotated_corners
        .iter()
        .map(|[x, y]| {
            let (lon, lat) = to_wgs84.convert((*x, *y)).expect("Projection failed");
            [lon, lat]
        })
        .collect();


    CoverageRect {
        coords: [
            wgs84_coords[0],
            wgs84_coords[1],
            wgs84_coords[2],
            wgs84_coords[3],
            wgs84_coords[0],
        ],
        center: {
            let (lon, lat) = to_wgs84
                .convert((waypoint.x, waypoint.y))
                .expect("Projection failed");
            [lon, lat]
        },
    }
}

/// Returns a grid of waypoints that cover the entire search area using a lawnmower pattern
/// with slope adjustment applied to each waypoint as it's created
fn get_waypoints_with_slope_adjustment(
    polygon: &Polygon,
    mbr: &Polygon,
    angle: &f64,
    base_spacing: &f64,
    vrt_path: &str,
    drone: &Drone,
) -> Vec<Waypoint> {
    let mut waypoints = Vec::new();
    let mbr_coords = mbr.exterior().coords().collect::<Vec<_>>();
    let mbr_coords_meters = get_coord_meters(&mbr_coords);

    // Convert the search area polygon to meters
    let search_coords_meters = get_coord_meters(&polygon.exterior().coords().collect::<Vec<_>>());
    let search_polygon_meters = Polygon::new(LineString::from(search_coords_meters), vec![]);

    // Setup elevation data access
    let dataset = match Dataset::open(vrt_path) {
        Ok(ds) => ds,
        Err(_) => {
            // Fallback to original method without slope adjustment
            return get_waypoints_fallback(polygon, mbr, angle, base_spacing, drone);
        }
    };

    let rasterband = match dataset.rasterband(1) {
        Ok(band) => band,
        Err(_) => {
            return get_waypoints_fallback(polygon, mbr, angle, base_spacing, drone);
        }
    };

    let geotransform = match dataset.geo_transform() {
        Ok(gt) => gt,
        Err(_) => {
            return get_waypoints_fallback(polygon, mbr, angle, base_spacing, drone);
        }
    };

    let raster_size = dataset.raster_size();

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

    // Calculate the number of parallel lines needed (using base spacing)
    let width = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
    let num_lines = (width / base_spacing).ceil() as i32;

    let to_wgs84 =
        Proj::new_known_crs("EPSG:2193", "EPSG:4326", None).expect("Failed to create projection");

    // Generate waypoints for each flight line
    let mut line_index = 0;
    for i in -(num_lines / 2)..=(num_lines / 2) {
        let offset_dist = i as f64 * base_spacing;

        // Calculate the center point of the MBR
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Calculate the starting point of this flight line
        let line_start_x = center_x + offset_dist * line_dx;
        let line_start_y = center_y + offset_dist * line_dy;

        // Generate points along this flight line with adaptive spacing
        let mut line_waypoints = Vec::new();
        let line_length = width * 2.0; // Make sure we cover the entire area

        // Start from one end of the line
        let start_point_x = line_start_x - (line_length / 2.0) * flight_dx;
        let start_point_y = line_start_y - (line_length / 2.0) * flight_dy;

        let mut current_distance = 0.0;
        let mut waypoint_count = 0;

        while current_distance < line_length {
            let point_x = start_point_x + current_distance * flight_dx;
            let point_y = start_point_y + current_distance * flight_dy;

            let point = Coord {
                x: point_x,
                y: point_y,
            };

            // Check if this point is within the search area
            if search_polygon_meters.coordinate_position(&point) == CoordPos::Inside
                || search_polygon_meters.coordinate_position(&point) == CoordPos::OnBoundary
            {
                // Calculate slope at this point
                let slope_angle =
                    calculate_slope_at_point(point, &rasterband, &geotransform, raster_size);

                let coverage_rect =
                    generate_coverage_rect(&point, &slope_angle, &perp_angle, drone);

                // Apply slope adjustment to this waypoint position
                let adjusted_point = adjust_waypoint_for_slope(
                    point,
                    &rasterband,
                    &geotransform,
                    raster_size,
                    drone.altitude,
                );

                // Convert adjusted waypoint back to lat/lon
                if let Ok((lon, lat)) = to_wgs84.convert((adjusted_point.x, adjusted_point.y)) {
                    line_waypoints.push(Waypoint {
                        coverage_rect,
                        position: [lon, lat],
                        bearing: 0.0,
                        altitude: 100.0,
                    });
                }

                // Calculate next waypoint distance based on slope
                // When slope increases, effective coverage width decreases by cos(slope)
                // So we need to reduce spacing to maintain overlap
                let slope_factor = slope_angle.cos().max(0.1); // Prevent division by very small numbers
                let adjusted_spacing = base_spacing * slope_factor;

                current_distance += adjusted_spacing;
            } else {
                // Move forward by a small increment if outside search area
                current_distance += base_spacing / 4.0;
            }

            waypoint_count += 1;
            // Safety check to prevent infinite loops
            if waypoint_count > 10000 {
                break;
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

    waypoints
}

/// Fallback waypoint generation without slope adjustment
fn get_waypoints_fallback(
    polygon: &Polygon,
    mbr: &Polygon,
    angle: &f64,
    spacing: &f64,
    drone: &Drone,
) -> Vec<Waypoint> {
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
        let coverage_rect = generate_coverage_rect(&coord, &0.0, &perp_angle, drone);
        let (x, y) = to_wgs84
            .convert((coord.x, coord.y))
            .expect("Cannot convert coords to wgs84");
        waypoints_latlon.push(Waypoint {
            coverage_rect,
            position: [x, y],
            bearing: 0.0,
            altitude: 100.0,
        });
    }

    waypoints_latlon
}

fn adjust_waypoint_for_slope(
    waypoint: Coord,
    rasterband: &gdal::raster::RasterBand,
    geotransform: &[f64; 6],
    raster_size: (usize, usize),
    altitude: f64,
) -> Coord {
    let x = waypoint.x;
    let y = waypoint.y;

    // Calculate slope using finite differences
    let pixel_size = geotransform[1].abs(); // assuming square pixels
    let sample_distance = pixel_size * 2.0; // sample 2 pixels away

    // Get elevations in 4 directions
    let elevations = [
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            x + sample_distance,
            y,
        ),
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            x - sample_distance,
            y,
        ),
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            x,
            y + sample_distance,
        ),
        get_elevation_at_point(
            rasterband,
            geotransform,
            raster_size,
            x,
            y - sample_distance,
        ),
    ];

    // Calculate gradients and adjust position
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

        Coord {
            x: x + shift_x,
            y: y + shift_y,
        }
    } else {
        // Return original waypoint if slope calculation fails
        waypoint
    }
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

    if pixel_x < 0
        || pixel_y < 0
        || pixel_x >= raster_size.0 as isize
        || pixel_y >= raster_size.1 as isize
    {
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
fn write_flightpath_kml(waypoints: &[Waypoint], drone: &Drone) -> std::io::Result<()> {
    let mut elements = Vec::new();

    for (i, waypoint) in waypoints.iter().enumerate() {
        let point = kml::types::Point {
            coord: kml::types::Coord {
                x: waypoint.position[0], // longitude
                y: waypoint.position[1], // latitude
                z: None,
            },
            extrude: false,
            altitude_mode: kml::types::AltitudeMode::RelativeToGround,
            attrs: HashMap::new(),
        };

        let placemark = kml::types::Placemark {
            name: Some(format!("Waypoint {}", i + 1)),
            description: Some(format!(
                "Lat: {}, Lon: {}",
                waypoint.position[1], waypoint.position[0]
            )),
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
