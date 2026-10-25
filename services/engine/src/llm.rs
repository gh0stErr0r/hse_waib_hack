//! Preference extraction is independent of the source of venue data.
use crate::{error::ApiError, gemini, ollama};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Gemini,
    Ollama,
}

pub fn provider() -> Result<Provider, ApiError> {
    match std::env::var("LLM_PROVIDER").as_deref() {
        Ok("gemini") | Err(_) => Ok(Provider::Gemini),
        Ok("ollama") => Ok(Provider::Ollama),
        _ => Err(ApiError::new(
            503,
            "LLM_PROVIDER должен быть gemini или ollama",
        )),
    }
}

pub async fn extract(
    client: &reqwest::Client,
    system: &str,
    input: &str,
    declaration: Value,
) -> Result<Value, ApiError> {
    match provider()? {
        Provider::Ollama => {
            ollama::extract(client, system, input, declaration["parameters"].clone()).await
        }
        Provider::Gemini => {
            let response = gemini::request(
                client,
                gemini::Mode::Function,
                system,
                input,
                Some(declaration),
            )
            .await?;
            gemini::function_args(&response).ok_or_else(|| {
                ApiError::new(
                    502,
                    "Gemini не вернул структурированные предпочтения. Попробуйте уточнить сообщение.",
                )
            })
        }
    }
}
