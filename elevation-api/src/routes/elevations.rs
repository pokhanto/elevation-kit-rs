//! HTTP routes for serving elevations.

use axum::{Json, Router, extract::State, routing::post};

use crate::{
    AppError, AppState,
    domain::{Coordinate, CoordinateWithElevation},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(get_elevations))
}

#[tracing::instrument(skip(state))]
async fn get_elevations(
    State(state): State<AppState>,
    Json(payload): Json<Vec<Coordinate>>,
) -> Result<Json<Vec<CoordinateWithElevation>>, AppError> {
    tracing::info!("starting handling elevations at requested points");

    let coords_with_elevations = state
        .elevation_service
        .elevations_at_points(&payload)
        .await?;

    Ok(Json(coords_with_elevations))
}
