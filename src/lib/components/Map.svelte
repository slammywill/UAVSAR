<script lang="ts">
    import { onMount } from "svelte";
    import {
        areaCoordsStore,
        flightPathResultStore,
        type FlightPathResult,
    } from "$lib/stores/stores";
    import maplibregl from "maplibre-gl";
    import type {
        Feature,
        FeatureCollection,
        LineString,
        Point,
        Polygon,
    } from "geojson";

    let mapContainer: HTMLDivElement;
    let area_coordinates: [number, number][] = [];
    let isDragging = false;
    let selectedPointId: number | null = null;
    let map: maplibregl.Map;
    let coverageVisible = false;
    let coverageColor = "#0000FF";

    function resetArea() {
        area_coordinates = [];
        areaCoordsStore.set([]);
        flightPathResultStore.set(null);
        const emptyPointData: FeatureCollection<Point> = {
            type: "FeatureCollection",
            features: [],
        };
        const emptyPolygonData: FeatureCollection<Polygon> = {
            type: "FeatureCollection",
            features: [],
        };

        const flightPathSource = map?.getSource(
            "flight-path",
        ) as maplibregl.GeoJSONSource;
        const pointSource = map?.getSource(
            "click-points",
        ) as maplibregl.GeoJSONSource;
        const polygonSource = map?.getSource(
            "drawn-polygon",
        ) as maplibregl.GeoJSONSource;
        const coverageRectsSource = map?.getSource(
            "coverage-rects",
        ) as maplibregl.GeoJSONSource;
        const coverageLinesSource = map?.getSource(
            "coverage-lines",
        ) as maplibregl.GeoJSONSource;

        if (pointSource && polygonSource && flightPathSource) {
            flightPathSource.setData(emptyPointData);
            pointSource.setData(emptyPointData);
            polygonSource.setData(emptyPolygonData);
            coverageRectsSource.setData(emptyPointData);
            coverageLinesSource.setData(emptyPointData);
        }
    }

    function toggleCoverage() {
        coverageVisible = !coverageVisible;
        const visibility = coverageVisible ? "visible" : "none";

        if (map.getLayer("coverage-rects-fill")) {
            map.setLayoutProperty(
                "coverage-rects-fill",
                "visibility",
                visibility,
            );
        }
        if (map.getLayer("coverage-rects-outline")) {
            map.setLayoutProperty(
                "coverage-rects-outline",
                "visibility",
                visibility,
            );
        }

        if (map.getLayer("coverage-lines-layer")) {
            map.setLayoutProperty(
                "coverage-lines-layer",
                "visibility",
                visibility,
            );
        }
    }

    onMount(() => {
        map = new maplibregl.Map({
            container: mapContainer,
            style: "https://demotiles.maplibre.org/style.json",
            center: [172.5, -41.2],
            zoom: 5,
        });

        const canvas = map.getCanvasContainer();
        map.addControl(new maplibregl.NavigationControl(), "top-right");
        map.doubleClickZoom.disable();

        map.on("load", () => {
            // Topo tile map source
            map.addSource("xyz-tiles", {
                type: "raster",
                tiles: [
                    "https://tiles-cdn.koordinates.com/services;key=00c7539f05a240adbe2f0285fff024fe/tiles/v4/layer=50767/EPSG:3857/{z}/{x}/{y}.png",
                ],
                tileSize: 256,
            });

            // Topo tile map layer
            map.addLayer({
                id: "xyz-tiles-layer",
                type: "raster",
                source: "xyz-tiles",
                paint: {},
            });

            // Search area source
            map.addSource("drawn-polygon", {
                type: "geojson",
                data: {
                    type: "FeatureCollection",
                    features: [],
                },
            });

            // Search area fill layer
            map.addLayer({
                id: "polygon-fill",
                type: "fill",
                source: "drawn-polygon",
                paint: {
                    "fill-color": "#00c951",
                    "fill-opacity": 0.2,
                },
            });

            // Search area outline layer
            map.addLayer({
                id: "polygon-outline",
                type: "line",
                source: "drawn-polygon",
                layout: {},
                paint: {
                    "line-color": "#00c951",
                    "line-width": 2,
                },
            });

            // Search area vertices source
            map.addSource("click-points", {
                type: "geojson",
                data: { type: "FeatureCollection", features: [] },
            });

            // Search area vertices layer (points)
            map.addLayer({
                id: "click-points",
                type: "circle",
                source: "click-points",
                paint: {
                    "circle-radius": 4,
                    "circle-color": "#1e2939",
                },
            });

            map.addSource("coverage-rects", {
                type: "geojson",
                data: { type: "FeatureCollection", features: [] },
            });

            map.addLayer({
                id: "coverage-rects-fill",
                type: "fill",
                source: "coverage-rects",
                paint: {
                    "fill-color": coverageColor,
                    "fill-opacity": 0.1,
                },
            });

            map.addLayer({
                id: "coverage-rects-outline",
                type: "line",
                source: "coverage-rects",
                layout: {
                    "line-join": "round",
                    "line-cap": "round",
                },
                paint: {
                    "line-color": coverageColor,
                    "line-width": 1,
                    "line-opacity": 0.2,
                },
            });

            map.addSource("coverage-lines", {
                type: "geojson",
                data: { type: "FeatureCollection", features: [] },
            });

            map.addLayer({
                id: "coverage-lines-layer",
                type: "line",
                source: "coverage-lines",
                layout: {
                    "line-join": "round",
                    "line-cap": "round",
                },
                paint: {
                    "line-color": coverageColor,
                    "line-width": 2,
                    "line-dasharray": [1, 2],
                },
            });

            map.addSource("flight-path", {
                type: "geojson",
                data: {
                    type: "FeatureCollection",
                    features: [],
                },
            });

            map.addLayer({
                id: "flight-path-line",
                type: "line",
                source: "flight-path",
                layout: {
                    "line-join": "round",
                    "line-cap": "round",
                },
                paint: {
                    "line-color": "#000000",
                    "line-width": 3,
                },
            });

            flightPathResultStore.subscribe(
                (result: FlightPathResult | null) => {
                    if (!result) return;

                    // Flight path lines
                    const coords = result.waypoints.map((wp) => wp.position);

                    if (
                        !map ||
                        !map.getSource("flight-path") ||
                        !map.getSource("coverage-rects")
                    )
                        return;

                    const fp_source = map.getSource(
                        "flight-path",
                    ) as maplibregl.GeoJSONSource;

                    if (coords.length < 2) {
                        fp_source.setData({
                            type: "FeatureCollection",
                            features: [],
                        });
                        return;
                    }

                    // waypoint lines
                    const lineFeature: Feature<LineString> = {
                        type: "Feature",
                        geometry: {
                            type: "LineString",
                            coordinates: coords,
                        },
                        properties: {},
                    };

                    const flightPathData: FeatureCollection<LineString> = {
                        type: "FeatureCollection",
                        features: [lineFeature],
                    };

                    fp_source.setData(flightPathData);

                    // Coverage rects
                    const rectFeatures = result.waypoints.map((wp) => ({
                        type: "Feature",
                        geometry: {
                            type: "Polygon",
                            coordinates: [wp.coverage_rect.coords],
                        },
                        properties: {
                            center: wp.coverage_rect.center,
                        },
                    }));

                    const coverageRectSource = map.getSource(
                        "coverage-rects",
                    ) as maplibregl.GeoJSONSource;
                    coverageRectSource.setData({
                        type: "FeatureCollection",
                        features: rectFeatures,
                    });

                    // coverage-waypoint connector lines
                    const lineFeatures = result.waypoints.map((wp) => ({
                        type: "Feature",
                        geometry: {
                            type: "LineString",
                            coordinates: [
                                wp.position, // waypoint
                                wp.coverage_rect.center, // rectangle center
                            ],
                        },
                        properties: {},
                    }));

                    const coverageLinesSource = map.getSource(
                        "coverage-lines",
                    ) as maplibregl.GeoJSONSource;
                    coverageLinesSource.setData({
                        type: "FeatureCollection",
                        features: lineFeatures,
                    });
                },
            );
        });

        // Add a new point to the search area on double click
        map.on("dblclick", (point) => {
            const lngLat: [number, number] = [
                point.lngLat.lng,
                point.lngLat.lat,
            ];
            area_coordinates.push(lngLat);
            areaCoordsStore.set([...area_coordinates]);

            // Create the new search area polygon feature collection
            const polygon: FeatureCollection<Polygon> = {
                type: "FeatureCollection",
                features: [
                    {
                        type: "Feature",
                        geometry: {
                            type: "Polygon",
                            coordinates: [
                                area_coordinates.concat([area_coordinates[0]]),
                            ],
                        },
                        properties: {},
                    },
                ],
            };
            // Reset the search area polygon when a new point is added
            const source = map.getSource(
                "drawn-polygon",
            ) as maplibregl.GeoJSONSource;
            source.setData(polygon);

            // Create the new search area points feature collection
            const pointData: FeatureCollection<Point> = {
                type: "FeatureCollection",
                features: area_coordinates.map((coord, index) => ({
                    type: "Feature",
                    id: index,
                    geometry: {
                        type: "Point",
                        coordinates: coord,
                    },
                    properties: {},
                })),
            };

            // Reset the search area vertices when a new point is added
            const pointSource = map.getSource(
                "click-points",
            ) as maplibregl.GeoJSONSource;
            pointSource.setData(pointData);
        });

        // Change cursor when hovering over a draggable point
        map.on("mouseenter", "click-points", () => {
            canvas.style.cursor = "move";
        });

        // Change cursor back to default after leaving hover over draggable point
        map.on("mouseleave", "click-points", () => {
            canvas.style.cursor = "";
        });

        // Find the selected point when a user clicks on a search area vertice
        map.on("mousedown", "click-points", (e) => {
            if (!e.features || e.features.length === 0) return;
            e.preventDefault();
            isDragging = true;

            const feature = e.features[0];
            selectedPointId = feature.id as number;
        });

        // Move the search area vertice on drag when clicked on it
        map.on("mousemove", (e) => {
            if (!isDragging || selectedPointId === null) return;

            area_coordinates[selectedPointId] = [e.lngLat.lng, e.lngLat.lat];
            areaCoordsStore.set([...area_coordinates]);

            const pointData: FeatureCollection<Point> = {
                type: "FeatureCollection",
                features: area_coordinates.map((coord, index) => ({
                    type: "Feature",
                    id: index,
                    geometry: {
                        type: "Point",
                        coordinates: coord,
                    },
                    properties: {},
                })),
            };
            const pointSource = map.getSource(
                "click-points",
            ) as maplibregl.GeoJSONSource;
            pointSource.setData(pointData);

            // Update the polygon layer
            const polygon: FeatureCollection<Polygon> = {
                type: "FeatureCollection",
                features: [
                    {
                        type: "Feature",
                        geometry: {
                            type: "Polygon",
                            coordinates: [
                                area_coordinates.concat([area_coordinates[0]]),
                            ],
                        },
                        properties: {},
                    },
                ],
            };
            const polygonSource = map.getSource(
                "drawn-polygon",
            ) as maplibregl.GeoJSONSource;
            polygonSource.setData(polygon);
        });

        // Reset to normal after moving point
        map.on("mouseup", () => {
            if (!isDragging) return;
            map.getCanvas().style.cursor = "";
            isDragging = false;
            selectedPointId = null;
        });
    });
</script>

<div bind:this={mapContainer} class="w-full h-screen">
    <div class="z-10 m-2 absolute">
        <button
            on:click={resetArea}
            class="p-2 text-md font-bold text-black bg-white rounded-sm hover:bg-neutral-100 shadow-[0_0_0_2px_rgba(0,0,0,0.1)]"
        >
            Reset Area
        </button>
        <button
            on:click={toggleCoverage}
            class="p-2 text-md font-bold text-black bg-white rounded-sm hover:bg-neutral-100 shadow-[0_0_0_2px_rgba(0,0,0,0.1)]"
        >
            Toggle Coverage
        </button>
    </div>
</div>
