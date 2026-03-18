use std::path::PathBuf;

use clap::Parser;
use elevation_core::ElevationService;
// TODO: import from root
use elevation_core::raster_reader::GdalRasterReader;
use elevation_metadata_fs::FsMetadataStorage;

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
