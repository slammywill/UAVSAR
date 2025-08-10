import { writable } from "svelte/store";

export type Coordinate = [number, number];
export const areaCoordsStore = writable<Coordinate[]>([]);
export const flightPathResultStore = writable<FlightPathResult | null>(null);
export const droneStore = writable<Drone | null>(null);

export interface Drone {
    model: string,
    fov: number,
    altitude: number,
    overlap: number,
    speed: number,
};

export interface FlightPathResult {
    waypoints: Waypoint[];
    search_area: number,
    est_flight_time: number,
}

export interface Waypoint {
    coverage_rect: CoverageRect,
    position: Coordinate,
    bearing: number,
    altitude: number,
}

export interface CoverageRect {
    coords: Coordinate[][],
    center: Coordinate
}

