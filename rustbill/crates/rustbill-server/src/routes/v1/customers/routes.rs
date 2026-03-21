use crate::app::SharedState;
use axum::Router;

pub fn router() -> Router<SharedState> {
    crate::routes::customers::routes::v1_router()
}
