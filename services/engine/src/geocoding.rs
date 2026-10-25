//! Keyless Photon geocoder. Shared cache and a conservative one-request/second gate.
use crate::{error::ApiError, model::Coordinates, ranking::valid_coordinates};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
struct Cache {
    entries: HashMap<String, (Instant, Coordinates)>,
    last: Option<Instant>,
}
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
pub async fn geocode(
    client: &reqwest::Client,
    query: &str,
    bias: Option<&Coordinates>,
) -> Result<Coordinates, ApiError> {
    if query.trim().is_empty() || query.chars().count() > 600 {
        return Err(ApiError::new(422, "Укажите город и адрес для поиска"));
    }
    let url =
        std::env::var("GEOCODER_URL").unwrap_or_else(|_| "https://photon.komoot.io/api/".into());
    let cache_key = format!(
        "{url}|{}|{:?}",
        query.to_lowercase(),
        bias.as_ref().map(|b| (b.lat, b.lon))
    );
    let mut cache = CACHE
        .get_or_init(|| {
            Mutex::new(Cache {
                entries: HashMap::new(),
                last: None,
            })
        })
        .lock()
        .await;
    if let Some((time, coordinates)) = cache.entries.get(&cache_key)
        && time.elapsed() < Duration::from_secs(86400)
    {
        return Ok(coordinates.clone());
    }
    if let Some(last) = cache.last {
        let delay = Duration::from_millis(1100).saturating_sub(last.elapsed());
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
    cache.last = Some(Instant::now());
    let mut request = client
        .get(url)
        .header(
            "User-Agent",
            "GastroSlots/0.1 (venue planner; Photon geocoding)",
        )
        .query(&[("q", query), ("limit", "1")])
        .timeout(Duration::from_secs(8));
    if let Some(center) = bias {
        request = request.query(&[("lat", center.lat), ("lon", center.lon)]);
    }
    let response = request
        .send()
        .await
        .map_err(|e| crate::upstream::transport("Photon", "geocode", e))?;
    if response.status().as_u16() == 429 {
        tracing::warn!(
            provider = "Photon",
            operation = "geocode",
            http_status = 429,
            "geocoder rate limited"
        );
        return Err(ApiError::new(
            503,
            "Геокодер ограничил частоту запросов. Повторите позже.",
        ));
    }
    let status = response.status().as_u16();
    let value: Value = response
        .error_for_status()
        .map_err(|e| crate::upstream::transport("Photon", "geocode", e))?
        .json()
        .await
        .map_err(|e| crate::upstream::json("Photon", "geocode", status, e))?;
    if bias.is_some()
        && value
            .pointer("/features/0/properties/type")
            .and_then(Value::as_str)
            .is_some_and(|kind| {
                matches!(kind, "city" | "state" | "county" | "country" | "district")
            })
    {
        return Err(ApiError::new(
            422,
            "Геокодер нашёл только район или город, а не адрес заведения.",
        ));
    }
    let coordinates = parse_coordinates(&value)?;
    if cache.entries.len() >= 2048 {
        cache
            .entries
            .retain(|_, (time, _)| time.elapsed() < Duration::from_secs(3600));
        if cache.entries.len() >= 2048 {
            cache.entries.clear();
        }
    }
    cache
        .entries
        .insert(cache_key, (Instant::now(), coordinates.clone()));
    Ok(coordinates)
}
pub(crate) fn parse_coordinates(value: &Value) -> Result<Coordinates, ApiError> {
    let xy = value
        .pointer("/features/0/geometry/coordinates")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::new(422, "Адрес не найден. Укажите город, улицу и дом."))?;
    let (Some(lon), Some(lat)) = (
        xy.first().and_then(Value::as_f64),
        xy.get(1).and_then(Value::as_f64),
    ) else {
        return Err(ApiError::upstream());
    };
    let coordinates = Coordinates { lat, lon };
    if !valid_coordinates(&coordinates) {
        return Err(ApiError::upstream());
    }
    Ok(coordinates)
}
