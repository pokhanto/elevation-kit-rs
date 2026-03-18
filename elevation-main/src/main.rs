use clap::Parser;
use elevation_core::ElevationService;
use elevation_core::raster_reader::GdalRasterReader;
use elevation_ingest::{FsArtifactStorage, FsMetadataStorage, ingest};
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    dataset_id: String,
}

fn main() {
    let args = Args::parse();
    let Args { source, dataset_id } = args;
    let metadata_storage = FsMetadataStorage {};
    let artifact_storage = FsArtifactStorage {};

    ingest(dataset_id, source, artifact_storage, metadata_storage);

    let metadata_storage = FsMetadataStorage {};
    let raster_reader = GdalRasterReader;

    let service = ElevationService::new(metadata_storage, raster_reader);

    let elevation = service.elevation_at(36.2304, 49.9935);
    println!("elev {:?}", elevation);
}
