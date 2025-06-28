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

    // Calculate the area of the search polygon (approximate using bounding box)
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

    // Calculate ground coverage per photo
    let fov_rad = drone.fov.to_radians();
    let ground_width = 2.0 * (drone.altitude as f64) * (fov_rad / 2.0).tan();
    
    // Calculate spacing between waypoints considering overlap
    let overlap_factor = 1.0 - (drone.overlap as f64 / 100.0);
    let spacing = ground_width * overlap_factor;
    
    // Convert spacing from meters to degrees (approximate)
    // Using rough conversion: 1 degree ≈ 111,000 meters at equator
    let spacing_deg = spacing / 111000.0;

    // Create rotation matrix for the longest side angle
    let cos_angle = longest_side_angle.cos();
    let sin_angle = longest_side_angle.sin();

    // Get the bounding box of the MBR in the rotated coordinate system
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for coord in mbr_coords.iter() {
        // Rotate coordinate to align with longest side
        let rotated_x = coord.x * cos_angle + coord.y * sin_angle;
        let rotated_y = -coord.x * sin_angle + coord.y * cos_angle;
        
        min_x = min_x.min(rotated_x);
        max_x = max_x.max(rotated_x);
        min_y = min_y.min(rotated_y);
        max_y = max_y.max(rotated_y);
    }

    // Generate lawnmower pattern waypoints
    let mut waypoints = Vec::new();
    let mut y = min_y;
    let mut going_right = true;

    while y <= max_y {
        if going_right {
            // Left to right pass
            let mut x = min_x;
            while x <= max_x {
                // Rotate back to original coordinate system
                let world_x = x * cos_angle - y * sin_angle;
                let world_y = x * sin_angle + y * cos_angle;
                let point = Coord { x: world_x, y: world_y };
                
                // Check if the point is inside the original polygon
                if polygon.contains(&point) {
                    waypoints.push([world_x, world_y]);
                }
                x += spacing_deg;
            }
        } else {
            // Right to left pass
            let mut x = max_x;
            while x >= min_x {
                // Rotate back to original coordinate system
                let world_x = x * cos_angle - y * sin_angle;
                let world_y = x * sin_angle + y * cos_angle;
                let point = Coord { x: world_x, y: world_y };
                
                // Check if the point is inside the original polygon
                if polygon.contains(&point) {
                    waypoints.push([world_x, world_y]);
                }
                x -= spacing_deg;
            }
        }
        
        going_right = !going_right;
        y += spacing_deg;
    }

    // Calculate estimated flight time
    let mut total_distance = 0.0;
    for i in 1..waypoints.len() {
        let dx = waypoints[i][0] - waypoints[i-1][0];
        let dy = waypoints[i][1] - waypoints[i-1][1];
        // Convert degrees to meters (approximate)
        let distance_meters = (dx * dx + dy * dy).sqrt() * 111000.0;
        total_distance += distance_meters;
    }
    let est_flight_time = total_distance / drone.speed;

    FlightPlanResult {
        waypoints,
        search_area,
        est_flight_time,
    }
}
