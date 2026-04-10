//! HTTP-facing application errors and response mapping.
//!
//! Converts internal service errors into API responses.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::application::TileServiceError;

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid bounds")]
    InvalidBounds,

    #[error("Invalid zoom level")]
    InvalidZoomLevel,

    #[error("Invalid chunk resolution")]
    InvalidChunkResolution,

    #[error("Tile not found")]
    TileNotFound,

    #[error("Failed to resolve tiles")]
    ResolveTiles,

    #[error("Failed to compute tile data")]
    ComputeTileData,
}

impl From<TileServiceError> for AppError {
    fn from(value: TileServiceError) -> Self {
        match value {
            TileServiceError::ZoomLevel => AppError::InvalidZoomLevel,
            TileServiceError::ChunkResolution => AppError::InvalidChunkResolution,
            TileServiceError::UnknownTile => AppError::TileNotFound,
            TileServiceError::BuildTiles => AppError::ResolveTiles,
            TileServiceError::Elevation => AppError::ComputeTileData,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::InvalidBounds => StatusCode::BAD_REQUEST,
            AppError::InvalidZoomLevel => StatusCode::BAD_REQUEST,
            AppError::InvalidChunkResolution => StatusCode::BAD_REQUEST,
            AppError::TileNotFound => StatusCode::NOT_FOUND,
            AppError::ResolveTiles => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ComputeTileData => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}
