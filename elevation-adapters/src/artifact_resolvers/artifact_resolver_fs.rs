use elevation_domain::{ArtifactLocator, ArtifactResolveError, ArtifactResolver};

/// Resolves local filesystem artifact locators as is.
#[derive(Debug, Clone, Default)]
pub struct FsArtifactResolver;

impl ArtifactResolver for FsArtifactResolver {
    fn resolve(&self, locator: &ArtifactLocator) -> Result<String, ArtifactResolveError> {
        Ok(locator.to_string())
    }
}
