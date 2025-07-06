<script lang="ts">
    import { flightPathResultStore, droneStore } from "$lib/stores/stores";
    import { generateFlightPath } from "$lib/common/common";

    const ALTITUDE = 100;
    const OVERLAP = 55;

    let showDronePopup = false;
    let droneModel = "";
    let droneSpeed = "";
    let cameraFov = "";

    // Initialize local variables with drone store values
    $: {
        if ($droneStore) {
            droneModel = $droneStore.model || "";
            droneSpeed = $droneStore.speed ? $droneStore.speed.toString() : "";
            cameraFov = $droneStore.fov ? $droneStore.fov.toString() : "";
        }
    }

    function formatSearchArea(area: number | undefined | null) {
        return area ? `${area.toFixed(2)} km²` : "—";
    }

    function openDronePopup() {
        // Update local variables with current store values when opening popup
        if ($droneStore) {
            droneModel = $droneStore.model || "";
            droneSpeed = $droneStore.speed ? $droneStore.speed.toString() : "";
            cameraFov = $droneStore.fov ? $droneStore.fov.toString() : "";
        }
        showDronePopup = true;
    }

    function closeDronePopup() {
        showDronePopup = false;
    }

    function saveDroneSettings() {
        // Handle saving drone settings here
        droneStore?.set({
            model: droneModel,
            fov: parseFloat(cameraFov),
            altitude: ALTITUDE,
            overlap: OVERLAP,
            speed: parseFloat(droneSpeed),
        });
        closeDronePopup();
    }

    function handlePopupClick(event: MouseEvent) {
        // Close popup if clicking outside the modal content
        if (event.target === event.currentTarget) {
            closeDronePopup();
        }
    }

    function handlePopupKey(event: KeyboardEvent) {
        if (event.key === "Escape") {
            closeDronePopup();
        }
    }
</script>

<div
    class="h-full p-4 bg-background border-r-2 shadow-md flex flex-col border-accent-1 shadow-xl"
>
    <div class="flex-grow">
        <!-- HEADER -->
        <div class="pb-4 mb-2 border-b border-slate-700/50">
            <h2 class="text-xl font-bold text-white tracking-wide">UAVSAR</h2>
            <p class="text-slate-400 text-sm mt-1">Flight Planning System</p>
        </div>
        <!-- MISSION OVERVIEW -->
        <h2 class="text-l text-slate-400 pb-2 text-center font-mono">
            Mission Overview
        </h2>

        <div
            class="grid grid-cols-1 xl:grid-cols-2 gap-2 border-b border-slate-700/50 pb-4 mb-2"
        >
            <!-- Search Area Card -->
            <div
                class="bg-background-accent/50 rounded-lg p-4 border border-slate-700/30"
            >
                <div class="flex items-center justify-between mb-2">
                    <span class="text-slate-400 text-sm font-medium"
                        >Search Area</span
                    >
                    <div class="w-2 h-2 bg-emerald-500 rounded-full"></div>
                </div>
                <p class="text-white font-mono text-lg">
                    {formatSearchArea($flightPathResultStore?.search_area)}
                </p>
            </div>

            <!-- Est. Flight Time Card -->
            <div
                class="bg-background-accent/50 rounded-lg p-4 border border-slate-700/30"
            >
                <div class="flex items-center justify-between mb-2">
                    <span class="text-slate-400 text-sm font-medium"
                        >Est. Flight Time</span
                    >
                    <div class="w-2 h-2 bg-emerald-500 rounded-full"></div>
                </div>
                <p class="text-white font-mono text-lg">
                    {formatSearchArea($flightPathResultStore?.est_flight_time)}
                </p>
            </div>
        </div>
        <!-- DRONE SETTINGS -->
        <h2 class="text-l text-slate-400 pb-2 text-center font-mono">
            Drone Settings
        </h2>
        <div class="border-b border-slate-700/50 pb-4 mb-2">
            <button
                class="w-full bg-background-accent/50 rounded-lg p-4 border border-slate-700/30 hover:bg-background-accent/70 transition-colors text-left"
                on:click={openDronePopup}
            >
                <div class="flex items-center justify-between mb-2">
                    <span class="text-slate-400 text-sm font-medium"
                        >Configure Drone</span
                    >
                    <div class="w-2 h-2 bg-blue-500 rounded-full"></div>
                </div>

                {#if $droneStore?.model && $droneStore?.speed && $droneStore?.fov}
                    <p class="text-white font-mono text-sm">
                        Model: {$droneStore.model}<br />
                        Speed: {$droneStore.speed} km/h<br />
                        FOV: {$droneStore.fov}&deg;
                    </p>
                {:else}
                    <p class="text-white font-mono text-sm">Click to set</p>
                {/if}
            </button>
        </div>
    </div>
    <button
        class="p-4 font-semibold align-bottom text-white grad-fill"
        on:click={generateFlightPath}
    >
        Generate Flightpath
    </button>
</div>

<!-- Drone Settings Popup -->
{#if showDronePopup}
    <div
        class="fixed inset-0 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        aria-labelledby="drone-config-title"
        tabindex="-1"
        on:click={handlePopupClick}
        on:keydown={handlePopupKey}
    >
        <div
            class="bg-background border border-slate-700/50 rounded-lg p-6 w-96 max-w-md mx-4 shadow-xl"
        >
            <div class="pb-4 mb-4 border-b border-slate-700/50">
                <h3
                    id="drone-config-title"
                    class="text-lg font-bold text-white tracking-wide"
                >
                    Drone Configuration
                </h3>
                <p class="text-slate-400 text-sm mt-1">
                    Enter your drone specifications
                </p>
            </div>

            <div class="space-y-4">
                <!-- Model Name -->
                <div>
                    <label
                        for="droneModel"
                        class="block text-slate-400 text-sm font-medium mb-2"
                    >
                        Drone Model
                    </label>
                    <input
                        id="droneModel"
                        type="text"
                        bind:value={droneModel}
                        placeholder="e.g., DJI Mavic 3"
                        class="w-full px-3 py-2 bg-background-accent/50 border border-slate-700/30 rounded-md text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                    />
                </div>

                <!-- Speed -->
                <div>
                    <label
                        for="droneSpeed"
                        class="block text-slate-400 text-sm font-medium mb-2"
                    >
                        Speed (km/h)
                    </label>
                    <input
                        id="droneSpeed"
                        type="number"
                        bind:value={droneSpeed}
                        placeholder="e.g., 65"
                        class="w-full px-3 py-2 bg-background-accent/50 border border-slate-700/30 rounded-md text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                    />
                </div>

                <!-- Camera FOV -->
                <div>
                    <label
                        for="cameraFov"
                        class="block text-slate-400 text-sm font-medium mb-2"
                    >
                        Camera FOV (degrees)
                    </label>
                    <input
                        id="cameraFov"
                        type="number"
                        bind:value={cameraFov}
                        placeholder="e.g., 84"
                        class="w-full px-3 py-2 bg-background-accent/50 border border-slate-700/30 rounded-md text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent"
                    />
                </div>
            </div>

            <!-- Action Buttons -->
            <div class="flex gap-3 mt-6">
                <button
                    class="flex-1 px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-md transition-colors"
                    on:click={closeDronePopup}
                >
                    Cancel
                </button>
                <button
                    class="flex-1 px-4 py-2 grad-fill text-white rounded-md font-semibold"
                    on:click={saveDroneSettings}
                >
                    Save Settings
                </button>
            </div>
        </div>
    </div>
{/if}
