<script setup>
import { onMounted, onBeforeUnmount, ref, watch } from "vue";
const props = defineProps({
  cards: { type: Array, default: () => [] },
  center: { type: Object, default: null },
  color: { type: String, default: "#d79921" },
});
const container = ref(null),
  error = ref("");
let map,
  disposed = false;
let scriptPromise;
function loadMaps() {
  if (window.ymaps)
    return new Promise((resolve) =>
      window.ymaps.ready(() => resolve(window.ymaps)),
    );
  if (window.__gastroMaps) return window.__gastroMaps;
  const key = import.meta.env.VITE_YANDEX_MAPS_KEY;
  if (!key)
    return Promise.reject(
      new Error(
        "Для карты нужен VITE_YANDEX_MAPS_KEY. Подборка доступна в карточках.",
      ),
    );
  scriptPromise = new Promise((resolve, reject) => {
    const script = document.createElement("script");
    const timer = setTimeout(
      () => reject(new Error("Карта не ответила. Проверьте сеть и ключ API.")),
      20000,
    );
    script.src = `https://api-maps.yandex.ru/2.1/?apikey=${encodeURIComponent(key)}&lang=ru_RU`;
    script.onload = () => {
      if (!window.ymaps) {
        clearTimeout(timer);
        reject(new Error("API карты недоступен"));
        return;
      }
      window.ymaps.ready(() => {
        clearTimeout(timer);
        resolve(window.ymaps);
      });
    };
    script.onerror = () => {
      clearTimeout(timer);
      reject(new Error("Не удалось загрузить Яндекс Карты"));
    };
    document.head.appendChild(script);
  });
  window.__gastroMaps = scriptPromise.catch((e) => {
    window.__gastroMaps = null;
    throw e;
  });
  return window.__gastroMaps;
}
const escape = (s) =>
  String(s).replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
function render() {
  if (!map) return;
  map.geoObjects.removeAll();
  for (const card of props.cards) {
    const p = card.place;
    map.geoObjects.add(
      new window.ymaps.Placemark(
        [p.coordinates.lat, p.coordinates.lon],
        {
          balloonContentHeader: escape(p.name),
          balloonContentBody: escape(p.address),
          hintContent: escape(p.name),
        },
        { preset: "islands#circleDotIcon", iconColor: props.color },
      ),
    );
  }
  const bounds = map.geoObjects.getBounds();
  if (props.cards.length > 1 && bounds)
    map.setBounds(bounds, { checkZoomRange: true, zoomMargin: 45 });
  else if (props.cards.length === 1)
    map.setCenter(
      [
        props.cards[0].place.coordinates.lat,
        props.cards[0].place.coordinates.lon,
      ],
      15,
    );
  else if (props.center)
    map.setCenter([props.center.lat, props.center.lon], 13);
}
onMounted(async () => {
  try {
    const ymaps = await loadMaps();
    if (disposed) return;
    map = new ymaps.Map(container.value, {
      center: props.center
        ? [props.center.lat, props.center.lon]
        : [59.9343, 30.3351],
      zoom: 12,
      controls: ["zoomControl"],
    });
    render();
  } catch (e) {
    error.value = e.message;
  }
});
watch(() => [props.cards, props.center, props.color], render, { deep: true });
onBeforeUnmount(() => {
  disposed = true;
  map?.destroy();
});
</script>
<template>
  <div class="map-shell">
    <div
      ref="container"
      class="map"
      aria-label="Заведения выбранного временного слота на карте"
    />
    <div v-if="error" class="map-error" role="status">{{ error }}</div>
    <span class="map-caption"
      >Точки вашего слота · без построения маршрута</span
    >
  </div>
</template>
