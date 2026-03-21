pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::CreditNote;
use crate::error::Result;
use repository::PgCreditNotesRepository;
use sqlx::PgPool;

pub use schema::{
    CreateCreditNoteDraft, CreateCreditNoteRequest, CreditNoteItemDraft, CreditNoteItemInput,
    CreditNoteView, ListCreditNotesFilter, UpdateCreditNoteDraft, UpdateCreditNoteRequest,
};

pub async fn list_credit_notes(
    pool: &PgPool,
    filter: &ListCreditNotesFilter,
) -> Result<Vec<CreditNoteView>> {
    let repo = PgCreditNotesRepository::new(pool);
    service::list_credit_notes(&repo, filter).await
}

pub async fn get_credit_note(pool: &PgPool, id: &str) -> Result<CreditNote> {
    let repo = PgCreditNotesRepository::new(pool);
    service::get_credit_note(&repo, id).await
}

pub async fn create_credit_note(pool: &PgPool, req: CreateCreditNoteRequest) -> Result<CreditNote> {
    let repo = PgCreditNotesRepository::new(pool);
    service::create_credit_note(&repo, req).await
}

pub async fn update_credit_note(
    pool: &PgPool,
    id: &str,
    req: UpdateCreditNoteRequest,
) -> Result<CreditNote> {
    let repo = PgCreditNotesRepository::new(pool);
    service::update_credit_note(&repo, id, req).await
}

pub async fn delete_credit_note(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgCreditNotesRepository::new(pool);
    service::delete_credit_note(&repo, id).await
}
