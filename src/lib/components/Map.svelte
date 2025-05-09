<script lang="ts">
  import { onMount } from 'svelte';
  import maplibregl from 'maplibre-gl';

  let mapContainer: HTMLDivElement;

  onMount(() => {
    const map = new maplibregl.Map({
      container: mapContainer,
      style: 'https://demotiles.maplibre.org/style.json',
      center: [0, 0],
      zoom: 2
    });

    map.addControl(new maplibregl.NavigationControl(), 'top-right');
  });
</script>

<style>
  .map-container {
    width: 100%;
    height: 100vh;
  }
</style>

<div bind:this={mapContainer} class="map-container"></div>
