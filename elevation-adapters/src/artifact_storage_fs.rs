use elevation_types::{ArtifactStorage, ArtifactStorageError};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{debug, instrument};

pub struct FsArtifactStorage {
    base_dir: PathBuf,
}

impl FsArtifactStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

impl ArtifactStorage for FsArtifactStorage {
    #[instrument(skip(self), fields(dataset_id, source_path))]
    fn save_artifact(
        &self,
        dataset_id: &str,
        source_path: &Path,
    ) -> Result<String, ArtifactStorageError> {
        debug!("base directory {:?}", &self.base_dir);
        fs::create_dir_all(&self.base_dir).unwrap();
        let storage_path = Path::join(&self.base_dir, format!("{dataset_id}.tif"));
        debug!("storage path composed {:?}", &storage_path);
        fs::copy(source_path, &storage_path).unwrap();

        Ok(storage_path.to_string_lossy().into_owned())
    }
}
