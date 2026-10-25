import { beforeEach, afterEach, describe, it, expect, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useChatStore } from "./chat";
class Socket {
  static OPEN = 1;
  static latest;
  readyState = 1;
  sent = [];
  constructor() {
    Socket.latest = this;
  }
  send(value) {
    this.sent.push(JSON.parse(value));
  }
  close() {}
  receive(data) {
    this.onmessage({ data: JSON.stringify(data) });
  }
}
let store;
beforeEach(() => {
  setActivePinia(createPinia());
  vi.stubGlobal("localStorage", { getItem: () => null, setItem: vi.fn() });
  vi.stubGlobal("document", { documentElement: { dataset: {} } });
  vi.stubGlobal("location", { protocol: "http:", host: "localhost:8088" });
  vi.stubGlobal("WebSocket", Socket);
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => ({
      ok: true,
      json: async () => ({ conversations: [] }),
    })),
  );
  store = useChatStore();
  store.token = "test-token";
  store.conversationId = "a-conversation";
  store.connect();
  Socket.latest.onopen();
  Socket.latest.receive({ type: "ready" });
});
afterEach(() => {
  store.disconnect();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});
describe("chat delivery", () => {
  it("sends the token in an auth frame and preserves request ID on retry", async () => {
    expect(Socket.latest.sent[0]).toEqual({
      type: "auth",
      token: "test-token",
    });
    await store.send("Рамен");
    const sent = Socket.latest.sent.at(-1);
    Socket.latest.onclose();
    expect(store.pending.request_id).toBe(sent.request_id);
    store.connect();
    Socket.latest.onopen();
    Socket.latest.receive({ type: "ready" });
    store.retry();
    expect(Socket.latest.sent.at(-1).request_id).toBe(sent.request_id);
    expect(store.messages).toHaveLength(1);
  });
  it("deduplicates replayed assistant results", async () => {
    await store.send("Обед");
    const id = store.pending.request_id;
    const reply = {
      type: "assistant",
      request_id: id,
      text: "Готово",
      slots: [{ slot: { id: "lunch" }, cards: [] }],
      dynamic_buttons: [],
    };
    Socket.latest.receive(reply);
    Socket.latest.receive(reply);
    expect(store.messages.filter((m) => m.type === "assistant")).toHaveLength(
      1,
    );
    expect(store.selectedSlot).toBe("lunch");
  });
  it("ignores events from a disconnected socket", () => {
    const old = Socket.latest;
    store.connect();
    old.receive({ type: "error", text: "stale" });
    expect(store.error).not.toBe("stale");
  });
});

describe("prepared presentation", () => {
  it("opens a complete plan without auth, fetch or a new WebSocket", async () => {
    const connection = Socket.latest;
    fetch.mockClear();
    store.enterPresentation();
    expect(store.isPresentation).toBe(true);
    expect(store.slots.map((s) => s.slot.id)).toEqual([
      "breakfast",
      "lunch",
      "dinner",
    ]);
    expect(store.cards.length).toBeGreaterThan(0);
    await store.loadConversations();
    await store.openChat("presentation-ready-plan");
    expect(fetch).not.toHaveBeenCalled();
    expect(Socket.latest).toBe(connection);
  });
  it("switches slots and demonstrates strict budget relaxation locally", async () => {
    store.enterPresentation();
    fetch.mockClear();
    await store.send("Джаз вечером");
    expect(store.selectedSlot).toBe("dinner");
    expect(store.cards[0].breakdown.scenario).toBe("A");
    await store.send("Бюджет до 300 рублей");
    expect(store.selectedSlot).toBe("lunch");
    expect(store.cards).toHaveLength(0);
    const button = store.selectedResult.dynamic_buttons[0];
    await store.send(button.value);
    expect(store.cards.length).toBeGreaterThan(0);
    expect(fetch).not.toHaveBeenCalled();
  });
  it("resets the presentation and exits without calling the real logout endpoint", async () => {
    store.enterPresentation();
    await store.send("Джаз вечером");
    await store.newChat();
    expect(store.selectedSlot).toBe("breakfast");
    fetch.mockClear();
    await store.logout();
    expect(store.isPresentation).toBe(false);
    expect(store.token).toBe("");
    expect(fetch).not.toHaveBeenCalled();
  });
  it("uses internally consistent scores and explicitly fictional fixtures", () => {
    store.enterPresentation();
    for (const result of store.slots)
      for (const card of result.cards) {
        const b = card.breakdown;
        expect(card.place.demo).toBe(true);
        expect(card.score).toBeCloseTo(
          b.relevance + b.website_menu + b.load + b.event,
        );
        expect(b.load).toBeCloseTo(b.max_load * (1 - b.load_factor));
        expect(card.score).toBeGreaterThanOrEqual(0);
        expect(card.score).toBeLessThanOrEqual(100);
        expect(card.place.average_price_rub).toBeLessThanOrEqual(
          result.slot.criteria.max_price_rub,
        );
      }
  });
});

it("standalone presentation cannot expose or submit a registration form after logout", async () => {
  store.disconnect();
  vi.stubEnv("VITE_PRESENTATION_MODE", "true");
  vi.stubGlobal("location", {
    href: "http://127.0.0.1:8090/?demo=1",
    search: "?demo=1",
  });
  setActivePinia(createPinia());
  store = useChatStore();
  fetch.mockClear();
  expect(store.isStandalonePresentation).toBe(true);
  expect(store.applicationUrl).toBe("http://127.0.0.1:8088/");
  await store.logout();
  expect(store.isPresentation).toBe(true);
  await expect(
    store.authenticate("register", { email: "example@example.org" }),
  ).rejects.toThrow("основном приложении");
  expect(fetch).not.toHaveBeenCalled();
});
