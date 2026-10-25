mod db;
mod dialogue;
mod error;
mod gemini;
mod geocoding;
mod llm;
mod model;
mod ollama;
mod providers;
mod ranking;
#[cfg(test)]
mod tests;
mod upstream;
mod web_search;
use axum::{
    Json, Router,
    extract::Request,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use error::ApiError;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;
#[derive(Clone)]
struct App {
    db: db::Db,
    client: reqwest::Client,
    auth_url: String,
}
#[derive(Clone)]
struct Owner(String);
async fn authenticate(
    State(app): State<Arc<App>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let owner = identify(&app, req.headers()).await?;
    req.extensions_mut().insert(owner);
    Ok(next.run(req).await)
}
async fn identify(app: &App, headers: &HeaderMap) -> Result<Owner, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .filter(|x| x.starts_with("Bearer ") && x.len() > 7 && x.len() < 8192)
        .ok_or_else(|| ApiError::new(401, "Требуется Bearer JWT"))?;
    let res = app
        .client
        .get(format!("{}/api/auth/users/me", app.auth_url))
        .header("authorization", token)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| crate::upstream::transport("Auth", "validate_session", e))?;
    if res.status() == StatusCode::UNAUTHORIZED || res.status() == StatusCode::FORBIDDEN {
        return Err(ApiError::new(401, "Сессия истекла или отозвана"));
    }
    let status = res.status().as_u16();
    if !res.status().is_success() {
        tracing::error!(
            provider = "Auth",
            operation = "validate_session",
            http_status = status,
            "auth request failed"
        );
        return Err(ApiError::new(
            502,
            format!("Auth: сервис авторизации вернул HTTP {status}."),
        ));
    }
    let v: Value = res
        .json()
        .await
        .map_err(|e| crate::upstream::json("Auth", "validate_session", status, e))?;
    if v.get("status").and_then(Value::as_str) != Some("active") {
        return Err(ApiError::new(403, "Аккаунт неактивен"));
    }
    let id = v
        .get("user_id")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(ApiError::upstream)?;
    Ok(Owner(id.to_string()))
}
async fn rank(Json(req): Json<model::RankRequest>) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"slots":ranking::rank(&req).map_err(|e|ApiError::new(422,e))?}),
    ))
}
async fn create(
    State(a): State<Arc<App>>,
    axum::Extension(Owner(owner)): axum::Extension<Owner>,
) -> Result<Json<db::Conversation>, ApiError> {
    Ok(Json(db::create(a.db.clone(), owner).await?))
}
#[derive(Deserialize)]
struct Pagination {
    #[serde(default)]
    offset: i64,
}
async fn list(
    State(a): State<Arc<App>>,
    axum::Extension(Owner(owner)): axum::Extension<Owner>,
    Query(p): Query<Pagination>,
) -> Result<Json<Value>, ApiError> {
    if !(0..=100_000).contains(&p.offset) {
        return Err(ApiError::new(422, "Некорректный offset"));
    }
    Ok(Json(
        json!({"conversations":db::list(a.db.clone(),owner,p.offset).await?}),
    ))
}
async fn read(
    State(a): State<Arc<App>>,
    axum::Extension(Owner(owner)): axum::Extension<Owner>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::Conversation>, ApiError> {
    Ok(Json(db::get(a.db.clone(), owner, id).await?))
}
#[derive(Deserialize)]
struct Message {
    request_id: Uuid,
    text: String,
}
async fn message(
    State(a): State<Arc<App>>,
    axum::Extension(Owner(owner)): axum::Extension<Owner>,
    Path(id): Path<Uuid>,
    Json(m): Json<Message>,
) -> Result<Json<Value>, ApiError> {
    if m.text.trim().is_empty() || m.text.chars().count() > 4000 {
        return Err(ApiError::new(422, "Введите от 1 до 4000 символов"));
    }
    let conv = db::get(a.db.clone(), owner.clone(), id).await?;
    let mut messages = conv
        .document
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(existing) = messages.iter().find(|v| {
        v.get("request_id") == Some(&json!(m.request_id))
            && v.get("type") == Some(&json!("assistant"))
    }) {
        return Ok(Json(existing.clone()));
    }
    if messages.len() >= 400 {
        return Err(ApiError::new(
            422,
            "Диалог достиг лимита. Создайте новый чат.",
        ));
    }
    let mut profile: dialogue::Profile =
        serde_json::from_value(conv.document.get("profile").cloned().unwrap_or(json!({})))
            .map_err(|_| ApiError::internal())?;
    let parsed = providers::extract(&a.client, &profile, &messages, &m.text).await?;
    profile.merge(parsed)?;
    let mut reply = dialogue::answer(&a.client, &profile).await?;
    reply["request_id"] = json!(m.request_id);
    reply["conversation_id"] = json!(id);
    reply["created_at"] = json!(chrono::Utc::now());
    messages.push(json!({"type":"user","text":m.text,"request_id":m.request_id,"created_at":chrono::Utc::now()}));
    messages.push(reply.clone());
    let title = if conv.version == 0 {
        m.text.chars().take(80).collect()
    } else {
        conv.title
    };
    db::save(
        a.db.clone(),
        owner,
        id,
        conv.version,
        json!({"profile":profile,"messages":messages}),
        title,
    )
    .await?;
    Ok(Json(reply))
}
async fn health(State(a): State<Arc<App>>) -> Result<Json<Value>, ApiError> {
    db::run(a.db.clone(), |c| {
        use diesel::RunQueryDsl;
        diesel::sql_query("SELECT 1").execute(c)?;
        Ok(())
    })
    .await?;
    Ok(Json(json!({"status":"ok"})))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(40))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    if std::env::args().any(|arg| arg == "--check-upstreams") {
        let ok = crate::upstream::check(&client).await;
        std::process::exit(if ok { 0 } else { 1 });
    }
    let app = Arc::new(App {
        db: db::init(&std::env::var("DATABASE_URL")?)?,
        client,
        auth_url: std::env::var("AUTH_URL")
            .unwrap_or("http://auth:8080".into())
            .trim_end_matches('/')
            .into(),
    });
    let router = router(app);
    let listener =
        tokio::net::TcpListener::bind(std::env::var("BIND_ADDR").unwrap_or("0.0.0.0:8082".into()))
            .await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            #[cfg(unix)]
            {
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("signal handler");
                tokio::select! {_=tokio::signal::ctrl_c()=>{},_=term.recv()=>{}}
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
        })
        .await?;
    Ok(())
}

fn router(app: Arc<App>) -> Router {
    let private = Router::new()
        .route("/api/v1/rank", post(rank))
        .route("/api/v1/conversations", get(list).post(create))
        .route("/api/v1/conversations/{id}", get(read))
        .route("/api/v1/conversations/{id}/messages", post(message))
        .route(
            "/api/v1/session",
            get(|| async { Json(json!({"status":"authenticated"})) }),
        )
        .route_layer(middleware::from_fn_with_state(app.clone(), authenticate));
    Router::new()
        .merge(private)
        .route("/health", get(health))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(240),
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(app)
}
