use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use elevation_types::{DatasetMetadata, MetadataStorage};
use serde::{Deserialize, Serialize};

pub struct FsMetadataStorage {
    base_dir: PathBuf,
}

impl FsMetadataStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

#[derive(Serialize, Deserialize)]
struct FsMetadataRegistry {
    metadata: Vec<DatasetMetadata>,
}

const METADATA_FILE: &str = "registry.json";

// TODO: rework to Result
impl MetadataStorage for FsMetadataStorage {
    fn save_metadata(&self, metadata: DatasetMetadata) {
        std::fs::create_dir_all(&self.base_dir).unwrap();

        let metadata_file = File::create(Path::new(&self.base_dir).join(METADATA_FILE)).unwrap();
        let writer = BufWriter::new(&metadata_file);

        let mut registry: FsMetadataRegistry = match serde_json::from_reader(&metadata_file) {
            Ok(registry) => registry,
            Err(_) => FsMetadataRegistry { metadata: vec![] },
        };

        registry.metadata.push(metadata);

        serde_json::to_writer_pretty(writer, &registry).unwrap();
    }

    // TODO: must return result
    fn load_metadata(&self) -> Vec<DatasetMetadata> {
        let metadata_file = File::open(Path::new(&self.base_dir).join(METADATA_FILE)).unwrap();

        match serde_json::from_reader::<&File, FsMetadataRegistry>(&metadata_file) {
            Ok(registry) => registry.metadata,
            Err(_) => vec![],
        }
    }
}
