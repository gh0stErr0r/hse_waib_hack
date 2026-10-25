<script setup>
import { computed } from "vue";
const props = defineProps({
  card: { type: Object, required: true },
  slotStart: { type: String, required: true },
  slotEnd: { type: String, required: true },
  timezone: { type: String, required: true },
});
const place = computed(() => props.card.place);
const breakdown = computed(() => props.card.breakdown);
const safeUrl = (url) => {
  try {
    const u = new URL(url);
    return ["http:", "https:"].includes(u.protocol) ? u.href : null;
  } catch {
    return null;
  }
};
const menu = computed(
  () => safeUrl(place.value.menu_url) || safeUrl(place.value.website),
);
const metrics = computed(() => [
  [
    "Соответствие запросу",
    breakdown.value.relevance,
    breakdown.value.max_relevance,
  ],
  [
    "Сайт и меню",
    breakdown.value.website_menu,
    breakdown.value.max_website_menu,
  ],
  ["Комфортная загрузка", breakdown.value.load, breakdown.value.max_load],
  ...(breakdown.value.max_event
    ? [["Программа", breakdown.value.event, breakdown.value.max_event]]
    : []),
]);
</script>
<template>
  <article class="place-card">
    <div class="flex justify-between items-start gap-4">
      <div>
        <span v-if="place.demo" class="eyebrow">Демонстрационные данные</span>
        <h3>{{ place.name }}</h3>
        <p class="muted text-sm">{{ place.address }}</p>
      </div>
      <div class="score">{{ card.score.toFixed(1) }}<small>/ 100</small></div>
    </div>
    <div class="flex justify-between mt-5 text-sm">
      <details v-if="place.presentation_menu?.length" class="presentation-menu">
        <summary>Посмотреть меню примера</summary>
        <dl>
          <div
            v-for="item in place.presentation_menu"
            :key="item.name"
            class="flex justify-between gap-4 py-1"
          >
            <dt>{{ item.name }}</dt>
            <dd>{{ item.price_rub }} ₽</dd>
          </div>
        </dl>
      </details>
      <a
        v-else-if="menu"
        :href="menu"
        target="_blank"
        rel="noopener noreferrer"
        >{{ place.menu_url ? "Открыть меню ↗" : "Сайт заведения ↗" }}</a
      ><span v-else class="muted">Сайт и меню не указаны</span
      ><span>{{
        place.average_price_rub == null
          ? "Чек неизвестен"
          : `~ ${place.average_price_rub} ₽`
      }}</span>
    </div>
    <div class="mt-5">
      <div class="flex justify-between text-sm">
        <span>Загруженность</span
        ><span>{{
          breakdown.load_estimated
            ? "Нет полных данных · нейтральная оценка"
            : `${Math.round(breakdown.load_factor * 100)}%`
        }}</span>
      </div>
      <meter
        class="load-meter"
        min="0"
        max="1"
        :value="breakdown.load_factor"
        :aria-label="
          breakdown.load_estimated
            ? 'Загрузка неизвестна'
            : `Загрузка ${Math.round(breakdown.load_factor * 100)} процентов`
        "
      />
    </div>
    <details class="mt-4">
      <summary>Как получен балл · сценарий {{ breakdown.scenario }}</summary>
      <dl class="mt-3 text-sm">
        <div
          v-for="[label, value, max] in metrics"
          :key="label"
          class="flex justify-between py-1"
        >
          <dt>{{ label }}</dt>
          <dd>{{ value.toFixed(1) }} / {{ max }}</dd>
        </div>
      </dl>
      <p class="muted text-xs mt-2">
        Место прошло проверку радиуса, кухни, бюджета и часов работы.
        Соответствие запросу зависит от порядка результатов поиска, а не от
        рейтинга отзывов.
      </p>
    </details>
    <details v-if="place.evidence?.length" class="mt-4">
      <summary>Откуда данные о заведении</summary>
      <p v-if="place.geocoding_source" class="muted text-xs mt-2">
        {{ place.geocoding_source }}
      </p>
      <ul class="text-xs mt-2">
        <li
          v-for="(evidence, index) in place.evidence"
          :key="index"
          class="mt-2"
        >
          <a
            v-if="safeUrl(evidence.source.url)"
            :href="safeUrl(evidence.source.url)"
            target="_blank"
            rel="noopener noreferrer"
            >{{ evidence.source.title }} ↗</a
          >
          <p class="muted">{{ evidence.quote }}</p>
        </li>
      </ul>
    </details>
    <div
      v-for="event in place.events.filter(
        (e) =>
          new Date(e.start) < new Date(slotEnd) &&
          new Date(e.end) > new Date(slotStart),
      )"
      :key="event.title + event.start"
      class="event mt-4"
    >
      <span>♫ {{ event.title }}</span
      ><span class="block text-sm muted">{{
        new Date(event.start).toLocaleString("ru-RU", { timeZone: timezone })
      }}</span
      ><a
        v-if="safeUrl(event.url)"
        :href="safeUrl(event.url)"
        target="_blank"
        rel="noopener noreferrer"
        >Подробности ↗</a
      >
    </div>
  </article>
</template>
