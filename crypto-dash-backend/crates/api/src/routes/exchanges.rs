use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use crypto_dash_core::model::ExchangeInfo;

/// GET /api/exchanges - List supported exchanges and their status
pub async fn list_exchanges(
    State(state): State<AppState>,
) -> Result<Json<Vec<ExchangeInfo>>, StatusCode> {
    let exchanges = state.get_exchange_info().await;
    Ok(Json(exchanges))
}
