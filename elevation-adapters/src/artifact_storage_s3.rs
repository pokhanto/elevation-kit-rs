//! S3-backed artifact storage.

use std::path::Path;

use aws_sdk_s3::Client;
use elevation_domain::{ArtifactLocator, ArtifactStorage, ArtifactStorageError};

#[derive(Debug, Clone)]
pub struct S3ArtifactStorage {
    client: Client,
    bucket: String,
    prefix: Option<String>,
}

const GEOTIFF_CONTENT_TYPE: &str = "image/tiff";

impl S3ArtifactStorage {
    pub fn new<B, P>(client: Client, bucket: B, prefix: Option<P>) -> Self
    where
        B: Into<String>,
        P: Into<String>,
    {
        Self {
            client,
            bucket: bucket.into(),
            prefix: prefix.map(Into::into),
        }
    }

    fn object_key(&self, dataset_id: &str) -> String {
        match &self.prefix {
            Some(prefix) if !prefix.is_empty() => {
                format!("{}/{}.tif", prefix.trim_end_matches('/'), dataset_id)
            }
            _ => format!("{dataset_id}.tif"),
        }
    }
}

impl ArtifactStorage for S3ArtifactStorage {
    #[tracing::instrument(skip(self), fields(dataset_id, source_path = %source_path.display()))]
    async fn save_artifact(
        &self,
        dataset_id: &str,
        source_path: &Path,
    ) -> Result<ArtifactLocator, ArtifactStorageError> {
        let object_key = self.object_key(dataset_id);

        tracing::debug!(
            bucket = %self.bucket,
            object_key = %object_key,
            "preparing to upload artifact to s3"
        );

        let body = aws_sdk_s3::primitives::ByteStream::from_path(source_path.to_path_buf())
            .await
            .map_err(|err| {
                tracing::debug!(
                    error = %err,
                    source_path = %source_path.display(),
                    "failed to create s3 upload stream from file"
                );
                ArtifactStorageError::Save
            })?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .body(body)
            .content_type(GEOTIFF_CONTENT_TYPE)
            .send()
            .await
            .map_err(|err| {
                tracing::debug!(
                    error = %err,
                    bucket = %self.bucket,
                    object_key = %object_key,
                    "failed to upload artifact to s3"
                );
                ArtifactStorageError::Save
            })?;

        let locator = ArtifactLocator::new(format!("s3://{}/{}", self.bucket, object_key));

        tracing::debug!(locator = %locator, "artifact uploaded to s3");

        Ok(locator)
    }
}
