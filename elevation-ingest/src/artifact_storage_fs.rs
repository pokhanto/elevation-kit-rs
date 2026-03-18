use elevation_types::ArtifactStorage;
use std::path::Path;

pub struct FsArtifactStorage {}

impl ArtifactStorage for FsArtifactStorage {
    fn save_artifact(&self, _dataset_id: &str, path: &Path) -> String {
        let storage_path = String::from("./data/cog.tif");
        std::fs::copy(path, &storage_path).unwrap();

        storage_path
    }
}
