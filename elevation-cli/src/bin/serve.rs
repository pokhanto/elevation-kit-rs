use std::path::PathBuf;

use clap::Parser;
use elevation_adapters::{FsMetadataStorage, GdalRasterReader};
use elevation_core::ElevationService;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    x: f64,
    #[arg(long)]
    y: f64,
    #[arg(long)]
    base_dir: PathBuf,
}

fn main() {
    let args = Args::parse();
    let Args { x, y, base_dir } = args;

    let metadata_storage = FsMetadataStorage::new(base_dir);
    let raster_reader = GdalRasterReader;

    let service = ElevationService::new(metadata_storage, raster_reader);

    let elevation = service.elevation_at(x, y);
    println!("elev {:?}", elevation);
}
