use super::repository::CreditNotesRepository;
use super::schema::{
    CreateCreditNoteDraft, CreateCreditNoteRequest, CreditNoteItemDraft, CreditNoteView,
    ListCreditNotesFilter, UpdateCreditNoteDraft, UpdateCreditNoteRequest,
};
use crate::db::models::{CreditNote, CreditNoteStatus};
use crate::error::{BillingError, Result};
use chrono::Utc;
use rust_decimal::Decimal;
use validator::Validate;

pub async fn list_credit_notes<R: CreditNotesRepository + ?Sized>(
    repo: &R,
    filter: &ListCreditNotesFilter,
) -> Result<Vec<CreditNoteView>> {
    repo.list_credit_notes(filter).await
}

pub async fn get_credit_note<R: CreditNotesRepository + ?Sized>(
    repo: &R,
    id: &str,
) -> Result<CreditNote> {
    repo.get_credit_note(id)
        .await?
        .ok_or_else(|| BillingError::not_found("credit_note", id))
}

pub async fn create_credit_note<R: CreditNotesRepository + ?Sized>(
    repo: &R,
    req: CreateCreditNoteRequest,
) -> Result<CreditNote> {
    req.validate().map_err(BillingError::from_validation)?;

    if req.items.is_empty() {
        return Err(BillingError::bad_request(
            "at least one credit note item is required",
        ));
    }

    for item in &req.items {
        item.validate().map_err(BillingError::from_validation)?;
    }

    let items: Vec<CreditNoteItemDraft> = req
        .items
        .iter()
        .map(|item| CreditNoteItemDraft {
            description: item.description.clone(),
            quantity: item.quantity,
            unit_price: item.unit_price,
            amount: (item.quantity * item.unit_price).round_dp(2),
        })
        .collect();
    let amount = items.iter().map(|item| item.amount).sum::<Decimal>();
    let status = req.status.unwrap_or(CreditNoteStatus::Draft);
    let issued_at = if status == CreditNoteStatus::Issued {
        Some(Utc::now().naive_utc())
    } else {
        None
    };

    let draft = CreateCreditNoteDraft {
        invoice_id: req.invoice_id,
        customer_id: req.customer_id,
        reason: req.reason,
        amount,
        status,
        issued_at,
        items,
    };

    repo.create_credit_note(&draft).await
}

pub async fn update_credit_note<R: CreditNotesRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateCreditNoteRequest,
) -> Result<CreditNote> {
    req.validate().map_err(BillingError::from_validation)?;

    let _existing = get_credit_note(repo, id).await?;

    let status = req.status.clone();
    let draft = UpdateCreditNoteDraft {
        reason: req.reason,
        status: status.clone(),
        issued_at: if status == Some(CreditNoteStatus::Issued) {
            Some(Utc::now().naive_utc())
        } else {
            None
        },
    };

    repo.update_credit_note(id, &draft).await
}

