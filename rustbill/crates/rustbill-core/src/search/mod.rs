pub mod repository;
pub mod schema;
pub mod service;

use crate::error::Result;
use repository::PgSearchRepository;
use schema::SearchResult;
use sqlx::PgPool;

pub async fn global_search(pool: &PgPool, query: &str) -> Result<Vec<SearchResult>> {
    let repo = PgSearchRepository::new(pool);
    service::global_search(&repo, query).await
}
