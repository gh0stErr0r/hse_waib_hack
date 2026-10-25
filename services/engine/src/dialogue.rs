use crate::{error::ApiError, model::*, providers, ranking};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub location: Option<String>,
    pub cuisines: Option<Vec<String>>,
    pub max_price_rub: Option<u32>,
    pub requires_event: Option<bool>,
    pub interests: Option<Vec<String>>,
    #[serde(default)]
    pub slots: Vec<Value>,
}
impl Profile {
    pub fn merge(&mut self, v: Value) -> Result<(), ApiError> {
        let mut merged = serde_json::to_value(&*self).map_err(|_| ApiError::internal())?;
        let object = v
            .as_object()
            .ok_or_else(|| ApiError::new(422, "Не удалось разобрать предпочтения"))?;
        for (k, value) in object {
            if [
                "location",
                "cuisines",
                "max_price_rub",
                "requires_event",
                "interests",
                "slots",
            ]
            .contains(&k.as_str())
                && !value.is_null()
            {
                merged[k] = value.clone();
            }
        }
        *self = serde_json::from_value(merged)
            .map_err(|_| ApiError::new(422, "Некорректные параметры предпочтений"))?;
        if self.slots.len() > 12 || self.location.as_ref().is_some_and(|x| x.len() > 500) {
            return Err(ApiError::new(
                422,
                "Слишком много слотов или слишком длинная локация",
            ));
        }
        Ok(())
    }
    pub fn question(&self) -> Option<(&'static str, Vec<&'static str>)> {
        if self.location.as_ref().is_none_or(|s| s.trim().is_empty()) {
            return Some((
                "В каком городе, районе или у какого метро будем искать?",
                vec!["Москва, центр", "Санкт-Петербург, Невский проспект"],
            ));
        }
        if self.cuisines.is_none()
            && (self.slots.is_empty()
                || !self
                    .slots
                    .iter()
                    .all(|s| s.get("cuisines").is_some_and(Value::is_array)))
        {
            return Some((
                "Какую кухню или формат предпочитаете?",
                vec![
                    "Итальянская",
                    "Паназиатская",
                    "Спешелти-кофе",
                    "Любая кухня",
                ],
            ));
        }
        if self.max_price_rub.is_none()
            && (self.slots.is_empty()
                || !self
                    .slots
                    .iter()
                    .all(|s| s.get("max_price_rub").is_some_and(Value::is_u64)))
        {
            return Some((
                "Какой максимальный чек на человека?",
                vec!["До 1000 ₽", "До 2500 ₽", "Без ограничения бюджета"],
            ));
        }
        if self.slots.is_empty() {
            return Some((
                "На какую дату и время планируем? Укажите часовой пояс или город. Можно несколько интервалов.",
                vec![
                    "Завтра, завтрак 09:00–11:00 и обед 13:30–15:00 по местному времени",
                    "Завтра, ужин 19:30–22:00 по местному времени",
                ],
            ));
        }
        if self.requires_event.is_none()
            && !self
                .slots
                .iter()
                .all(|s| s.get("requires_event").is_some_and(Value::is_boolean))
        {
            return Some((
                "Нужна ли программа в заведении?",
                vec![
                    "Без программы",
                    "Джаз вечером",
                    "Стендап вечером",
                    "Дегустация вечером",
                ],
            ));
        }
        None
    }
}
pub async fn answer(client: &reqwest::Client, profile: &Profile) -> Result<Value, ApiError> {
    if let Some((text, options)) = profile.question() {
        return Ok(
            json!({"type":"assistant","text":text,"dynamic_buttons":options.into_iter().map(|v|Button{label:v.into(),value:v.into()}).collect::<Vec<_>>(),"slots":[]}),
        );
    }
    let center = if std::env::var("DEMO_PLACES").as_deref() == Ok("true") {
        Coordinates {
            lat: 59.9343,
            lon: 30.3351,
        }
    } else {
        providers::geocode(client, profile.location.as_deref().unwrap_or_default()).await?
    };
    let mut slots = vec![];
    for raw in &profile.slots {
        let mut v = raw.clone();
        let criteria = FilterCriteria {
            center: center.clone(),
            radius_m: v.get("radius_m").and_then(Value::as_f64).unwrap_or(1000.0),
            cuisines: match v.get("cuisines") {
                Some(c) => serde_json::from_value(c.clone())
                    .map_err(|_| ApiError::new(422, "Неверная кухня слота"))?,
                None => profile.cuisines.clone().unwrap_or_default(),
            },
            max_price_rub: match v.get("max_price_rub").and_then(Value::as_u64) {
                Some(0) => None,
                Some(n) => {
                    Some(u32::try_from(n).map_err(|_| ApiError::new(422, "Некорректный бюджет"))?)
                }
                None => profile.max_price_rub.filter(|x| *x > 0),
            },
        };
        v["criteria"] = serde_json::to_value(criteria).map_err(|_| ApiError::internal())?;
        if v.get("requires_event").is_none() {
            v["requires_event"] = json!(profile.requires_event.unwrap_or(false));
        }
        if v.get("interests").is_none() {
            v["interests"] = json!(profile.interests.clone().unwrap_or_default());
        }
        let slot: TimeSlot = serde_json::from_value(v).map_err(|_| {
            ApiError::new(
                422,
                "Укажите конкретную дату, время и часовой пояс для каждого слота",
            )
        })?;
        ranking::validate_slot(&slot).map_err(|e| ApiError::new(422, e))?;
        slots.push(slot);
    }
    let mut ids = std::collections::HashSet::new();
    if slots.iter().any(|s| !ids.insert(&s.id)) {
        return Err(ApiError::new(
            422,
            "Слоты должны иметь уникальные идентификаторы",
        ));
    }
    let use_web = std::env::var("PLACES_PROVIDER").unwrap_or_else(|_| "gemini".into()) == "gemini"
        && std::env::var("DEMO_PLACES").as_deref() != Ok("true");
    let mut results = vec![];
    if use_web {
        let mut search = crate::web_search::search_slots(
            client,
            &slots,
            profile.location.as_deref().unwrap_or_default(),
        )
        .await?;
        for slot in slots {
            let candidates = search.by_slot.remove(&slot.id).unwrap_or_default();
            let mut ranked = ranking::rank(&RankRequest {
                slots: vec![slot],
                candidates,
            })
            .map_err(|e| ApiError::new(422, e))?;
            for result in &mut ranked {
                result.grounding = Some(search.grounding.clone());
            }
            results.extend(ranked);
        }
    } else {
        for slot in slots {
            let candidates = providers::search_legacy(
                client,
                &slot,
                profile.location.as_deref().unwrap_or_default(),
            )
            .await?;
            results.extend(
                ranking::rank(&RankRequest {
                    slots: vec![slot],
                    candidates,
                })
                .map_err(|e| ApiError::new(422, e))?,
            );
        }
    }
    Ok(
        json!({"type":"assistant","text":"Ваш гастрономический план. Выберите слот, чтобы увидеть места на карте.","dynamic_buttons":[],"slots":results}),
    )
}