pub async fn delete_credit_note<R: CreditNotesRepository + ?Sized>(
    repo: &R,
    id: &str,
) -> Result<()> {
    let affected = repo.delete_credit_note(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("credit_note", id));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::credit_notes::CreditNoteItemInput;
    use crate::db::models::{CreditNote, CreditNoteStatus};
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        note: Option<CreditNote>,
        list_rows: Vec<CreditNoteView>,
        created_draft: Option<CreateCreditNoteDraft>,
        updated_draft: Option<(String, UpdateCreditNoteDraft)>,
        deleted_id: Option<String>,
    }

    #[derive(Clone, Default)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    impl StubRepo {
        fn with_state(state: StubState) -> Self {
            Self {
                state: Arc::new(Mutex::new(state)),
            }
        }
    }

    fn sample_note(status: CreditNoteStatus) -> CreditNote {
        CreditNote {
            id: "cn_1".to_string(),
            credit_note_number: "CN-00000001".to_string(),
            invoice_id: "inv_1".to_string(),
            customer_id: "cus_1".to_string(),
            reason: "Adjustment".to_string(),
            amount: Decimal::from(25),
            status,
            issued_at: None,
            deleted_at: None,
            created_at: NaiveDate::from_ymd_opt(2026, 1, 1)
                .expect("date")
                .and_hms_opt(0, 0, 0)
                .expect("time"),
            updated_at: NaiveDate::from_ymd_opt(2026, 1, 1)
                .expect("date")
                .and_hms_opt(0, 0, 0)
                .expect("time"),
        }
    }

    fn sample_request() -> CreateCreditNoteRequest {
        CreateCreditNoteRequest {
            invoice_id: "inv_1".to_string(),
            customer_id: "cus_1".to_string(),
            reason: "Adjustment".to_string(),
            status: None,
            items: vec![CreditNoteItemInput {
                description: "Item 1".to_string(),
                quantity: Decimal::from(1),
                unit_price: Decimal::from(25),
            }],
        }
    }

    #[async_trait]
    impl CreditNotesRepository for StubRepo {
        async fn list_credit_notes(
            &self,
            _filter: &ListCreditNotesFilter,
        ) -> Result<Vec<CreditNoteView>> {
            Ok(self.state.lock().expect("mutex").list_rows.clone())
        }

        async fn get_credit_note(&self, _id: &str) -> Result<Option<CreditNote>> {
            Ok(self.state.lock().expect("mutex").note.clone())
        }

        async fn create_credit_note(&self, draft: &CreateCreditNoteDraft) -> Result<CreditNote> {
            let mut state = self.state.lock().expect("mutex");
            state.created_draft = Some(draft.clone());
            Ok(state.note.clone().expect("note"))
        }

        async fn update_credit_note(
            &self,
            id: &str,
            draft: &UpdateCreditNoteDraft,
        ) -> Result<CreditNote> {
            let mut state = self.state.lock().expect("mutex");
            state.updated_draft = Some((id.to_string(), draft.clone()));
            Ok(state.note.clone().expect("note"))
        }

        async fn delete_credit_note(&self, id: &str) -> Result<u64> {
            self.state.lock().expect("mutex").deleted_id = Some(id.to_string());
            Ok(1)
        }
    }

    #[tokio::test]
    async fn create_credit_note_builds_draft_and_forwards() {
        let repo = StubRepo::with_state(StubState {
            note: Some(sample_note(CreditNoteStatus::Draft)),
            ..StubState::default()
        });

        let result = create_credit_note(&repo, sample_request())
            .await
            .expect("create_credit_note");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(result.id, "cn_1");
        assert!(state.created_draft.is_some());
        assert_eq!(
            state.created_draft.as_ref().unwrap().amount,
            Decimal::from(25)
        );
    }

    #[tokio::test]
    async fn create_credit_note_rejects_empty_items() {
        let repo = StubRepo::default();

        let err = create_credit_note(
            &repo,
            CreateCreditNoteRequest {
                invoice_id: "inv_1".to_string(),
                customer_id: "cus_1".to_string(),
                reason: "Adjustment".to_string(),
                status: None,
                items: vec![],
            },
        )
        .await
        .expect_err("should fail");

        assert!(err.to_string().contains("at least one credit note item"));
    }

    #[tokio::test]
    async fn update_credit_note_returns_not_found_when_missing() {
        let repo = StubRepo::default();

        let err = update_credit_note(
            &repo,
            "cn_1",
            UpdateCreditNoteRequest {
                reason: Some("updated".to_string()),
                status: Some(CreditNoteStatus::Issued),
            },
        )
        .await
        .expect_err("should fail");

        assert!(matches!(
            err,
            BillingError::NotFound {
                entity: "credit_note",
                id
            } if id == "cn_1"
        ));
    }

    #[tokio::test]
    async fn delete_credit_note_maps_zero_rows_to_not_found() {
        struct ZeroDeleteRepo;

        #[async_trait]
        impl CreditNotesRepository for ZeroDeleteRepo {
            async fn list_credit_notes(
                &self,
                _filter: &ListCreditNotesFilter,
            ) -> Result<Vec<CreditNoteView>> {
                Ok(vec![])
            }

            async fn get_credit_note(&self, _id: &str) -> Result<Option<CreditNote>> {
                Ok(Some(sample_note(CreditNoteStatus::Draft)))
            }

            async fn create_credit_note(
                &self,
                _draft: &CreateCreditNoteDraft,
            ) -> Result<CreditNote> {
                Ok(sample_note(CreditNoteStatus::Draft))
            }

            async fn update_credit_note(
                &self,
                _id: &str,
                _draft: &UpdateCreditNoteDraft,
            ) -> Result<CreditNote> {
                Ok(sample_note(CreditNoteStatus::Draft))
            }

            async fn delete_credit_note(&self, _id: &str) -> Result<u64> {
                Ok(0)
            }
        }

        let err = delete_credit_note(&ZeroDeleteRepo, "cn_1")
            .await
            .expect_err("should fail");
        assert!(matches!(
            err,
            BillingError::NotFound {
                entity: "credit_note",
                id
            } if id == "cn_1"
        ));
    }
}
