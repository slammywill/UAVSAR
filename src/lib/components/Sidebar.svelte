<script lang="ts">
    import { areaCoordsStore } from "$lib/stores/stores";
    import { generateFlightPath } from "$lib/common/common";

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
        on:click={generateFlightPath}
    >
        Generate Flightpath
    </button>
</div>
