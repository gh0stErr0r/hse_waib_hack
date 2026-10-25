//! Provider diagnostics without logging request URLs, headers, prompts or credentials.
use crate::error::ApiError;
use serde_json::Value;
use std::time::{Duration, Instant};

pub fn transport(
    provider: &'static str,
    operation: &'static str,
    error: reqwest::Error,
) -> ApiError {
    let timeout = error.is_timeout();
    let detail = format!("{error:?}").to_lowercase();
    let dns = detail.contains("dns")
        || detail.contains("lookup address")
        || detail.contains("resolveerror");
    let status = error.status().map(|s| s.as_u16());
    tracing::error!(provider,operation,timeout,dns,http_status=status,error=?error.without_url(),"upstream request failed");
    if timeout {
        return ApiError::new(
            504,
            format!(
                "{provider}: превышено время ожидания ответа. Проверьте HTTPS-доступ из контейнера и настройки сети/VPN."
            ),
        );
    }
    if dns {
        return ApiError::new(
            502,
            format!(
                "{provider}: ошибка DNS внутри контейнера. Проверьте DNS Docker Desktop; пароль или API-ключ ещё не проверялись."
            ),
        );
    }
    if let Some(status) = status {
        return ApiError::new(
            502,
            format!("{provider}: внешний сервис вернул HTTP {status}."),
        );
    }
    ApiError::new(
        502,
        format!(
            "{provider}: не удалось выполнить HTTPS-запрос. Проверьте доступность сервиса, TLS и настройки прокси."
        ),
    )
}
pub fn json(
    provider: &'static str,
    operation: &'static str,
    status: u16,
    error: reqwest::Error,
) -> ApiError {
    if error.is_timeout() {
        return transport(provider, operation, error);
    }
    tracing::error!(provider,operation,http_status=status,error=?error.without_url(),"upstream returned invalid JSON");
    ApiError::new(
        502,
        format!(
            "{provider}: вместо ожидаемого JSON получен некорректный ответ (HTTP {status}). Проверьте прокси и сетевую фильтрацию."
        ),
    )
}

pub async fn check(client: &reqwest::Client) -> bool {
    let auth = std::env::var("AUTH_URL").unwrap_or_else(|_| "http://auth:8080".into());
    let gemini = std::env::var("GEMINI_API_BASE_URL")
        .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta".into());
    let photon =
        std::env::var("GEOCODER_URL").unwrap_or_else(|_| "https://photon.komoot.io/api/".into());
    let mut success = true;
    for (provider, url) in [
        ("Auth", format!("{}/health", auth.trim_end_matches('/'))),
        ("Gemini", format!("{}/models", gemini.trim_end_matches('/'))),
        ("Photon", photon),
    ] {
        let start = Instant::now();
        let mut request = client
            .get(url)
            .header("User-Agent", "GastroSlots/0.1 (connectivity diagnostic)")
            .timeout(Duration::from_secs(12));
        if provider == "Photon" {
            request = request.query(&[("q", "Москва"), ("limit", "1")]);
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                // Gemini is deliberately queried without a key. 400/401/403/429 prove HTTP connectivity, not generation access.
                let ok = match provider {
                    "Auth" | "Photon" => status.is_success(),
                    _ => status.is_success() || matches!(status.as_u16(), 400 | 401 | 403 | 429),
                };
                println!(
                    "{provider}: HTTP {} after {} ms{}",
                    status.as_u16(),
                    start.elapsed().as_millis(),
                    if provider == "Gemini" {
                        " (no API key sent; this does not verify model access or quota)"
                    } else {
                        ""
                    }
                );
                if !ok {
                    success = false;
                }
                if provider == "Photon" && ok && response.json::<Value>().await.is_err() {
                    println!("Photon: response is not JSON");
                    success = false;
                }
            }
            Err(error) => {
                success = false;
                let failure = transport(provider, "connectivity_check", error);
                println!(
                    "{provider}: {} after {} ms",
                    failure.1,
                    start.elapsed().as_millis()
                );
            }
        }
    }
    success
}
