import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  preparedPlan,
  presentationReply,
  presentationButtons,
} from "../presentation/plan";

export const useChatStore = defineStore("chat", () => {
  const messages = ref([]),
    activeButtons = ref([]),
    slots = ref([]),
    selectedSlot = ref(null);
  const theme = ref(
    localStorage.getItem("gastro-theme") === "paper" ? "paper" : "gruvbox",
  );
  const isPresentation = ref(false);
  const isStandalonePresentation =
    import.meta.env.MODE === "presentation" ||
    import.meta.env.VITE_PRESENTATION_MODE === "true";
  const applicationUrl = (() => {
    try {
      const configured = import.meta.env.VITE_APPLICATION_URL;
      const url = new URL(
        configured ||
          (typeof location !== "undefined"
            ? location.href ||
              `${location.protocol || "http:"}//${location.host || "localhost:8090"}`
            : "http://localhost:8090"),
      );
      if (!["http:", "https:"].includes(url.protocol))
        return "http://localhost:8088/";
      if (!configured) {
        url.port = "8088";
        url.pathname = "/";
        url.search = "";
        url.hash = "";
      }
      return url.href;
    } catch {
      return "http://localhost:8088/";
    }
  })();
  const token = ref(""),
    conversations = ref([]),
    conversationId = ref(null);
  const connected = ref(false),
    busy = ref(false),
    error = ref(""),
    pending = ref(null),
    hasMoreConversations = ref(false);
  let socket,
    heartbeat,
    refreshToken = "",
    generation = 0;
  const selectedResult = computed(() =>
    slots.value.find((s) => s.slot.id === selectedSlot.value),
  );
  const cards = computed(() => selectedResult.value?.cards || []);
  function setTheme(value) {
    theme.value = value === "paper" ? "paper" : "gruvbox";
    localStorage.setItem("gastro-theme", theme.value);
    document.documentElement.dataset.theme = theme.value;
  }
  setTheme(theme.value);
  async function api(path, { method = "GET", body, auth = true } = {}) {
    const response = await fetch(path, {
      method,
      headers: {
        "Content-Type": "application/json",
        ...(auth ? { Authorization: `Bearer ${token.value}` } : {}),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(260000),
    });
    const data = await response.json().catch(() => ({}));
    if (!response.ok) {
      if (response.status === 401 && auth) disconnect();
      throw new Error(
        data.text ||
          data.message ||
          data.error?.message ||
          `Ошибка ${response.status}`,
      );
    }
    return data;
  }
  async function authenticate(path, body) {
    if (isStandalonePresentation)
      throw new Error(
        "Вход и регистрация доступны в основном приложении. Используйте ссылку «Вход и регистрация».",
      );
    const result = await api(`/api/auth/auth/${path}`, {
      method: "POST",
      body,
      auth: false,
    });
    if (result.access_token) {
      token.value = result.access_token;
      refreshToken = result.refresh_token;
      await loadConversations();
      connect();
    }
    return result;
  }
  async function refreshSession() {
    if (!refreshToken) throw new Error("Войдите в аккаунт");
    await authenticate("refresh", { refresh_token: refreshToken });
  }
  function disconnect() {
    generation++;
    clearInterval(heartbeat);
    socket?.close();
    socket = undefined;
    connected.value = false;
    busy.value = false;
  }
  function connect() {
    if (isPresentation.value) {
      connected.value = true;
      return;
    }
    disconnect();
    if (!token.value) return;
    const current = generation;
    const next = new WebSocket(
      `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/ws`,
    );
    socket = next;
    next.onopen = () => {
      if (current === generation)
        next.send(JSON.stringify({ type: "auth", token: token.value }));
    };
    next.onmessage = ({ data }) => {
      if (current !== generation) return;
      let reply;
      try {
        reply = JSON.parse(data);
      } catch {
        error.value = "Некорректный ответ сервера";
        return;
      }
      if (reply.type === "ready") {
        connected.value = true;
        error.value = "";
        heartbeat = setInterval(() => {
          if (next.readyState === WebSocket.OPEN)
            next.send(JSON.stringify({ type: "ping" }));
        }, 30000);
        return;
      }
      if (reply.type === "pong") return;
      if (reply.type === "thinking") {
        busy.value = true;
        return;
      }
      busy.value = false;
      if (reply.type === "error") {
        error.value = reply.text;
        if (reply.status === 401) disconnect();
        return;
      }
      if (reply.type === "assistant") {
        if (
          !messages.value.some(
            (m) => m.type === "assistant" && m.request_id === reply.request_id,
          )
        )
          messages.value.push(reply);
        applyReply(reply);
        pending.value = null;
        error.value = "";
        loadConversations().catch((e) => {
          error.value = e.message;
        });
      }
    };
    next.onclose = () => {
      if (current !== generation) return;
      clearInterval(heartbeat);
      connected.value = false;
      busy.value = false;
      error.value =
        "Соединение закрыто. Подключитесь снова; отправку можно безопасно повторить.";
    };
    next.onerror = () => {
      if (current === generation)
        error.value = "Не удалось подключиться к чату";
    };
  }
  function applyReply(reply) {
    activeButtons.value = reply.dynamic_buttons || [];
    slots.value = reply.slots || [];
    if (!slots.value.some((s) => s.slot.id === selectedSlot.value))
      selectedSlot.value = slots.value[0]?.slot.id || null;
  }
  async function loadConversations() {
    if (isPresentation.value) return;
    const page = (await api("/api/v1/conversations")).conversations;
    conversations.value = page;
    hasMoreConversations.value = page.length === 30;
  }
  async function loadMoreConversations() {
    if (isPresentation.value) return;
    const page = (
      await api(`/api/v1/conversations?offset=${conversations.value.length}`)
    ).conversations;
    conversations.value.push(...page);
    hasMoreConversations.value = page.length === 30;
  }
  async function newChat() {
    if (busy.value) return;
    if (isPresentation.value) {
      enterPresentation();
      return;
    }
    const result = await api("/api/v1/conversations", {
      method: "POST",
      body: {},
    });
    conversationId.value = result.id;
    messages.value = [];
    slots.value = [];
    activeButtons.value = [];
    selectedSlot.value = null;
    pending.value = null;
    await loadConversations();
  }
  async function openChat(id) {
    if (busy.value) return;
    if (isPresentation.value) {
      enterPresentation();
      return;
    }
    const result = await api(`/api/v1/conversations/${id}`);
    conversationId.value = id;
    messages.value = result.document.messages || [];
    pending.value = null;
    applyReply(
      [...messages.value].reverse().find((m) => m.type === "assistant") || {},
    );
  }
  async function send(text) {
    text = text.trim();
    if (!text || busy.value || !connected.value) return;
    if (text.length > 4000) {
      error.value = "Максимум 4000 символов";
      return;
    }
    if (isPresentation.value) {
      const id = crypto.randomUUID();
      messages.value.push({ type: "user", text, request_id: id });
      const reply = presentationReply(text);
      reply.request_id = id;
      messages.value.push(reply);
      applyReply(reply);
      selectedSlot.value = reply.selected;
      return;
    }
    busy.value = true;
    error.value = "";
    try {
      // Temporarily release the new-chat guard while creating the initial conversation.
      if (!conversationId.value) {
        const result = await api("/api/v1/conversations", {
          method: "POST",
          body: {},
        });
        conversationId.value = result.id;
      }
      const request = {
        type: "message",
        conversation_id: conversationId.value,
        request_id: crypto.randomUUID(),
        text,
      };
      pending.value = request;
      messages.value.push({
        type: "user",
        text,
        request_id: request.request_id,
      });
      socket.send(JSON.stringify(request));
    } catch (e) {
      busy.value = false;
      error.value = e.message;
    }
  }
  function retry() {
    if (pending.value && connected.value && !busy.value) {
      busy.value = true;
      error.value = "";
      socket.send(JSON.stringify(pending.value));
    }
  }
  async function logout() {
    if (isStandalonePresentation) {
      enterPresentation();
      return;
    }
    try {
      if (!isPresentation.value)
        await api("/api/auth/auth/logout", { method: "POST", body: {} });
    } finally {
      disconnect();
      isPresentation.value = false;
      token.value = "";
      refreshToken = "";
      messages.value = [];
      conversations.value = [];
      slots.value = [];
      activeButtons.value = [];
      conversationId.value = null;
      pending.value = null;
      error.value = "";
    }
  }
  function enterPresentation() {
    disconnect();
    isPresentation.value = true;
    connected.value = true;
    error.value = "";
    pending.value = null;
    hasMoreConversations.value = false;
    conversationId.value = "presentation-ready-plan";
    conversations.value = [
      {
        id: conversationId.value,
        title: "Петербург: кофе, рамен и джаз",
        updated_at: "2026-11-01T09:00:00+03:00",
      },
    ];
    const reply = {
      type: "assistant",
      request_id: "prepared-plan",
      text: "Ваш день в центре Петербурга уже подобран: кофе за завтраком, рамен днём и джаз вечером. Это готовый пример для презентации. Все места и показатели демонстрационные; поиск и генерация не запускаются.",
      dynamic_buttons: structuredClone(presentationButtons),
      slots: preparedPlan(),
    };
    messages.value = [
      {
        type: "user",
        request_id: "prepared-request",
        text: "Хочу завтрак с кофе, рамен около двух, а вечером — джаз. Центр Петербурга, без длинных переездов.",
      },
      reply,
    ];
    applyReply(reply);
    selectedSlot.value = "breakfast";
  }
  if (
    isStandalonePresentation ||
    (typeof location !== "undefined" &&
      new URLSearchParams(location.search || "").get("demo") === "1")
  )
    enterPresentation();
  return {
    isStandalonePresentation,
    applicationUrl,
    isPresentation,
    enterPresentation,
    messages,
    activeButtons,
    slots,
    selectedSlot,
    theme,
    token,
    conversations,
    conversationId,
    connected,
    busy,
    error,
    pending,
    selectedResult,
    cards,
    setTheme,
    authenticate,
    refreshSession,
    connect,
    disconnect,
    loadConversations,
    loadMoreConversations,
    hasMoreConversations,
    newChat,
    openChat,
    send,
    retry,
    logout,
  };
});
