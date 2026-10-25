use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);
impl ApiError {
    pub fn new(code: u16, msg: impl Into<String>) -> Self {
        Self(
            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            msg.into(),
        )
    }
    pub fn internal() -> Self {
        Self::new(500, "Внутренняя ошибка сервиса")
    }
    pub fn upstream() -> Self {
        Self::new(502, "Внешний сервис временно недоступен. Повторите запрос.")
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(serde_json::json!({"type":"error","text":self.1})),
        )
            .into_response()
    }
}
impl From<diesel::result::Error> for ApiError {
    fn from(e: diesel::result::Error) -> Self {
        if matches!(e, diesel::result::Error::NotFound) {
            Self::new(404, "Диалог не найден")
        } else {
            tracing::error!(error=%e,"database operation failed");
            Self::internal()
        }
    }
}
