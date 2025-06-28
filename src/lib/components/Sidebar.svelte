<script lang="ts">
    import { flightPathResultStore } from "$lib/stores/stores";
    import { generateFlightPath } from "$lib/common/common";

    function formatSearchArea(area: number | undefined | null) {
        return area ? `${area.toFixed(2)} km²` : "—";
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
    </div>
    <button
        class="p-4 font-semibold align-bottom text-white grad-fill"
        on:click={generateFlightPath}
    >
        Generate Flightpath
    </button>
</div>
