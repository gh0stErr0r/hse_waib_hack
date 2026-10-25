use crate::error::ApiError;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool},
    sql_types::{Integer, Jsonb, Text, Timestamptz, Uuid as SqlUuid},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;
pub type Db = Pool<ConnectionManager<PgConnection>>;
const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
#[derive(Debug, QueryableByName, Serialize)]
pub struct Conversation {
    #[diesel(sql_type=SqlUuid)]
    pub id: Uuid,
    #[diesel(sql_type=Text)]
    pub title: String,
    #[diesel(sql_type=Jsonb)]
    pub document: Value,
    #[diesel(sql_type=Integer)]
    pub version: i32,
    #[diesel(sql_type=Timestamptz)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
pub fn init(url: &str) -> Result<Db, Box<dyn std::error::Error + Send + Sync>> {
    let pool = Pool::builder()
        .max_size(12)
        .connection_timeout(std::time::Duration::from_secs(5))
        .build(ConnectionManager::<PgConnection>::new(url))?;
    pool.get()?.run_pending_migrations(MIGRATIONS)?;
    Ok(pool)
}
pub async fn run<T: Send + 'static>(
    pool: Db,
    f: impl FnOnce(&mut PgConnection) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(move || {
        let mut c = pool.get().map_err(|_| ApiError::internal())?;
        f(&mut c)
    })
    .await
    .map_err(|_| ApiError::internal())?
}
pub async fn create(pool: Db, owner: String) -> Result<Conversation, ApiError> {
    run(pool,move|c|diesel::sql_query("INSERT INTO gastro_conversations(id,owner_id) VALUES($1,$2) RETURNING id,title,document,version,updated_at").bind::<SqlUuid,_>(Uuid::new_v4()).bind::<Text,_>(owner).get_result(c).map_err(Into::into)).await
}
pub async fn get(pool: Db, owner: String, id: Uuid) -> Result<Conversation, ApiError> {
    run(pool,move|c|diesel::sql_query("SELECT id,title,document,version,updated_at FROM gastro_conversations WHERE id=$1 AND owner_id=$2").bind::<SqlUuid,_>(id).bind::<Text,_>(owner).get_result(c).map_err(Into::into)).await
}
pub async fn list(pool: Db, owner: String, offset: i64) -> Result<Vec<Conversation>, ApiError> {
    run(pool,move|c|diesel::sql_query("SELECT id,title,'{}'::jsonb AS document,version,updated_at FROM gastro_conversations WHERE owner_id=$1 ORDER BY updated_at DESC,id LIMIT 30 OFFSET $2").bind::<Text,_>(owner).bind::<diesel::sql_types::BigInt,_>(offset).load(c).map_err(Into::into)).await
}
pub async fn save(
    pool: Db,
    owner: String,
    id: Uuid,
    version: i32,
    doc: Value,
    title: String,
) -> Result<(), ApiError> {
    run(pool,move|c| {
 let n=diesel::sql_query("UPDATE gastro_conversations SET document=$1,title=$2,version=version+1,updated_at=now() WHERE id=$3 AND owner_id=$4 AND version=$5").bind::<Jsonb,_>(doc).bind::<Text,_>(title).bind::<SqlUuid,_>(id).bind::<Text,_>(owner).bind::<Integer,_>(version).execute(c)?;
 if n==0 {Err(ApiError::new(409,"Диалог изменён другим подключением. Обновите историю."))} else {Ok(())}
 }).await
}
