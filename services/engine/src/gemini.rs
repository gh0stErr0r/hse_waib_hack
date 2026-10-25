//! Gemini transport. Interactions is the default; legacy generateContent is opt-in.
use crate::error::ApiError;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Mode {
    Function,
    Search,
    Structured,
}

fn operation(mode: Mode) -> &'static str {
    match mode {
        Mode::Function => "extract_preferences",
        Mode::Search => "google_search",
        Mode::Structured => "extract_search_results",
    }
}

pub async fn request(
    client: &reqwest::Client,
    mode: Mode,
    system: &str,
    input: &str,
    schema: Option<Value>,
) -> Result<Value, ApiError> {
    let key = std::env::var("GEMINI_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ApiError::new(
                503,
                "Добавьте GEMINI_API_KEY в infra/.env и перезапустите engine.",
            )
        })?;
    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-3.8-flash".into());
    if model.is_empty()
        || !model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
    {
        return Err(ApiError::new(503, "Некорректное имя GEMINI_MODEL"));
    }
    let base = std::env::var("GEMINI_API_BASE_URL")
        .unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta".into());
    let legacy = std::env::var("GEMINI_API_MODE").as_deref() == Ok("generate_content");
    let (url, body) = request_body(&base, &model, legacy, mode, system, input, schema);
    let timeout = if matches!(mode, Mode::Search) { 70 } else { 40 };
    let response = client
        .post(url)
        .header("x-goog-api-key", key)
        .json(&body)
        .timeout(Duration::from_secs(timeout))
        .send()
        .await
        .map_err(|e| crate::upstream::transport("Gemini", operation(mode), e))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| crate::upstream::json("Gemini", operation(mode), status.as_u16(), e))?;
    if !status.is_success() {
        let error = provider_error(status.as_u16(), &body);
        let provider_status = body
            .pointer("/error/status")
            .and_then(Value::as_str)
            .filter(|s| {
                matches!(
                    *s,
                    "INVALID_ARGUMENT"
                        | "FAILED_PRECONDITION"
                        | "UNAUTHENTICATED"
                        | "PERMISSION_DENIED"
                        | "NOT_FOUND"
                        | "RESOURCE_EXHAUSTED"
                        | "UNAVAILABLE"
                        | "INTERNAL"
                        | "DEADLINE_EXCEEDED"
                )
            })
            .unwrap_or("UNKNOWN");
        tracing::error!(provider="Gemini",operation=operation(mode),http_status=status.as_u16(),provider_status,reason=%error.1,"Gemini rejected request");
        return Err(error);
    }
    if body
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|s| matches!(s, "failed" | "cancelled" | "incomplete"))
    {
        return Err(ApiError::new(
            502,
            "Gemini не завершил запрос. Повторите его позже.",
        ));
    }
    Ok(body)
}

pub(crate) fn request_body(
    base: &str,
    model: &str,
    legacy: bool,
    mode: Mode,
    system: &str,
    input: &str,
    schema: Option<Value>,
) -> (String, Value) {
    if legacy {
        let mut body = json!({"systemInstruction":{"parts":[{"text":system}]},"contents":[{"role":"user","parts":[{"text":input}]}],"generationConfig":{"temperature":0}});
        match mode {
            Mode::Function => {
                body["tools"] = json!([{"functionDeclarations":[schema.unwrap_or(Value::Null)]}]);
                body["toolConfig"] = json!({"functionCallingConfig":{"mode":"ANY","allowedFunctionNames":["query_places"]}});
            }
            Mode::Search => body["tools"] = json!([{"google_search":{}}]),
            Mode::Structured => {
                body["generationConfig"]["responseMimeType"] = json!("application/json");
                body["generationConfig"]["responseJsonSchema"] = schema.unwrap_or(Value::Null);
            }
        }
        (
            format!(
                "{}/models/{model}:generateContent",
                base.trim_end_matches('/')
            ),
            body,
        )
    } else {
        let mut body = json!({"model":model,"input":input,"system_instruction":system,"store":false,"generation_config":{"temperature":0}});
        match mode {
            Mode::Function => {
                let mut tool = schema.unwrap_or(json!({}));
                tool["type"] = json!("function");
                body["tools"] = json!([tool]);
                body["generation_config"]["tool_choice"] = json!("any");
            }
            Mode::Search => body["tools"] = json!([{"type":"google_search"}]),
            Mode::Structured => {
                body["response_format"] =
                    json!({"type":"text","mime_type":"application/json","schema":schema})
            }
        }
        (format!("{}/interactions", base.trim_end_matches('/')), body)
    }
}

pub(crate) fn provider_error(status: u16, body: &Value) -> ApiError {
    let message = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    if status == 429 {
        return ApiError::new(
            429,
            "Квота Gemini исчерпана или превышен лимит запросов. Проверьте квоты и биллинг в Google AI Studio; автоматический повтор не выполняется.",
        );
    }
    if message.contains("location") || message.contains("region") {
        return ApiError::new(
            503,
            "Gemini недоступен из региона сервера. Для реального поиска нужен доступ к Gemini API из поддерживаемого региона.",
        );
    }
    if status == 401 || status == 403 || message.contains("api key") {
        return ApiError::new(
            503,
            "Gemini отклонил API-ключ или у проекта нет доступа к выбранной модели. Проверьте GEMINI_API_KEY и права в AI Studio.",
        );
    }
    if status == 404 {
        return ApiError::new(
            503,
            "Модель Gemini недоступна. Обновите GEMINI_MODEL в infra/.env и перезапустите engine.",
        );
    }
    ApiError::new(
        502,
        "Gemini не смог обработать запрос. Проверьте модель, доступность инструментов и конфигурацию API.",
    )
}

pub(crate) fn text_parts(response: &Value) -> Vec<Value> {
    if let Some(parts) = response
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
    {
        return parts
            .iter()
            .filter(|p| p.get("thought") != Some(&json!(true)) && p.get("text").is_some())
            .cloned()
            .collect();
    }
    response
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|s| s.get("type").and_then(Value::as_str) == Some("model_output"))
        .filter_map(|s| s.get("content").and_then(Value::as_array))
        .flatten()
        .filter(|c| c.get("type").and_then(Value::as_str) == Some("text"))
        .cloned()
        .collect()
}
pub(crate) fn response_text(response: &Value) -> String {
    text_parts(response)
        .iter()
        .filter_map(|p| p.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}
pub(crate) fn function_args(response: &Value) -> Option<Value> {
    if let Some(parts) = response
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
    {
        return parts.iter().find_map(|p| {
            let call = p.get("functionCall")?;
            if call.get("name")?.as_str()? == "query_places" {
                call.get("args").cloned()
            } else {
                None
            }
        });
    }
    response.get("steps")?.as_array()?.iter().find_map(|s| {
        if s.get("type")?.as_str()? == "function_call" && s.get("name")?.as_str()? == "query_places"
        {
            let args = s.get("arguments")?;
            if let Some(text) = args.as_str() {
                serde_json::from_str(text).ok()
            } else {
                Some(args.clone())
            }
        } else {
            None
        }
    })
}
