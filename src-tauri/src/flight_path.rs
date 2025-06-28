use geo::{
    algorithm::{bounding_rect::BoundingRect, contains::Contains},
    Coord, LineString, Polygon,
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

    let bounding_rect = polygon.bounding_rect().unwrap();

    // Estimate ground coverage dimensions of each photo (simple planar projection)
    let fov_rad = drone.fov.to_radians();
    let footprint = 2.0 * (drone.altitude as f64) * (fov_rad / 2.0).tan(); // meters
    let overlap = drone.overlap as f64 / 100.0;
    let step = footprint * (1.0 - overlap); // spacing between points in meters

    // Convert bounding box lat/lon to meters using a naive equirectangular projection for spacing
    // Assume 1 deg lat ~ 111_000m, lon adjusted by cos(latitude)
    let lat_avg = (bounding_rect.min().y + bounding_rect.max().y) / 2.0;
    let lat_factor = 111_000.0;
    let lon_factor = 111_000.0 * lat_avg.to_radians().cos();

    let mut flight_path = Vec::new();

    let min_lat = bounding_rect.min().y;
    let max_lat = bounding_rect.max().y;
    let min_lon = bounding_rect.min().x;
    let max_lon = bounding_rect.max().x;

    let lat_step_deg = step / lat_factor;
    let lon_step_deg = step / lon_factor;

    let mut y = min_lat;
    let mut toggle = false;

    while y <= max_lat {
        let mut row = Vec::new();
        let mut x = min_lon;

        while x <= max_lon {
            let coord = Coord::from((x, y));
            if polygon.contains(&coord) {
                row.push([x, y]);
            }
            x += lon_step_deg;
        }

        if toggle {
            row.reverse();
        }

        flight_path.extend(row);
        y += lat_step_deg;
        toggle = !toggle;
    }

     FlightPlanResult {waypoints: flight_path, search_area: 0.0, est_flight_time: 0.0}
}
