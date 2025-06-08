import { writable } from "svelte/store";

export type Coordinate = [number, number];

export const areaCoordsStore = writable<Coordinate[]>([]);
export const flightPathCoordsStore = writable<Coordinate[]>([]);
