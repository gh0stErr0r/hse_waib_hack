<script setup>
import { onMounted, onBeforeUnmount, ref, watch } from "vue";
import "leaflet/dist/leaflet.css";
const props = defineProps({
  cards: { type: Array, default: () => [] },
  center: { type: Object, default: null },
  color: { type: String, default: "#d79921" },
});
const element = ref(null),
  error = ref("");
let map,
  markers,
  L,
  observer,
  disposed = false;
function render() {
  if (!map) return;
  markers.clearLayers();
  const color = /^#[0-9a-f]{6}$/i.test(props.color) ? props.color : "#d79921";
  for (const [index, card] of props.cards.entries()) {
    const { place } = card;
    const content = document.createElement("div");
    const title = document.createElement("strong");
    title.textContent = place.name;
    const address = document.createElement("p");
    address.textContent = place.address;
    content.append(title, address);
    const icon = L.divIcon({
      className: "venue-marker",
      html: `<span style="background:${color}">${index + 1}</span>`,
      iconSize: [32, 40],
      iconAnchor: [16, 36],
      popupAnchor: [0, -32],
    });
    L.marker([place.coordinates.lat, place.coordinates.lon], {
      icon,
      title: place.name,
      alt: place.name,
    })
      .bindPopup(content)
      .addTo(markers);
  }
  if (props.cards.length)
    map.fitBounds(markers.getBounds(), {
      padding: [45, 45],
      maxZoom: 16,
      animate: false,
    });
  else if (props.center)
    map.setView([props.center.lat, props.center.lon], 13, { animate: false });
  map.invalidateSize();
}
onMounted(async () => {
  try {
    L = await import("leaflet");
    if (disposed) return;
    map = L.map(element.value, { scrollWheelZoom: false }).setView(
      props.center ? [props.center.lat, props.center.lon] : [59.9343, 30.3351],
      12,
    );
    const tiles = L.tileLayer(
      import.meta.env.VITE_MAP_TILES_URL ||
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
      {
        maxZoom: 19,
        attribution:
          import.meta.env.VITE_MAP_ATTRIBUTION ||
          '&copy; <a href="https://www.openstreetmap.org/copyright" target="_blank" rel="noopener noreferrer">OpenStreetMap</a> contributors',
      },
    );
    tiles.on("tileerror", () => {
      error.value =
        "Подложка карты временно недоступна. Метки и карточки сохранены.";
    });
    tiles.on("tileload", () => {
      error.value = "";
    });
    tiles.addTo(map);
    markers = L.featureGroup().addTo(map);
    observer = new ResizeObserver(() => map?.invalidateSize());
    observer.observe(element.value);
    render();
  } catch {
    error.value = "Не удалось открыть карту. Адреса доступны в карточках.";
  }
});
watch(() => [props.cards, props.center, props.color], render, { deep: true });
onBeforeUnmount(() => {
  disposed = true;
  observer?.disconnect();
  map?.remove();
  map = null;
});
</script>
<template>
  <div class="map-shell">
    <div
      ref="element"
      class="map"
      aria-label="Заведения выбранного слота на карте OpenStreetMap"
    />
    <div v-if="error" class="map-notice" role="status">{{ error }}</div>
    <span class="map-caption">Места выбранного слота · без маршрута</span>
  </div>
</template>
