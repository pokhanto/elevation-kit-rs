use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use clap::{Parser, ValueEnum};
use elevation_adapters::{FsArtifactStorage, FsMetadataStorage, S3ArtifactStorage};
use elevation_core::IngestService;
use elevation_domain::Crs;
use std::path::PathBuf;

mod telemetry;

// TODO: atm this is only supported CRS,
// so every dataset will be translated to it
const CRS: &str = "EPSG:4326";

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ArtifactBackend {
    Fs,
    S3,
}

/// Ingest source DEM dataset into artifact and metadata storage.
#[derive(Debug, Parser)]
#[command(
    name = "elevation-prepare",
    version,
    about = "Ingests source elevation dataset into base directory.",
    long_about = "Reads source elevation dataset, prepares artifacts, and stores metadata \
about ingested dataset in base directory.",
    next_line_help = true
)]
struct Args {
    /// Path to source dataset file to ingest.
    #[arg(long, value_name = "FILE")]
    source_dataset_path: PathBuf,

    /// Identifier for dataset being ingested.
    #[arg(long, value_name = "DATASET_ID")]
    dataset_id: String,

    /// Base directory for metadata and generated artifacts.
    ///
    /// Used for local metadata storage and local filesystem artifact storage.
    #[arg(long, value_name = "DIR")]
    base_dir: PathBuf,

    /// Name of metadata storage file.
    #[arg(long, value_name = "REGISTRY_NAME")]
    registry_name: String,

    /// Artifact storage backend.
    #[arg(long, value_enum, default_value_t = ArtifactBackend::Fs)]
    artifact_backend: ArtifactBackend,

    /// S3 bucket for artifact storage.
    ///
    /// Required when --artifact-backend s3 is used.
    #[arg(long, value_name = "BUCKET")]
    s3_bucket: Option<String>,

    /// Optional S3 key prefix for stored artifacts.
    #[arg(long, value_name = "PREFIX")]
    s3_prefix: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    telemetry::init_tracing();

    let args = Args::parse();

    let metadata_storage = FsMetadataStorage::new(args.base_dir.clone(), args.registry_name);

    match args.artifact_backend {
        ArtifactBackend::Fs => {
            let artifact_storage = FsArtifactStorage::new(args.base_dir);
            let ingest_service =
                IngestService::new(Crs::new(CRS), artifact_storage, metadata_storage);

            ingest_service
                .run(args.dataset_id, args.source_dataset_path)
                .await?;
        }
        ArtifactBackend::S3 => {
            let bucket = args
                .s3_bucket
                .ok_or("--s3-bucket is required when --artifact-backend s3 is used")?;

            let aws_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
            let s3_client = Client::new(&aws_config);

            let artifact_storage = S3ArtifactStorage::new(s3_client, bucket, args.s3_prefix);

            let ingest_service =
                IngestService::new(Crs::new(CRS), artifact_storage, metadata_storage);

            ingest_service
                .run(args.dataset_id, args.source_dataset_path)
                .await?;
        }
    };

    Ok(())
}
