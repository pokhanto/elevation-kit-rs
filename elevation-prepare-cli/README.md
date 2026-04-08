# elevation-prepare-cli

CLI tool for preparing elevation datasets for use in `elevation-kit`.

It takes source GeoTIFF, optionally reprojects it, converts it to Cloud Optimized GeoTIFF (COG) when needed, stores prepared artifact either locally or in S3, and writes dataset metadata.

## What it does

- reads source raster dataset
- reprojects it to target CRS when needed
- converts it to COG when needed
- stores prepared artifact in local filesystem or S3
- writes metadata registry entry

## Requirements

- GDAL must be available when running locally
- or use provided Docker image
- when using S3 artifact storage, AWS credentials and region must be available in runtime environment

## Usage

### Local artifact storage

```bash
elevation-prepare-cli \
  --source-dataset-path /path/to/source.tif \
  --dataset-id datasetid \
  --base-dir /path/to/data \
  --registry-name registry \
  --artifact-backend fs
```

### S3 artifact storage

```bash
elevation-prepare-cli \
  --source-dataset-path /path/to/source.tif \
  --dataset-id datasetid \
  --base-dir /path/to/data \
  --registry-name registry \
  --artifact-backend s3 \
  --s3-bucket elevation-tiffs \
  --s3-prefix artifacts
```

## Docker

Build image from workspace root:

```bash
docker build -f elevation-prepare-cli/Dockerfile -t elevation-prepare-cli .
```

### Run with local artifact storage

```bash
docker run --rm \
  -v "$(pwd)/data_input:/input:ro" \
  -v "$(pwd)/data:/data" \
  elevation-prepare-cli \
  --source-dataset-path /input/sample.tif \
  --dataset-id my-dataset \
  --base-dir /data \
  --registry-name registry \
  --artifact-backend fs
```

### Run with S3 artifact storage

```bash
docker run --rm \
  -v "$(pwd)/data_input:/input:ro" \
  -v "$(pwd)/data:/data" \
  -e AWS_ACCESS_KEY_ID="$AWS_ACCESS_KEY_ID" \
  -e AWS_SECRET_ACCESS_KEY="$AWS_SECRET_ACCESS_KEY" \
  -e AWS_REGION="$AWS_REGION" \
  elevation-prepare-cli \
  --source-dataset-path /input/sample.tif \
  --dataset-id my-dataset \
  --base-dir /data \
  --registry-name registry \
  --artifact-backend s3 \
  --s3-bucket elevation-tiffs \
  --s3-prefix artifacts
```

## Notes

- `--artifact-backend fs` stores prepared artifacts in local base directory
- `--artifact-backend s3` stores prepared artifacts in specified S3 bucket
- metadata is still written to local metadata registry
- when source CRS already matches target CRS, reprojection is skipped
- when source dataset is already COG, conversion is skipped
