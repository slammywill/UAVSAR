use geo::{
    algorithm::{bounding_rect::BoundingRect, contains::Contains}, 
    Coord, LineString, MinimumRotatedRect, Polygon
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Drone {
    pub fov: f64,       // degrees
    pub altitude: i32,  // meters
    pub overlap: i32,   // percent
    pub speed: f64,     // ms^-1
}

#[derive(Serialize, Deserialize)]
pub struct FlightPlanResult {
    pub waypoints: Vec<[f64; 2]>,
    pub search_area: f64,
    pub est_flight_time: f64,
}

#[tauri::command]
pub fn generate_flightpath(coords: Vec<[f64; 2]>, drone: Drone) -> FlightPlanResult {
    let points: Vec<Coord> = coords.iter().map(|c| Coord::from((c[0], c[1]))).collect();
    let polygon = Polygon::new(LineString::from(points.clone()), vec![]);
    let mbr = MinimumRotatedRect::minimum_rotated_rect(&polygon).unwrap();

    // Calculate the area of the search polygon
    let bbox = polygon.bounding_rect().unwrap();
    let search_area = (bbox.max().x - bbox.min().x) * (bbox.max().y - bbox.min().y);

    // Get MBR vertices and find the longest side
    let mbr_coords = mbr.exterior().coords().collect::<Vec<_>>();
    let mut max_length = 0.0;
    let mut longest_side_angle = 0.0;
    
    for i in 0..mbr_coords.len() - 1 {
        let p1 = mbr_coords[i];
        let p2 = mbr_coords[i + 1];
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let length = (dx * dx + dy * dy).sqrt();
        
        if length > max_length {
            max_length = length;
            longest_side_angle = dy.atan2(dx);
        }
    }

    // Calculate ground coverage per photo and spacing
    let fov_rad = drone.fov.to_radians();
    let ground_width = 2.0 * (drone.altitude as f64) * (fov_rad / 2.0).tan();
    let overlap_factor = 1.0 - (drone.overlap as f64 / 100.0);
    let spacing_meters = ground_width * overlap_factor;

    // Convert all coordinates to a local meter-based coordinate system
    // Use the center of the bounding box as origin
    let center_x = (bbox.max().x + bbox.min().x) / 2.0;
    let center_y = (bbox.max().y + bbox.min().y) / 2.0;
    
    // Convert degrees to meters (flat plane approximation)
    let meters_per_degree = 111320.0; // approximate at mid-latitudes
    
    // Convert polygon to meter coordinates
    let polygon_meters: Vec<Coord> = points.iter().map(|p| {
        Coord {
            x: (p.x - center_x) * meters_per_degree,
            y: (p.y - center_y) * meters_per_degree,
        }
    }).collect();
    let polygon_m = Polygon::new(LineString::from(polygon_meters), vec![]);
    
    // Convert MBR to meter coordinates and recalculate
    let mbr_meters: Vec<Coord> = mbr_coords.iter().map(|p| {
        Coord {
            x: (p.x - center_x) * meters_per_degree,
            y: (p.y - center_y) * meters_per_degree,
        }
    }).collect();

    // Find bounding box of MBR in meter coordinates
    let cos_angle = longest_side_angle.cos();
    let sin_angle = longest_side_angle.sin();
    
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for coord in mbr_meters.iter() {
        // Rotate coordinate to align with longest side
        let rotated_x = coord.x * cos_angle + coord.y * sin_angle;
        let rotated_y = -coord.x * sin_angle + coord.y * cos_angle;
        
        min_x = min_x.min(rotated_x);
        max_x = max_x.max(rotated_x);
        min_y = min_y.min(rotated_y);
        max_y = max_y.max(rotated_y);
    }

    // Generate waypoints in meter space with consistent spacing
    let mut waypoints = Vec::new();
    let mut y = min_y;
    let mut going_right = true;

    while y <= max_y {
        if going_right {
            // Left to right pass
            let mut x = min_x;
            while x <= max_x {
                // Rotate back to meter coordinate system
                let meter_x = x * cos_angle - y * sin_angle;
                let meter_y = x * sin_angle + y * cos_angle;
                let point_m = Coord { x: meter_x, y: meter_y };
                
                // Check if the point is inside the polygon
                if polygon_m.contains(&point_m) {
                    // Convert back to lat/lon
                    let world_x = center_x + meter_x / meters_per_degree;
                    let world_y = center_y + meter_y / meters_per_degree;
                    waypoints.push([world_x, world_y]);
                }
                x += spacing_meters;
            }
        } else {
            // Right to left pass
            let mut x = max_x;
            while x >= min_x {
                // Rotate back to meter coordinate system
                let meter_x = x * cos_angle - y * sin_angle;
                let meter_y = x * sin_angle + y * cos_angle;
                let point_m = Coord { x: meter_x, y: meter_y };
                
                // Check if the point is inside the polygon
                if polygon_m.contains(&point_m) {
                    // Convert back to lat/lon
                    let world_x = center_x + meter_x / meters_per_degree;
                    let world_y = center_y + meter_y / meters_per_degree;
                    waypoints.push([world_x, world_y]);
                }
                x -= spacing_meters;
            }
        }
        
        going_right = !going_right;
        y += spacing_meters;
    }

    // Calculate flight time using meter distances
    let mut total_distance = 0.0;
    for i in 1..waypoints.len() {
        let dx = (waypoints[i][0] - waypoints[i-1][0]) * meters_per_degree;
        let dy = (waypoints[i][1] - waypoints[i-1][1]) * meters_per_degree;
        let distance_meters = (dx * dx + dy * dy).sqrt();
        total_distance += distance_meters;
    }
    let est_flight_time = total_distance / drone.speed;

    FlightPlanResult {
        waypoints,
        search_area,
        est_flight_time,
    }
}
