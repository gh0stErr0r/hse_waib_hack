//! Local Ollama chat API with constrained JSON output; no cloud credentials required.
use crate::error::ApiError;
use serde_json::{Value, json};
use std::time::Duration;

pub const DEFAULT_MODEL: &str = "qwen3:4b";

pub fn base_url() -> String {
    std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".into())
        .trim_end_matches('/')
        .to_string()
}

pub async fn extract(
    client: &reqwest::Client,
    system: &str,
    input: &str,
    schema: Value,
) -> Result<Value, ApiError> {
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into());
    if model.trim().is_empty() {
        return Err(ApiError::new(
            503,
            "Укажите OLLAMA_MODEL, например qwen3:4b",
        ));
    }
    let timeout = std::env::var("OLLAMA_TIMEOUT_SECS")
        .unwrap_or_else(|_| "180".into())
        .parse::<u64>()
        .ok()
        .filter(|n| (1..=210).contains(n))
        .ok_or_else(|| ApiError::new(503, "OLLAMA_TIMEOUT_SECS должен быть от 1 до 210"))?;
    request(client, &base_url(), &model, timeout, system, input, schema).await
}

pub(crate) async fn request(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    timeout: u64,
    system: &str,
    input: &str,
    schema: Value,
) -> Result<Value, ApiError> {
    let response = client
        .post(format!("{}/api/chat", base.trim_end_matches('/')))
        .json(&json!({
            "model": model,
            "stream": false,
            "think": false,
            "messages": [
                {"role": "system", "content": format!("{system}\nReturn only a JSON object matching this schema: {schema}. Omit preferences the user did not provide; do not fill missing fields with defaults.")},
                {"role": "user", "content": input}
            ],
            "format": schema,
            "options": {"temperature": 0, "num_ctx": 8192, "num_predict": 2048}
        }))
        .timeout(Duration::from_secs(timeout))
        .send()
        .await
        .map_err(|e| crate::upstream::transport("Ollama", "extract_preferences", e))?;
    let status = response.status();
    // A proxy may return HTML on errors. Classify HTTP failures before decoding JSON.
    if !status.is_success() {
        tracing::error!(
            provider = "Ollama",
            operation = "extract_preferences",
            http_status = status.as_u16(),
            "Ollama rejected request"
        );
        return Err(match status.as_u16() {
            404 => ApiError::new(
                503,
                "Модель Ollama не найдена. Выполните ollama pull для модели из OLLAMA_MODEL; проверьте OLLAMA_BASE_URL (без /api и /v1).",
            ),
            400 => ApiError::new(
                503,
                "Ollama отклонила запрос. Обновите Ollama и выберите модель с поддержкой JSON-схем, например qwen3:4b.",
            ),
            _ => ApiError::new(
                502,
                format!(
                    "Ollama вернула HTTP {}. Проверьте её логи и доступную память.",
                    status.as_u16()
                ),
            ),
        });
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| crate::upstream::json("Ollama", "extract_preferences", status.as_u16(), e))?;
    parse_response(&body)
}

pub(crate) fn parse_response(body: &Value) -> Result<Value, ApiError> {
    if body.get("done").and_then(Value::as_bool) != Some(true)
        || body.get("done_reason").and_then(Value::as_str) == Some("length")
    {
        return Err(ApiError::new(
            502,
            "Ollama не завершила ответ. Попробуйте сократить сообщение или число временных слотов.",
        ));
    }
    body.pointer("/message/content")
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .filter(Value::is_object)
        .ok_or_else(|| ApiError::new(502, "Ollama не вернула JSON с предпочтениями. Попробуйте уточнить сообщение или выбрать другую OLLAMA_MODEL."))
}
