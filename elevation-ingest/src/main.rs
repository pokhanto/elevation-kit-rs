use elevation_types::MetadataStorage;

fn ingest(metadata_storage: impl MetadataStorage) {
    metadata_storage.save_metadata();
}

fn main() {
    println!("Hello, world!");
}
