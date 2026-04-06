//! Thin abstraction over low-level elevation service.

use elevation_adapters::{FsMetadataStorage, GdalRasterReader};
use elevation_core::{ElevationService, ElevationServiceError};
use elevation_domain::Elevation;

/// Error returned by [`ElevationProvider`].
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ElevationProviderError {
    #[error("Elevation service error")]
    Elevation(#[from] ElevationServiceError),
}

/// Abstraction over [`ElevationService`] used to reduce coupling and improve testability.
pub trait ElevationProvider {
    /// Returns elevation at given geographic point.
    ///
    /// For detailed behavior see
    /// [`elevation_core::ElevationService::elevation_at_point`].
    fn elevation_at_point(
        &self,
        lon: f64,
        lat: f64,
    ) -> impl Future<Output = Result<Option<Elevation>, ElevationProviderError>> + Send;
}

/// Real implementation [`ElevationProvider`] backed by [`ElevationService`].
impl ElevationProvider for ElevationService<FsMetadataStorage, GdalRasterReader> {
    async fn elevation_at_point(
        &self,
        lon: f64,
        lat: f64,
    ) -> Result<Option<Elevation>, ElevationProviderError> {
        let elevations = ElevationService::elevation_at_point(self, lon, lat).await?;

        Ok(elevations)
    }
}
