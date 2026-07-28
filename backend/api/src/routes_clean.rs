use axum::Router;

use crate::state::AppState;

pub fn contract_routes() -> Router<AppState> {
    crate::routes::contract_routes()
}
