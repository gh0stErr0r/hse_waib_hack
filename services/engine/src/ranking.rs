use crate::model::*;
use std::collections::{BTreeMap, HashSet};

fn unit(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.5
    }
}
pub fn valid_coordinates(c: &Coordinates) -> bool {
    c.lat.is_finite() && c.lon.is_finite() && c.lat.abs() <= 90.0 && c.lon.abs() <= 180.0
}
pub fn distance_m(a: &Coordinates, b: &Coordinates) -> f64 {
    let dlat = (b.lat - a.lat).to_radians();
    let dlon = (b.lon - a.lon).to_radians();
    let h = (dlat / 2.0).sin().powi(2)
        + a.lat.to_radians().cos() * b.lat.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    6_371_000.0 * 2.0 * h.clamp(0.0, 1.0).sqrt().asin()
}
pub fn validate_slot(s: &TimeSlot) -> Result<(), String> {
    let tz = s
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| "Неизвестный часовой пояс")?;
    if s.id.trim().is_empty()
        || s.label.trim().is_empty()
        || s.end <= s.start
        || s.end - s.start > chrono::Duration::hours(24)
    {
        return Err("Неверный интервал слота (максимум 24 часа)".into());
    }
    use chrono::Offset;
    if s.start.with_timezone(&tz).offset().fix() != *s.start.offset()
        || s.end.with_timezone(&tz).offset().fix() != *s.end.offset()
    {
        return Err("Смещение времени не соответствует часовому поясу".into());
    }
    if !valid_coordinates(&s.criteria.center)
        || !s.criteria.radius_m.is_finite()
        || !(1.0..=50_000.0).contains(&s.criteria.radius_m)
    {
        return Err("Неверный центр или радиус поиска".into());
    }
    Ok(())
}
pub fn rejection_reason(p: &PlaceCandidate, s: &TimeSlot) -> Option<&'static str> {
    if !valid_coordinates(&p.coordinates)
        || distance_m(&s.criteria.center, &p.coordinates) > s.criteria.radius_m
    {
        return Some("outside_radius");
    }
    // Merge adjacent windows so a continuous opening interval is handled correctly.
    let mut windows = p.opening_windows.clone();
    windows.sort_by_key(|w| w.start);
    let mut covered = s.start;
    for w in windows {
        if w.start <= covered && w.end > covered {
            covered = w.end;
        }
    }
    if covered < s.end {
        return Some("closed_or_hours_unknown");
    }
    if !s.criteria.cuisines.is_empty()
        && !s.criteria.cuisines.iter().any(|c| {
            p.cuisines
                .iter()
                .any(|pc| pc.to_lowercase() == c.to_lowercase())
        })
    {
        return Some("cuisine_mismatch_or_unknown");
    }
    if let Some(max) = s.criteria.max_price_rub
        && p.average_price_rub.is_none_or(|price| price > max)
    {
        return Some("price_exceeded_or_unknown");
    }
    None
}
pub fn apply_hard_filter<'a>(
    places: &'a [PlaceCandidate],
    slot: &TimeSlot,
) -> Vec<&'a PlaceCandidate> {
    places
        .iter()
        .filter(|p| rejection_reason(p, slot).is_none())
        .collect()
}
fn safe_link(url: &Option<String>) -> bool {
    url.as_ref().is_some_and(|u| {
        reqwest::Url::parse(u)
            .is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
    })
}
pub fn calculate_score(place: &PlaceCandidate, slot: &TimeSlot) -> ScoredPlace {
    let (r, m, l, e) = if slot.requires_event {
        (20.0, 20.0, 20.0, 40.0)
    } else {
        (40.0, 30.0, 30.0, 0.0)
    };
    // Integrate load across the whole interval. Uncovered portions use neutral 0.5.
    let mut cuts = vec![slot.start, slot.end];
    for x in &place.load {
        if x.value.is_finite() && (0.0..=1.0).contains(&x.value) && x.end > x.start {
            cuts.push(x.start.max(slot.start).min(slot.end));
            cuts.push(x.end.max(slot.start).min(slot.end));
        }
    }
    cuts.sort();
    cuts.dedup();
    let mut weighted = 0.0;
    let mut estimated = false;
    for pair in cuts.windows(2) {
        let samples: Vec<_> = place
            .load
            .iter()
            .filter(|x| {
                x.start <= pair[0]
                    && x.end >= pair[1]
                    && x.value.is_finite()
                    && (0.0..=1.0).contains(&x.value)
            })
            .collect();
        let load = if samples.is_empty() {
            estimated = true;
            0.5
        } else {
            samples.iter().map(|x| x.value).sum::<f64>() / samples.len() as f64
        };
        weighted += load
            * (pair[1] - pair[0])
                .to_std()
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
    }
    let load_factor = if slot.end > slot.start {
        weighted
            / (slot.end - slot.start)
                .to_std()
                .map(|d| d.as_secs_f64())
                .unwrap_or(1.0)
    } else {
        estimated = true;
        0.5
    };
    let event = if slot.requires_event {
        place
            .events
            .iter()
            .filter(|x| x.end > x.start && x.start < slot.end && x.end > slot.start)
            .map(|x| {
                if slot.interests.is_empty()
                    || slot.interests.iter().any(|i| {
                        x.interests
                            .iter()
                            .any(|t| t.to_lowercase() == i.to_lowercase())
                    })
                {
                    40.0_f64
                } else {
                    20.0_f64
                }
            })
            .fold(0.0_f64, f64::max)
    } else {
        0.0
    };
    let breakdown = ScoringBreakdown {
        scenario: if slot.requires_event { "A" } else { "B" }.into(),
        relevance: r * unit(place.relevance),
        website_menu: if (!slot.requires_event || place.menu_with_prices)
            && safe_link(&place.menu_url)
        {
            m
        } else if safe_link(&place.website) || safe_link(&place.menu_url) {
            m / 2.0
        } else {
            0.0
        },
        load: l * (1.0 - unit(load_factor)),
        event,
        max_relevance: r,
        max_website_menu: m,
        max_load: l,
        max_event: e,
        load_factor,
        load_estimated: estimated,
    };
    ScoredPlace {
        place: place.clone(),
        slot_id: slot.id.clone(),
        score: (breakdown.relevance + breakdown.website_menu + breakdown.load + breakdown.event)
            .clamp(0.0, 100.0),
        breakdown,
    }
}
pub fn rank(request: &RankRequest) -> Result<Vec<SlotResult>, String> {
    if request.slots.is_empty() || request.slots.len() > 12 || request.candidates.len() > 500 {
        return Err("Нужно 1–12 слотов и не более 500 кандидатов".into());
    }
    let mut ids = HashSet::new();
    for slot in &request.slots {
        validate_slot(slot)?;
        if !ids.insert(&slot.id) {
            return Err("Идентификаторы слотов должны быть уникальны".into());
        }
    }
    Ok(request.slots.iter().map(|slot| {
        let mut rejected = BTreeMap::new();
        let mut seen = HashSet::new();
        for place in &request.candidates {
            if let Some(reason) = rejection_reason(place, slot) {
                *rejected.entry(reason.into()).or_insert(0) += 1;
            }
        }
        let mut cards: Vec<_> = apply_hard_filter(&request.candidates, slot).into_iter()
            .filter(|place| seen.insert(&place.id)).map(|place| calculate_score(place, slot)).collect();
        cards.sort_by(|a,b| b.score.total_cmp(&a.score).then_with(|| a.place.id.cmp(&b.place.id)));
        cards.truncate(10);
        let empty = cards.is_empty();
        let mut dynamic_buttons = vec![];
        if empty {
            if slot.criteria.radius_m < 50_000.0 {
                let radius=(slot.criteria.radius_m*2.0).clamp(3000.0,50_000.0) as u32;
                dynamic_buttons.push(Button{label:format!("Расширить радиус до {} км",radius/1000),value:format!("Для слота {} установи радиус {} метров",slot.id,radius)});
            }
            if !slot.criteria.cuisines.is_empty(){dynamic_buttons.push(Button{label:"Любая кухня".into(),value:format!("Для слота {} убери ограничение кухни",slot.id)});}
            if slot.criteria.max_price_rub.is_some(){dynamic_buttons.push(Button{label:"Без ограничения бюджета".into(),value:format!("Для слота {} убери ограничение бюджета: max_price_rub=0",slot.id)});}
        }
        SlotResult {
            slot:slot.clone(),cards,rejected,dynamic_buttons,grounding:None,
            text:if empty {"Нет мест с подтверждённым соответствием времени, кухне, радиусу и бюджету. Можно ослабить ограничения. Неизвестные часы и цены не считаются подтверждением."}else{"Подборка по вашему временному слоту"}.into(),
        }
    }).collect())
}
