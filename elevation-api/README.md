# elevation-api

`elevation-api` is small HTTP service for querying elevations from GeoTIFF dataset.

Service will do on startup:

- ingests input GeoTIFF
- stores prepared metadata and raster artifact locally
- starts HTTP API for elevation queries

Service is intended to run in Docker because it depends on **GDAL**.

## API

### Query elevations

```bash
curl -i -X POST http://localhost:8080/elevations \
  -H "Content-Type: application/json" \
  -d '[
    { "lat": 42.5, "lon": -111.5 },
    { "lat": 46.2, "lon": -107.2 }
  ]'
```

## Build Docker image

`elevation-api` is part of Rust workspace and depends on shared crates from that workspace.
Because of that, Docker image should be built from workspace root so all required crates are included in the build context.

Example:

```
docker build -f elevation-api/Dockerfile -t elevation-api .
```

## Running with Docker Compose

Example service definition:

```
services:
  elevation-api:
    image: elevation-api:latest
    ports:
      - "8080:8080"
    env_file:
      - .env
    environment:
      FILE_TO_INGEST: ${FILE_TO_INGEST}
    volumes:
      - ../input:/app/input:ro
      - ../data:/app/data
```

And run with:

```
FILE_TO_INGEST=/app/input/input.tif docker compose up
```

### Volume mapping

../input:/app/input:ro - service input GeoTIFF directory, mounted read-only
../data:/app/data - local storage for prepared data and metadata

### Configuration

Copy .env.example to .env and adjust values.

- METADATA_REGISTRY_NAME - name of metadata registry file
- FILE_TO_INGEST - path to input GeoTIFF that will be ingested on startup
- STORAGE_DIR - directory where prepared raster artifact and metadata are stored
- APP_HOST - host address for HTTP server
- APP_PORT - port for HTTP server

### Notes

- dataset ingestion happens on application startup
- GDAL is required, which is why Docker is recommended way to run service
- CRS is fixed to EPSG:4326
