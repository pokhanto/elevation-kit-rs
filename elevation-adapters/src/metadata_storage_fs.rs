use std::{
    fs::{File, OpenOptions},
    io::BufWriter,
    path::{Path, PathBuf},
};

use elevation_types::{DatasetMetadata, MetadataStorage, MetadataStorageError};
use serde::{Deserialize, Serialize};

pub struct FsMetadataStorage {
    base_dir: PathBuf,
}

impl FsMetadataStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FsMetadataRegistry {
    metadata: Vec<DatasetMetadata>,
}

const METADATA_FILE_NAME: &str = "registry.json";

impl MetadataStorage for FsMetadataStorage {
    #[tracing::instrument(skip(self), fields(base_dir = %self.base_dir.display()))]
    fn save_metadata(&self, metadata_to_save: DatasetMetadata) -> Result<(), MetadataStorageError> {
        std::fs::create_dir_all(&self.base_dir).map_err(|err| {
            tracing::debug!(error = %err, base_dir = %self.base_dir.display(), "failed to create metadata storage directory");

            MetadataStorageError::PrepareStorage
        })?;

        let metadata_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(Path::new(&self.base_dir).join(METADATA_FILE_NAME))
            .map_err(|err| {
                tracing::debug!(
                    error = %err,
                    base_dir = %self.base_dir.display(),
                    file_name = %METADATA_FILE_NAME,
                    "failed to open metadata file"
                );
                MetadataStorageError::PrepareStorage
            })?;

        let mut registry: FsMetadataRegistry = match serde_json::from_reader(&metadata_file) {
            Ok(registry) => registry,
            // if file just created - create empty registry
            Err(err) if err.is_eof() => FsMetadataRegistry { metadata: vec![] },
            Err(err) => {
                tracing::debug!(
                    error = %err,
                    "failed to deserialize metadata registry"
                );
                return Err(MetadataStorageError::PrepareStorage);
            }
        };
        tracing::debug!(registry = ?registry, "registry resolved");

        if registry
            .metadata
            .iter()
            .any(|m| m.dataset_id == metadata_to_save.dataset_id)
        {
            return Err(MetadataStorageError::DuplicateId);
        }
        registry.metadata.push(metadata_to_save);

        let metadata_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(Path::new(&self.base_dir).join(METADATA_FILE_NAME))
            .map_err(|err| {
                tracing::debug!(
                    error = %err,
                    base_dir = %self.base_dir.display(),
                    file_name = %METADATA_FILE_NAME,
                    "failed to open metadata file to write metadata"
                );
                MetadataStorageError::PrepareStorage
            })?;

        let writer = BufWriter::new(&metadata_file);
        serde_json::to_writer_pretty(writer, &registry).map_err(|err| {
            tracing::debug!(
                error = %err,
                base_dir = %self.base_dir.display(),
                file_name = %METADATA_FILE_NAME,
                "failed to save metadata file"
            );
            MetadataStorageError::Save
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(base_dir = %self.base_dir.display()))]
    fn load_metadata(&self) -> Result<Vec<DatasetMetadata>, MetadataStorageError> {
        let metadata_path = Path::new(&self.base_dir).join(METADATA_FILE_NAME);

        let metadata_file = File::open(&metadata_path).map_err(|err| {
            tracing::debug!(
                error = %err,
                path = %metadata_path.display(),
                "failed to open metadata file"
            );
            MetadataStorageError::Load
        })?;

        let registry: FsMetadataRegistry =
            serde_json::from_reader(metadata_file).map_err(|err| {
                tracing::debug!(
                    error = %err,
                    path = %metadata_path.display(),
                    "failed to deserialize metadata file"
                );
                MetadataStorageError::Load
            })?;

        Ok(registry.metadata)
    }
}
