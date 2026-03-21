use crate::app::SharedState;
use axum::Router;

pub fn router() -> Router<SharedState> {
    crate::routes::deals::routes::v1_router()
}
