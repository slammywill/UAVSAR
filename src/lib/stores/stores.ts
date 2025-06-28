import { writable } from "svelte/store";

export type Coordinate = [number, number];

export const areaCoordsStore = writable<Coordinate[]>([]);
export const flightPathResultStore = writable<FlightPathResult | null>(null);
export const droneStore = writable<Drone | null>({model: "Test", fov: 60, altitude: 100, overlap: 55, speed: 10.0});

export interface Drone {
    model: string,
    fov: number,
    altitude: number,
    overlap: number,
    speed: number,
};

export interface FlightPathResult {
    waypoints: Coordinate[];
    search_area: number,
    est_flight_time: number,
}
