<script setup>
import { ref, onBeforeUnmount, nextTick, watch } from "vue";
import { useChatStore } from "./stores/chat";
import PlaceCard from "./components/PlaceCard.vue";
import SearchSources from "./components/SearchSources.vue";
const chat = useChatStore(),
  text = ref(""),
  email = ref(""),
  password = ref(""),
  nickname = ref(""),
  code = ref(""),
  mode = ref("login"),
  authBusy = ref(false),
  authError = ref(""),
  userId = ref(""),
  pendingToken = ref(""),
  feed = ref(null);
const colors = ["#d79921", "#689d6a", "#d65d0e", "#b16286", "#458588"];
async function auth() {
  authBusy.value = true;
  authError.value = "";
  try {
    let result;
    if (mode.value === "register")
      result = await chat.authenticate("register", {
        email: email.value,
        password: password.value,
        username: nickname.value,
      });
    else if (mode.value === "verify")
      result = await chat.authenticate("verify-email", {
        user_id: userId.value,
        code: code.value,
      });
    else if (mode.value === "2fa")
      result = await chat.authenticate("2fa/verify", {
        pending_token: pendingToken.value,
        code: code.value,
      });
    else
      result = await chat.authenticate("login", {
        email: email.value,
        password: password.value,
      });
    if (result.status === "verification_sent") {
      userId.value = result.user_id;
      mode.value = "verify";
    }
    if (result.requires_2fa) {
      pendingToken.value = result.pending_token;
      mode.value = "2fa";
    }
    password.value = "";
  } catch (e) {
    authError.value = e.message;
  } finally {
    authBusy.value = false;
  }
}
async function send(value = text.value) {
  if (!value.trim()) return;
  await chat.send(value);
  text.value = "";
}
async function action(fn) {
  try {
    await fn();
  } catch (e) {
    chat.error = e.message;
  }
}
const formatTime = (value, tz) =>
  new Intl.DateTimeFormat("ru-RU", {
    hour: "2-digit",
    minute: "2-digit",
    timeZone: tz,
  }).format(new Date(value));
const formatDate = (value, tz) =>
  new Intl.DateTimeFormat("ru-RU", {
    day: "numeric",
    month: "long",
    timeZone: tz,
  }).format(new Date(value));
