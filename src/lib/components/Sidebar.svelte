<script lang="ts">
    import {
        flightPathResultStore,
        droneStore,
        type Drone,
    } from "$lib/stores/stores";
    import { generateFlightPath } from "$lib/common/common";
    import { saveDroneTypes, loadDroneTypes } from "$lib/stores/droneTypes";
    import { onMount } from "svelte";

    const ALTITUDE = 100;
    const OVERLAP = 55;
    const MIN_SPEED = 0.5; // Minimum reasonable speed in m/s
    const MAX_SPEED = 15; // Maximum reasonable speed in m/s
    const MIN_FOV = 10; // Minimum reasonable FOV
    const MAX_FOV = 180; // Maximum FOV

    let showDronePopup = false;
    let droneModel = "";
    let droneSpeed: number | undefined;
    let cameraFov: number | undefined;

    let droneTypes: Drone[] = [];

    let selectedDrone = "Custom";

    // Input string values for controlled inputs
    let droneSpeedInput = "";
    let cameraFovInput = "";

    // Validation state
    let modelError = "";
    let speedError = "";
    let fovError = "";

    onMount(async () => {
        try {
            droneTypes = await loadDroneTypes();
        } catch (error) {
            console.error("Failed to load drone types:", error);
        }
    });

    // Reactive validation
    $: {
        // Validate model
        if (!droneModel?.trim()) {
            modelError = "Drone model is required";
        } else if (droneModel.trim().length < 2) {
            modelError = "Drone model must be at least 2 characters";
        } else {
            modelError = "";
        }

        // Validate speed
        if (!droneSpeedInput.trim()) {
            speedError = "Speed is required";
            droneSpeed = undefined;
        } else {
            const speed = parseFloat(droneSpeedInput);
            if (isNaN(speed)) {
                speedError = "Speed must be a valid number";
                droneSpeed = undefined;
            } else if (speed < MIN_SPEED) {
                speedError = `Speed must be at least ${MIN_SPEED} m/s`;
                droneSpeed = undefined;
            } else if (speed > MAX_SPEED) {
                speedError = `Speed must not exceed ${MAX_SPEED} m/s`;
                droneSpeed = undefined;
            } else {
                speedError = "";
                droneSpeed = speed;
            }
        }

        // Validate FOV
        if (!cameraFovInput.trim()) {
            fovError = "Camera FOV is required";
            cameraFov = undefined;
        } else {
            const fov = parseFloat(cameraFovInput);
            if (isNaN(fov)) {
                fovError = "FOV must be a valid number";
                cameraFov = undefined;
            } else if (fov < MIN_FOV) {
                fovError = `FOV must be at least ${MIN_FOV} degrees`;
                cameraFov = undefined;
            } else if (fov > MAX_FOV) {
                fovError = `FOV must not exceed ${MAX_FOV} degrees`;
                cameraFov = undefined;
            } else {
                fovError = "";
                cameraFov = fov;
            }
        }
    }

    // Check if form is valid
    $: isSaveDisabled =
        !droneModel?.trim() ||
        droneSpeed === undefined ||
        cameraFov === undefined ||
        modelError !== "" ||
        speedError !== "" ||
        fovError !== "";

    function formatSearchArea(area: number | undefined | null) {
        return area ? `${area.toFixed(2)} km²` : "—";
    }

    function formatFlightTime(time: number | undefined | null) {
        return time ? `${time.toFixed(2)} mins` : "-";
    }

    function openDronePopup() {
        // Reset form when opening
        droneModel = $droneStore?.model || "";
        droneSpeedInput = $droneStore?.speed?.toString() || "";
        cameraFovInput = $droneStore?.fov?.toString() || "";

        // Clear errors
        modelError = "";
        speedError = "";
        fovError = "";

        showDronePopup = true;
    }

    function closeDronePopup() {
        showDronePopup = false;
    }

    function saveDroneSettings() {
        // Double-check validation before saving
        if (isSaveDisabled) return;

        droneStore?.set({
            model: droneModel.trim(),
            fov: cameraFov!,
            altitude: ALTITUDE,
            overlap: OVERLAP,
            speed: droneSpeed!,
        });
        closeDronePopup();
    }

    function handlePopupClick(event: MouseEvent) {
        if (event.target === event.currentTarget) {
            closeDronePopup();
        }
    }

    function handlePopupKey(event: KeyboardEvent) {
        if (event.key === "Escape") {
            closeDronePopup();
        }
    }

    function handleSpeedInput(event: Event) {
        const target = event.target as HTMLInputElement;
        // Allow only numbers and decimal point
        const filtered = target.value.replace(/[^\d.]/g, "");
        // Prevent multiple decimal points
        const parts = filtered.split(".");
        if (parts.length > 2) {
            droneSpeedInput = parts[0] + "." + parts.slice(1).join("");
        } else {
            droneSpeedInput = filtered;
        }
    }

    function handleFovInput(event: Event) {
        const target = event.target as HTMLInputElement;
        // Allow only numbers and decimal point
        const filtered = target.value.replace(/[^\d.]/g, "");
        // Prevent multiple decimal points
        const parts = filtered.split(".");
        if (parts.length > 2) {
            cameraFovInput = parts[0] + "." + parts.slice(1).join("");
        } else {
            cameraFovInput = filtered;
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
                    {formatFlightTime($flightPathResultStore?.est_flight_time)}
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
                        Speed: {$droneStore.speed} m/s<br />
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
                        for="dronePreset"
                        class="block text-slate-400 text-sm font-medium mb-2"
                        >Drone Preset
                    </label>
                    <select id="dronePreset" bind:value={selectedDrone}>
                        {#each droneTypes as droneType}
                            <option value={droneType}>{droneType.model}</option>
                        {/each}
                    </select>
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
                        class={`w-full px-3 py-2 bg-background-accent/50 border rounded-md text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:border-transparent ${
                            modelError
                                ? "border-red-500 focus:ring-red-500"
                                : "border-slate-700/30 focus:ring-emerald-500"
                        }`}
                    />
                    {#if modelError}
                        <p class="text-red-400 text-xs mt-1">
                            {modelError}
                        </p>
                    {/if}
                </div>

                <!-- Speed -->
                <div>
                    <label
                        for="droneSpeed"
                        class="block text-slate-400 text-sm font-medium mb-2"
                    >
                        Speed (m/s)
                    </label>
                    <input
                        id="droneSpeed"
                        type="text"
                        bind:value={droneSpeedInput}
                        on:input={handleSpeedInput}
                        placeholder={`${MIN_SPEED}-${MAX_SPEED}`}
                        class={`w-full px-3 py-2 bg-background-accent/50 border rounded-md text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:border-transparent ${
                            speedError
                                ? "border-red-500 focus:ring-red-500"
                                : "border-slate-700/30 focus:ring-emerald-500"
                        }`}
                    />
                    {#if speedError}
                        <p class="text-red-400 text-xs mt-1">{speedError}</p>
                    {/if}
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
                        type="text"
                        bind:value={cameraFovInput}
                        on:input={handleFovInput}
                        placeholder={`${MIN_FOV}-${MAX_FOV}º`}
                        class={`w-full px-3 py-2 bg-background-accent/50 border rounded-md text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:border-transparent ${
                            fovError
                                ? "border-red-500 focus:ring-red-500"
                                : "border-slate-700/30 focus:ring-emerald-500"
                        }`}
                    />
                    {#if fovError}
                        <p class="text-red-400 text-xs mt-1">{fovError}</p>
                    {/if}
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
                    class={`flex-1 px-4 py-2 text-white rounded-md font-semibold transition-colors duration-200 ${
                        isSaveDisabled
                            ? "bg-slate-600 cursor-not-allowed"
                            : "grad-fill hover:opacity-90"
                    }`}
                    on:click={saveDroneSettings}
                    disabled={isSaveDisabled}
                >
                    Save Settings
                </button>
            </div>
        </div>
    </div>
{/if}
