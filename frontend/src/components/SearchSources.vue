<script setup>
defineProps({ report: { type: Object, required: true } });
const safe = (url) => {
  try {
    return ["http:", "https:"].includes(new URL(url).protocol);
  } catch {
    return false;
  }
};
</script>
<template>
  <details class="search-sources">
    <summary>Источники поиска · {{ report.provider }}</summary>
    <p class="muted text-xs mt-2">
      Проверено {{ new Date(report.checked_at).toLocaleString("ru-RU") }}.
      Информация может измениться.
    </p>
    <p
      v-for="warning in report.warnings"
      :key="warning"
      class="muted text-xs mt-2"
    >
      {{ warning }}
    </p>
    <ul class="mt-3 text-sm">
      <li
        v-for="source in report.sources.filter((s) => safe(s.url))"
        :key="source.url"
      >
        <a :href="source.url" target="_blank" rel="noopener noreferrer"
          >{{ source.title }} ↗</a
        >
      </li>
    </ul>
  </details>
  <iframe
    v-for="(html, index) in report.search_suggestions_html"
    :key="report.checked_at + index"
    :srcdoc="html"
    sandbox="allow-popups allow-popups-to-escape-sandbox"
    referrerpolicy="no-referrer"
    title="Google Search Suggestions"
    class="search-suggestions"
  />
</template>
