//! Abstraction for retrieving elevation data.
//!
//! Main purpose is to keep high level services testable by allowing
//! other providers like fakes or mocks.
use elevation_adapters::{FsArtifactResolver, FsMetadataStorage, GdalRasterReader};
use elevation_core::{ElevationService, ElevationServiceError};
use elevation_domain::{BboxElevations, Bounds, ResolutionHint};

/// Error returned by [`ElevationProvider`].
///
/// This wraps lower level elevation service errors.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ElevationProviderError {
    /// Elevation service failed.
    #[error("Elevation service error")]
    Elevation(#[from] ElevationServiceError),
}

/// Provides elevation values for bounding box.
pub trait ElevationProvider {
    /// Returns elevations for given bounding box.
    ///
    /// For detailed behavior see
    /// [`elevation_core::ElevationService::elevations_in_bbox`].
    fn elevations_in_bbox(
        &self,
        bbox: Bounds,
        hint: Option<ResolutionHint>,
    ) -> impl Future<Output = Result<BboxElevations, ElevationProviderError>>;
}

/// Production [`ElevationProvider`] implementation backed by
/// [`ElevationService<FsMetadataStorage, GdalRasterReader>`].
impl ElevationProvider
    for ElevationService<FsMetadataStorage, GdalRasterReader<FsArtifactResolver>>
{
    async fn elevations_in_bbox(
        &self,
        bbox: Bounds,
        hint: Option<ResolutionHint>,
    ) -> Result<BboxElevations, ElevationProviderError> {
        let elevations = ElevationService::elevations_in_bbox(self, bbox, hint).await?;
        Ok(elevations)
    }
}
