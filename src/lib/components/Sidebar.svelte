<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { get } from "svelte/store";
    import { areaCoordsStore, flightPathCoordsStore } from "$lib/stores/stores";

    let flightCoords: [number, number][] = [];

    async function generateFlightpath() {
        const coords = get(areaCoordsStore);

        flightCoords = await invoke<[number, number][]>("generate_flightpath", {
            coords: coords,
            drone: {
                fov: 60, // example FoV in degrees
                altitude: 100, // example altitude in meters
                overlap: 55, // example 20% overlap
            },
        });

        flightPathCoordsStore.set(flightCoords);
    }
</script>

<div
    class="h-full p-4 bg-background border-r-2 shadow-md flex flex-col border-accent-1 shadow-xl"
>
    <div class="flex-grow">
        <h2 class="text-lg font-semibold mb-4 text-white">UAVSAR</h2>
        <p class="text-white">Search area: {areaCoordsStore}</p>
        <p class="text-white">Estimated flight time:</p>
    </div>
    <button
        class="p-4 align-bottom text-white grad-fill"
        on:click={generateFlightpath}
    >
        Generate Flightpath
    </button>
</div>
