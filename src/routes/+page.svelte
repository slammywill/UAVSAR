<script lang="ts">
    import Map from "$lib/components/Map.svelte";
    import Sidebar from "$lib/components/Sidebar.svelte";
</script>

<div class="flex h-screen">
    <div class="w-1/3 md:w-1/5">
        <Sidebar />
    </div>
    <div class="flex-grow">
        <Map />
    </div>
</div>
