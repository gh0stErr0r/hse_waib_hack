<script setup>
import { computed, ref, watch } from "vue";
const props = defineProps({
  cards: { type: Array, default: () => [] },
  color: { type: String, default: "#d79921" },
});
const selected = ref(null),
  zoom = ref(1);
const point = (c) => ({
  x: 50 + ((c.lon - 30.314) / 0.044) * 660,
  y: 265 - ((c.lat - 59.927) / 0.019) * 230,
});
const markers = computed(() =>
  props.cards.map((card) => ({ ...point(card.place.coordinates), card })),
);
const viewBox = computed(() => {
  const w = 760 / zoom.value,
    h = 330 / zoom.value;
  return `${380 - w / 2} ${165 - h / 2} ${w} ${h}`;
});
watch(
  () => props.cards,
  () => {
    selected.value = null;
  },
);
</script>
<template>
  <div class="map-shell presentation-map">
    <svg
      :viewBox="viewBox"
      class="map"
      role="group"
      aria-label="Интерактивная демонстрационная схема центра Петербурга"
    >
      <defs>
        <pattern
          id="city-blocks"
          width="76"
          height="57"
          patternUnits="userSpaceOnUse"
        >
          <rect
            x="4"
            y="4"
            width="64"
            height="45"
            rx="5"
            fill="var(--panel)"
            stroke="var(--line)"
            stroke-width="0.6"
          />
        </pattern>
      </defs>
      <rect x="-1000" y="-1000" width="3000" height="3000" fill="var(--soft)" />
      <rect width="760" height="330" fill="url(#city-blocks)" />
      <path
        d="M-20 46 C150 114 214 -25 403 45 S625 92 800 6"
        fill="none"
        stroke="var(--map-water)"
        stroke-width="60"
      />
      <path
        d="M488 -20 C463 101 560 123 530 211 S425 275 460 360"
        fill="none"
        stroke="var(--map-water)"
        stroke-width="19"
      />
      <path
        d="M-30 287 L780 118"
        fill="none"
        stroke="var(--bg)"
        stroke-width="18"
      />
      <path
        d="M-30 287 L780 118"
        fill="none"
        stroke="var(--muted)"
        stroke-opacity="0.3"
        stroke-width="1"
      />
      <text
        x="55"
        y="260"
        fill="var(--muted)"
        font-size="12"
        transform="rotate(-12 55 260)"
      >
        Невский проспект
      </text>
      <text x="160" y="55" fill="var(--muted)" font-size="13">Нева</text>
      <text x="550" y="105" fill="var(--muted)" font-size="11">Фонтанка</text>
      <text x="220" y="135" fill="var(--muted)" font-size="10">
        Дворцовая площадь
      </text>
      <text x="332" y="307" fill="var(--muted)" font-size="10">
        Центр Петербурга
      </text>
      <g
        v-for="({ x, y, card }, index) in markers"
        :key="card.place.id"
        :transform="`translate(${x},${y})`"
        class="presentation-marker"
        role="button"
        tabindex="0"
        :aria-label="card.place.name"
        @click="selected = card"
        @keydown.enter.prevent="selected = card"
        @keydown.space.prevent="selected = card"
      >
        <title>{{ card.place.name }}</title>
        <circle r="21" :fill="color" opacity="0.18" />
        <path
          d="M0 18 C-5 10 -14 2 -14 -5 A14 14 0 1 1 14 -5 C14 2 5 10 0 18"
          :fill="color"
          stroke="var(--bg)"
          stroke-width="2"
        />
        <text
          y="0"
          text-anchor="middle"
          fill="#202020"
          font-size="12"
          font-weight="bold"
        >
          {{ index + 1 }}
        </text>
      </g>
    </svg>
    <div class="map-zoom">
      <button
        aria-label="Приблизить схему"
        :disabled="zoom >= 2"
        @click="zoom = Math.min(2, zoom + 0.25)"
      >
        +</button
      ><button
        aria-label="Отдалить схему"
        :disabled="zoom <= 1"
        @click="zoom = Math.max(1, zoom - 0.25)"
      >
        −
      </button>
    </div>
    <div v-if="selected" class="map-popup">
      <button
        class="float-right ml-3"
        aria-label="Закрыть описание метки"
        @click="selected = null"
      >
        ×</button
      ><strong>{{ selected.place.name }}</strong>
      <p>
        {{ selected.place.average_price_rub }} ₽ ·
        {{ selected.score.toFixed(1) }} / 100
      </p>
    </div>
    <span v-if="!cards.length" class="map-empty"
      >В этом слоте нет подходящих мест</span
    >
    <span class="map-caption">Схема для презентации · вымышленные места</span>
  </div>
</template>