const reasons = {
  outside_radius: "За пределами радиуса",
  closed_or_hours_unknown: "Закрыто или нет часов работы",
  cuisine_mismatch_or_unknown: "Кухня не подходит или неизвестна",
  price_exceeded_or_unknown: "Чек выше бюджета или неизвестен",
};
watch(
  () => [chat.messages.length, chat.busy],
  async () => {
    await nextTick();
    if (feed.value) feed.value.scrollTop = feed.value.scrollHeight;
  },
);
onBeforeUnmount(() => chat.disconnect());
</script>
<template>
  <div class="app-shell">
    <header class="topbar">
      <a href="/" class="brand">соль<span>ГОРОД НА ВКУС</span></a>
      <a
        v-if="chat.isStandalonePresentation"
        :href="chat.applicationUrl"
        class="presentation-login"
        >Вход и регистрация ↗</a
      >
      <p v-else class="hidden sm:block muted text-sm">
        Хорошие места. В ваше время.
      </p>
      <button
        class="theme-switch"
        @click="chat.setTheme(chat.theme === 'paper' ? 'gruvbox' : 'paper')"
        :aria-label="
          chat.theme === 'paper'
            ? 'Включить тёмную тему'
            : 'Включить светлую тему'
        "
      >
        {{ chat.theme === "paper" ? "☾ gruvbox" : "☀ paper" }}
      </button>
    </header>
    <main
      v-if="
        !chat.token && !chat.isPresentation && !chat.isStandalonePresentation
      "
      class="landing"
    >
      <section>
        <span class="eyebrow">ВАШ ДЕНЬ, СО ВКУСОМ</span>
        <h1>У каждого часа<br />своё место<span class="accent">.</span></h1>
        <p class="muted text-lg mt-6 max-w-lg">
          Завтрак у метро, рамен после прогулки, джаз за ужином. Расскажите о
          планах — мы найдём подходящие места для каждого окна времени.
        </p>
        <div class="sample-slots">
          <span>09:00 · завтрак</span><span>14:00 · обед</span
          ><span>20:00 · вечер</span>
        </div>
      </section>
      <form class="auth-panel" @submit.prevent="auth">
        <button
          type="button"
          class="presentation-entry"
          @click="chat.enterPresentation()"
        >
          Открыть готовый пример без входа
        </button>
        <h2>
          {{
            mode === "register"
              ? "Создать аккаунт"
              : mode === "verify"
                ? "Подтвердить email"
                : mode === "2fa"
                  ? "Двухфакторная проверка"
                  : "С возвращением"
          }}
        </h2>
        <p class="muted text-sm mb-6">
          Ваши диалоги и найденные места сохранятся в истории.
        </p>
        <template v-if="mode === 'login' || mode === 'register'"
          ><label v-if="mode === 'register'"
            >Имя<input
              v-model="nickname"
              required
              maxlength="80"
              autocomplete="nickname" /></label
          ><label
            >Email<input
              v-model="email"
              type="email"
              required
              autocomplete="email" /></label
          ><label
            >Пароль<input
              v-model="password"
              type="password"
              required
              :autocomplete="
                mode === 'register' ? 'new-password' : 'current-password'
              " /></label></template
        ><label v-else
          >Код подтверждения<input
            v-model="code"
            required
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength="10"
        /></label>
        <p v-if="authError" class="error" role="alert">{{ authError }}</p>
        <button class="primary w-full mt-4" :disabled="authBusy">
          {{
            authBusy
              ? "Подождите…"
              : mode === "register"
                ? "Зарегистрироваться"
                : mode === "verify" || mode === "2fa"
                  ? "Подтвердить"
                  : "Войти"
          }}</button
        ><button
          class="text-sm mt-5"
          type="button"
          @click="
            mode = mode === 'login' ? 'register' : 'login';
            authError = '';
          "
        >
          {{ mode === "login" ? "Создать аккаунт" : "Вернуться ко входу" }}
        </button>
      </form>
    </main>
    <main v-else class="workspace">
      <aside class="history">
        <div class="flex items-center justify-between gap-2">
          <span class="eyebrow">ВАШИ ПЛАНЫ</span
          ><button
            @click="action(() => chat.newChat())"
            :disabled="chat.busy"
            aria-label="Новый чат"
          >
            ＋
          </button>
        </div>
        <button
          class="new-chat"
          @click="action(() => chat.newChat())"
          :disabled="chat.busy"
        >
          Новый гастрономический день ↗</button
        ><button
          v-for="conversation in chat.conversations"
          :key="conversation.id"
          class="history-item"
          :class="{ active: chat.conversationId === conversation.id }"
          @click="action(() => chat.openChat(conversation.id))"
          :disabled="chat.busy"
        >
          {{ conversation.title
          }}<small>{{
            new Date(conversation.updated_at).toLocaleDateString("ru-RU")
          }}</small></button
        ><button
          v-if="chat.hasMoreConversations"
          @click="action(() => chat.loadMoreConversations())"
        >
          Ещё диалоги</button
        ><a
          v-if="chat.isStandalonePresentation"
          class="mt-auto text-sm"
          :href="chat.applicationUrl"
          >Войти или создать аккаунт ↗</a
        ><button
          v-else
          class="mt-auto text-sm muted"
          @click="action(() => chat.logout())"
        >
          {{
            chat.isPresentation ? "Выйти из презентации" : "Выйти из аккаунта"
          }}
        </button>
      </aside>
      <section class="chat-panel">
        <div class="panel-heading">
          <span class="eyebrow">ЛИЧНЫЙ ГАСТРОНОМ</span
          ><span class="connection" :class="{ online: chat.connected }">{{
            chat.isPresentation
              ? "готовый пример"
              : chat.connected
                ? "на связи"
                : "не подключён"
          }}</span>
        </div>
        <div
          class="chat-feed"
          ref="feed"
          aria-live="polite"
          aria-relevant="additions"
        >
          <div v-if="!chat.messages.length" class="welcome">
            <span class="ornament">✳</span>
            <h2>Чем наполнить<br />ваш день?</h2>
            <p class="muted mt-4">
              Начните с города или расскажите всё сразу. Я уточню детали и
              подберу места по времени.
            </p>
            <button
              class="suggestion mt-5"
              @click="
                send(
                  'Я на Невском, хочу недорогой рамен завтра около двух часов дня, а вечером джаз с бокалом вина',
                )
              "
              :disabled="!chat.connected || chat.busy"
            >
              Рамен днём, джаз вечером ↗
            </button>
          </div>
          <article
            v-for="(message, i) in chat.messages"
            :key="message.request_id + message.type + i"
            class="message"
            :class="message.type"
          >
            <span class="eyebrow">{{
              message.type === "user" ? "ВЫ" : "СОЛЬ"
            }}</span>
            <p>{{ message.text }}</p>
            <button
              v-if="message.slots?.length"
              class="text-sm mt-3"
              @click="
                chat.slots = message.slots;
                chat.selectedSlot = message.slots[0].slot.id;
              "
            >
              Показать эту подборку ↗
            </button>
          </article>
          <div v-if="chat.busy" class="thinking" role="status">
            Собираем ваш день <span>···</span>
          </div>
        </div>
        <div class="composer">
          <div v-if="chat.error" class="error mb-3" role="alert">
            {{ chat.error }}
            <div class="flex gap-4 mt-2">
              <button v-if="!chat.connected" @click="chat.connect()">
                Подключиться</button
              ><button
                v-if="!chat.connected"
                @click="action(() => chat.refreshSession())"
              >
                Обновить сессию</button
              ><button
                v-if="chat.pending && chat.connected && !chat.busy"
                @click="chat.retry()"
              >
                Повторить запрос
              </button>
            </div>
          </div>
          <div class="quick-buttons">
            <button
              v-for="button in chat.activeButtons"
              :key="button.value"
              @click="send(button.value)"
              :disabled="chat.busy || !chat.connected"
            >
              {{ button.label }}
            </button>
          </div>
          <form @submit.prevent="send()" class="input-row">
            <textarea
              v-model="text"
              rows="2"
              maxlength="4000"
              :placeholder="
                chat.isPresentation
                  ? 'Например: другой обед или джаз вечером'
                  : 'Где, когда и чего хочется?'
              "
              aria-label="Сообщение"
              :disabled="chat.busy"
              @keydown.enter.exact.prevent="send()"
            /><button
              class="send"
              aria-label="Отправить"
              :disabled="chat.busy || !chat.connected || !text.trim()"
            >
              ↗
            </button>
          </form>
          <span class="muted text-xs"
            >Enter — отправить · Shift + Enter — новая строка</span
          >
        </div>
      </section>
      <section class="results-panel">
        <div class="results-heading">
          <span class="eyebrow">МЕСТА В ВАШЕМ РИТМЕ</span>
          <h2>План на день</h2>
          <p v-if="chat.isPresentation" class="presentation-note">
            Презентация · заранее подготовленная подборка
          </p>
        </div>
        <div class="slot-tabs" role="tablist" aria-label="Временные слоты">
          <button
            v-for="(result, i) in chat.slots"
            :key="result.slot.id"
            role="tab"
            :aria-selected="chat.selectedSlot === result.slot.id"
            :style="{ '--slot-color': colors[i % colors.length] }"
            :class="{ selected: chat.selectedSlot === result.slot.id }"
            @click="chat.selectedSlot = result.slot.id"
          >
            <span>{{ result.slot.label }}</span
            ><small
              >{{ formatDate(result.slot.start, result.slot.timezone) }} ·
              {{ formatTime(result.slot.start, result.slot.timezone) }}–{{
                formatTime(result.slot.end, result.slot.timezone)
              }}</small
            >
          </button>
        </div>
        <div v-if="!chat.slots.length" class="empty-results">
          <span>01 / Время для открытий</span>
          <p>Здесь появятся места, которые подходят вашим планам.</p>
        </div>
        <SearchSources
          v-if="chat.selectedResult?.grounding"
          :report="chat.selectedResult.grounding"
        />
        <template v-if="chat.selectedResult"
          ><p class="muted text-sm mt-5">{{ chat.selectedResult.text }}</p>
          <div v-if="!chat.cards.length" class="empty-results">
            <p
              v-for="(count, reason) in chat.selectedResult.rejected"
              :key="reason"
            >
              {{ reasons[reason] || reason }}: {{ count }}
            </p>
            <div class="quick-buttons mt-4">
              <button
                v-for="button in chat.selectedResult.dynamic_buttons"
                :key="button.value"
                @click="send(button.value)"
                :disabled="chat.busy || !chat.connected"
              >
                {{ button.label }}
              </button>
            </div>
          </div>
          <PlaceCard
            v-for="card in chat.cards"
            :key="card.place.id"
            :card="card"
            :slot-start="chat.selectedResult.slot.start"
            :slot-end="chat.selectedResult.slot.end"
            :timezone="chat.selectedResult.slot.timezone"
        /></template>
        <p class="muted text-xs mt-5">
          Подборка не подтверждает наличие свободных столиков. Перед визитом
          уточните часы и бронирование у заведения.
        </p>
      </section>
    </main>
  </div>
</template>
