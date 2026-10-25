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
        if self.0.is_server_error() {
            tracing::warn!(http_status=self.0.as_u16(),reason=%self.1,"request failed");
        }
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
