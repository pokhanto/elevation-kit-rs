//! HTTP routes for tile lookup and tile streaming.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::sse::{Event, Sse},
    routing::get,
};
use elevation_domain::Bounds;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

use crate::{
    AppError, AppState, application::MeanElevationCalculationStrategy, domain::ElevationTile,
};

/// Query parameters for tile streaming over bounding box.
#[derive(Debug, Deserialize)]
pub struct TilesStreamRequest {
    pub zoom: u8,
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

/// HTTP response for tile.
#[derive(Serialize, Debug, Clone)]
pub struct TileResponse {
    id: String,
    elevation: Option<f64>,
}

impl From<ElevationTile> for TileResponse {
    fn from(value: ElevationTile) -> Self {
        Self {
            id: value.id().to_owned(),
            elevation: value.elevation().map(|e| e.0),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
enum ServerEvent {
    Tile(TileResponse),
    Error { message: String },
    Done,
}

/// Builds tile routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/stream", get(stream_tiles))
        .route("/{id}", get(get_tile))
}

/// Returns a single tile by id.
#[tracing::instrument(skip(state), fields(tile_id = %id))]
pub async fn get_tile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TileResponse>, AppError> {
    tracing::info!("starting handling get tile");

    let tile = state
        .tile_service
        .get_tile_by_id(id, MeanElevationCalculationStrategy)
        .await
        .inspect_err(|err| {
            tracing::error!(error = ?err, "failed to build tile");
        })?;

    Ok(Json(tile.into()))
}

/// Streams tiles for the requested bounding box.
#[tracing::instrument(
    skip(state),
    fields(
        zoom = request.zoom,
        min_lon = request.min_lon,
        min_lat = request.min_lat,
        max_lon = request.max_lon,
        max_lat = request.max_lat,
    )
)]
pub async fn stream_tiles(
    State(state): State<AppState>,
    Query(request): Query<TilesStreamRequest>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    tracing::info!("starting handling tiles stream");

    let TilesStreamRequest {
        zoom,
        min_lon,
        min_lat,
        max_lon,
        max_lat,
    } = request;

    let bbox = Bounds::try_new(min_lon, min_lat, max_lon, max_lat).map_err(|err| {
        tracing::error!(error = ?err, "invalid bbox provided in request");
        AppError::InvalidBounds
    })?;

    let tile_stream = state
        .tile_service
        .stream_tiles_for_bbox(bbox, zoom, MeanElevationCalculationStrategy)
        .inspect_err(|err| {
            tracing::error!(error = ?err, "failed to create tile stream");
        })?;

    let stream = tile_stream
        .map(|result| match result {
            Ok(tile) => ServerEvent::Tile(tile.into()),
            Err(err) => {
                tracing::error!(error = ?err, "failed to resolve tile in stream");
                ServerEvent::Error {
                    message: "failed to resolve tile".to_string(),
                }
            }
        })
        .chain(futures_util::stream::once(async { ServerEvent::Done }))
        .map(|payload| {
            let event_name = match &payload {
                ServerEvent::Tile(_) => "tile",
                ServerEvent::Error { .. } => "error",
                ServerEvent::Done => "done",
            };

            let event = match Event::default().event(event_name).json_data(payload) {
                Ok(event) => event,
                Err(err) => {
                    tracing::error!(error = ?err, "failed to serialize SSE event");

                    Event::default()
                        .event("error")
                        .data("Failed to serialize event")
                }
            };

            Ok(event)
        });

    Ok(Sse::new(stream))
}
