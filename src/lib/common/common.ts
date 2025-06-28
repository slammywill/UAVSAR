import {
    areaCoordsStore,
    flightPathResultStore,
    droneStore,
    type FlightPathResult,
} from "$lib/stores/stores";
import { invoke } from "@tauri-apps/api/core";
import { get } from "svelte/store";

export async function generateFlightPath() {
    const area_coordinates = get(areaCoordsStore);
    const drone = get(droneStore);

    if (area_coordinates.length < 3) {
        flightPathResultStore.set(null);
        return;
    }

    try {
        const flightPathResult = await invoke<FlightPathResult>(
            "generate_flightpath",
            {
                coords: area_coordinates,
                drone: drone
            });
        flightPathResultStore.set(flightPathResult);
        console.log(flightPathResult)
    } catch (error) {
        console.error("Failed to generate flight path:", error);
        flightPathResultStore.set(null);
    }
}
