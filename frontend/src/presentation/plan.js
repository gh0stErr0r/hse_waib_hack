// Prepared fictional scenario. No live availability or provider data is implied.
export const presentationButtons = [
  { label: "Показать завтрак", value: "Показать завтрак" },
  { label: "Другой обед", value: "Другой обед" },
  { label: "Джаз вечером", value: "Джаз вечером" },
  { label: "Бюджет до 300 рублей", value: "Бюджет до 300 рублей" },
  { label: "Весь план", value: "Весь план" },
];
const date = "2026-11-01";
const at = (hour) => `${date}T${hour}:00+03:00`;
const definitions = [
  {
    id: "breakfast",
    label: "Завтрак",
    start: "09:00",
    end: "11:00",
    cuisine: "европейская",
    budget: 1200,
    places: [
      [
        "Бумага и кофе",
        "Район Казанского собора",
        59.9342,
        30.3245,
        850,
        0.95,
        0.2,
      ],
      [
        "Тёплый хлеб",
        "Район Гостиного двора",
        59.9345,
        30.332,
        700,
        0.88,
        0.35,
      ],
    ],
  },
  {
    id: "lunch",
    label: "Обед",
    start: "13:30",
    end: "15:00",
    cuisine: "японская",
    budget: 1500,
    places: [
      [
        "Рамен на углу",
        "Район Итальянской улицы",
        59.9372,
        30.3326,
        1100,
        0.97,
        0.3,
      ],
      [
        "Миска и пар",
        "Район Владимирского проспекта",
        59.9329,
        30.3452,
        950,
        0.91,
        0.45,
      ],
      [
        "Лапша и бульон",
        "Район Садовой улицы",
        59.9303,
        30.322,
        800,
        0.86,
        0.25,
      ],
    ],
  },
  {
    id: "dinner",
    label: "Ужин и джаз",
    start: "19:30",
    end: "22:00",
    cuisine: "европейская",
    budget: 3000,
    event: true,
    places: [
      [
        "Соль и винил",
        "Район набережной Фонтанки",
        59.9368,
        30.3435,
        2400,
        0.96,
        0.4,
      ],
      [
        "Тихий квартет",
        "Район Михайловской улицы",
        59.9375,
        30.33,
        2700,
        0.89,
        0.55,
      ],
    ],
  },
];
export function preparedPlan() {
  return definitions.map((definition) => {
    const slot = {
      id: definition.id,
      label: definition.label,
      start: at(definition.start),
      end: at(definition.end),
      timezone: "Europe/Moscow",
      requires_event: !!definition.event,
      interests: definition.event ? ["джаз"] : [],
      criteria: {
        center: { lat: 59.9348, lon: 30.333 },
        radius_m: 3000,
        cuisines: [definition.cuisine],
        max_price_rub: definition.budget,
      },
    };
    const cards = definition.places
      .map(([name, address, lat, lon, price, relevance, load], index) => {
        const weights = definition.event ? [20, 20, 20, 40] : [40, 30, 30, 0];
        const breakdown = {
          scenario: definition.event ? "A" : "B",
          relevance: weights[0] * relevance,
          website_menu: weights[1],
          load: weights[2] * (1 - load),
          event: weights[3],
          max_relevance: weights[0],
          max_website_menu: weights[1],
          max_load: weights[2],
          max_event: weights[3],
          load_factor: load,
          load_estimated: false,
        };
        return {
          slot_id: slot.id,
          score:
            breakdown.relevance +
            breakdown.website_menu +
            breakdown.load +
            breakdown.event,
          breakdown,
          place: {
            id: `presentation-${slot.id}-${index}`,
            name,
            address: `${address} · демонстрационная локация`,
            coordinates: { lat, lon },
            cuisines: [definition.cuisine],
            average_price_rub: price,
            opening_windows: [{ start: at("08:00"), end: at("23:59") }],
            website: "https://example.com",
            menu_url: "https://example.com/menu",
            menu_with_prices: true,
            relevance,
            load: [{ start: slot.start, end: slot.end, value: load }],
            events: definition.event
              ? [
                  {
                    title: "Джазовый квартет · демонстрационная программа",
                    start: at("20:00"),
                    end: at("21:30"),
                    interests: ["джаз"],
                    url: null,
                  },
                ]
              : [],
            demo: true,
            presentation_menu:
              definition.id === "breakfast"
                ? [
                    { name: "Капучино", price_rub: 250 },
                    { name: "Тост с яйцом", price_rub: 450 },
                  ]
                : definition.id === "lunch"
                  ? [
                      { name: "Рамен с курицей", price_rub: 690 },
                      { name: "Чай", price_rub: 190 },
                    ]
                  : [
                      { name: "Запечённая рыба", price_rub: 1100 },
                      { name: "Бокал вина", price_rub: 550 },
                    ],
          },
        };
      })
      .sort((a, b) => b.score - a.score);
    return {
      slot,
      cards,
      rejected: {},
      dynamic_buttons: [],
      text: "Готовая подборка для презентации. Заведения, цены, загрузка и события вымышлены.",
    };
  });
}
export function presentationReply(text) {
  const slots = preparedPlan();
  let selected = "breakfast",
    answer =
      "Открыт готовый план: завтрак, обед и ужин с джазом. Выберите слот или нажмите одну из кнопок, чтобы показать возможности сервиса.";
  if (/300|строг|эконом/i.test(text)) {
    const lunch = slots.find((s) => s.slot.id === "lunch");
    lunch.slot.criteria.max_price_rub = 300;
    lunch.rejected = { price_exceeded_or_unknown: lunch.cards.length };
    lunch.cards = [];
    lunch.text =
      "В готовом примере нет обедов до 300 рублей. Поднимем бюджет до 1500 рублей?";
    lunch.dynamic_buttons = [
      {
        label: "Вернуть бюджет 1500 рублей",
        value: "Вернуть бюджет 1500 рублей",
      },
    ];
    selected = "lunch";
    answer =
      "Все варианты обеда дороже 300 рублей и отсеяны до скоринга. Можно изменить ограничение, чтобы вернуть подборку.";
  } else if (/другой|альтернатив/i.test(text)) {
    const lunch = slots.find((s) => s.slot.id === "lunch");
    lunch.cards = lunch.cards.slice(1);
    selected = "lunch";
    answer =
      "Показаны оставшиеся варианты обеда из готовой подборки. Временные слоты завтрака и ужина сохранены.";
  } else if (/джаз|вечер|ужин/i.test(text)) {
    selected = "dinner";
    answer =
      "Открыт вечерний слот с джазом. Здесь используется сценарий А: до 40 баллов за совпадение программы по времени и интересам.";
  } else if (/обед|рамен|1500/i.test(text)) {
    selected = "lunch";
    answer =
      "Открыт слот обеда. У каждого места видны чек, загрузка и подробная оценка по сценарию Б.";
  } else if (/завтрак|кофе/i.test(text)) {
    answer =
      "Открыт завтрак: два варианта с кофе и европейской кухней. Метки на схеме соответствуют только этому слоту.";
  } else if (!/весь|план|начать/i.test(text)) {
    answer =
      "Это автономная презентация с заранее подготовленным планом Петербурга. Новый поиск здесь не выполняется. Кнопками можно показать завтрак, заменить обед, открыть джазовый вечер или продемонстрировать пустую выдачу.";
  }
  return {
    type: "assistant",
    text: answer,
    dynamic_buttons: structuredClone(presentationButtons),
    slots,
    selected,
  };
}
