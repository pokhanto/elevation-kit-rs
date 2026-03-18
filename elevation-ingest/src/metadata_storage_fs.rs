use std::{fs::File, io::BufWriter, path::Path};

use elevation_types::{DatasetMetadata, MetadataStorage};
use serde::{Deserialize, Serialize};

pub struct FsMetadataStorage {}

#[derive(Serialize, Deserialize)]
struct FsMetadataRegistry {
    metadata: Vec<DatasetMetadata>,
}

const METADATA_DIR: &str = "./data/metadata/";
const METADATA_FILE: &str = "registry.json";

// TODO: rework to Result
impl MetadataStorage for FsMetadataStorage {
    fn save_metadata(&self, metadata: DatasetMetadata) {
        std::fs::create_dir_all(METADATA_DIR).unwrap();

        let metadata_file = File::create(Path::new(METADATA_DIR).join(METADATA_FILE)).unwrap();
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
        let metadata_file = File::open(Path::new(METADATA_DIR).join(METADATA_FILE)).unwrap();

        match serde_json::from_reader::<&File, FsMetadataRegistry>(&metadata_file) {
            Ok(registry) => registry.metadata,
            Err(_) => vec![],
        }
    }
}
