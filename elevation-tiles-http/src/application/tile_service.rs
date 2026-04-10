//! Tile service for resolving H3 tiles and their aggregated elevations.

use elevation_domain::{BboxElevations, Bounds, Elevation, ResolutionHint};
use futures::Stream;
use geo::{BoundingRect, Contains, Intersects, Point, Polygon};
use h3o::{
    Resolution,
    geom::{ContainmentMode, TilerBuilder},
};
use moka::future::Cache;
use std::{collections::HashMap, str::FromStr};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    application::{
        elevation_accumulator::MeanElevationAccumulator, elevation_provider::ElevationProvider,
    },
    domain::{ElevationTile, Tile},
};

/// Controls split of big requested bounding box to smaller chunks.
const MAX_CELLS_PER_CHUNK: usize = 25000;

// TODO: extract this mapping into config/service.
// Or - rework to not rely on specific resolution
const RESOLUTION_HINT: ResolutionHint = ResolutionHint::Degrees {
    lon_resolution: 0.005,
    lat_resolution: 0.005,
};

/// Errors returned by [`TileService`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TileServiceError {
    #[error("Incorrect zoom level")]
    ZoomLevel,
    #[error("Can't build tiles for given bounding box")]
    BuildTiles,
    #[error("Unknown tile")]
    UnknownTile,
    #[error("Can't get elevation")]
    Elevation,
    #[error("Chunking requires explicit degree resolution")]
    ChunkResolution,
}

/// Resolves tiles and caches computed results.
#[derive(Clone, Debug)]
pub struct TileService<EP> {
    elevation_provider: EP,
    // moka Cache cloning is cheap.
    cache: Cache<String, ElevationTile>,
    max_cells_per_chunk: usize,
}

impl<EP> TileService<EP>
where
    EP: ElevationProvider,
{
    /// Creates tile service with in-memory cache.
    pub fn new(elevation_provider: EP, cache_max_capacity: u64) -> Self {
        let cache = Cache::builder().max_capacity(cache_max_capacity).build();

        Self {
            elevation_provider,
            cache,
            max_cells_per_chunk: MAX_CELLS_PER_CHUNK,
        }
    }

    /// Returns tile by id, using cache when possible.
    #[tracing::instrument(skip(self), fields(tile_id = %tile_id))]
    pub async fn get_tile_by_id(&self, tile_id: String) -> Result<ElevationTile, TileServiceError> {
        if let Some(tile) = self.cache.get(&tile_id).await {
            tracing::debug!(tile_id, "tile cache hit");
            return Ok(tile);
        }

        tracing::debug!(tile_id, "tile cache miss");

        let tile = Tile::from_str(&tile_id).map_err(|err| {
            tracing::debug!(error = ?err, tile_id, "failed to parse tile id as h3 cell");
            TileServiceError::UnknownTile
        })?;

        let tile_bounding_rect = tile.bounding_rect().ok_or_else(|| {
            tracing::debug!("failed to compute bounding rect from h3 cell boundary");
            TileServiceError::UnknownTile
        })?;

        let elevations = self
            .elevation_provider
            .elevations_in_bbox(tile_bounding_rect.into(), Some(ResolutionHint::Highest))
            .await
            .map_err(|err| {
                tracing::debug!(error = ?err, "failed to get elevations for tile bbox");
                TileServiceError::Elevation
            })?;

        let mut mean_elevation = MeanElevationAccumulator::new();
        mean_elevation.extend(elevations.values);

        let elevation_tile = ElevationTile::new(tile_id, mean_elevation.mean());

        self.cache
            .insert(elevation_tile.id().to_owned(), elevation_tile.clone())
            .await;

        Ok(elevation_tile)
    }
}

impl<EP> TileService<EP>
where
    EP: ElevationProvider + Clone + Send + Sync + 'static,
{
    /// Streams tiles for requested bounding box.
    ///
    /// To avoid loading elevations for whole bounding box at once service
    /// splits requested area into smaller chunks and processes them separately.
    ///
    /// Because H3 tiles may cross chunk boundaries tile aggregation state is kept
    /// across chunk processing. Tiles updated from every chunk they intersect
    /// and emitted only after all relevant chunks is processed.
    #[tracing::instrument(
        skip(self),
        fields(
            zoom_level,
            min_lon = bbox.min_lon(),
            min_lat = bbox.min_lat(),
            max_lon = bbox.max_lon(),
            max_lat = bbox.max_lat(),
        )
    )]
    pub fn stream_tiles_for_bbox(
        &self,
        bbox: Bounds,
        zoom_level: u8,
    ) -> Result<
        impl Stream<Item = Result<ElevationTile, TileServiceError>> + Send + 'static + use<EP>,
        TileServiceError,
    > {
        // resolve all H3 tiles covering requested bbox
        let tile_ids = get_tile_ids_for_bbox(bbox, zoom_level)?;

        // split requested bbox in chunks to avoid loading whole area at once
        let chunks = split_bbox_into_chunks(bbox, RESOLUTION_HINT, self.max_cells_per_chunk)?;

        tracing::info!(chunks_count = chunks.len(), "got bbox chunks count");

        let elevation_provider = self.elevation_provider.clone();
        let cache = self.cache.clone();

        // assign stable ids to chunks so we can track which tiles depend on which chunks
        let chunk_infos = chunks
            .into_iter()
            .enumerate()
            .map(|(id, bounds)| ChunkInfo { id, bounds })
            .collect::<Vec<_>>();

        // initialize aggregation state for every requested tile
        // this needs because some tiles will fall to chunk edges
        // and this will require to update them
        let mut tile_states = tile_ids
            .into_iter()
            .map(|tile_id| {
                let tile = Tile::from_str(&tile_id).map_err(|err| {
                    tracing::debug!(error = ?err, tile_id, "failed to parse tile id as h3 cell");
                    TileServiceError::UnknownTile
                })?;

                let polygon = tile.polygon();

                let bounds = polygon
                    .bounding_rect()
                    .ok_or(TileServiceError::BuildTiles)?;

                Ok((
                    tile_id,
                    TileAggregationState {
                        polygon,
                        bounds: bounds.into(),
                        mean_elevation_accumulator: MeanElevationAccumulator::new(),
                        remaining_chunks: 0,
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>, TileServiceError>>()?;

        // for each chunk calculate which tiles may be affected by it
        // also count how many chunks each tile must wait for before it is complete
        let mut chunk_to_tile_ids: HashMap<usize, Vec<String>> = HashMap::new();

        for chunk in &chunk_infos {
            let chunk_rect: geo::Rect<f64> = chunk.bounds.into();

            for (tile_id, state) in &mut tile_states {
                let tile_rect: geo::Rect<f64> = state.bounds.into();

                if chunk_rect.intersects(&tile_rect) {
                    chunk_to_tile_ids
                        .entry(chunk.id)
                        .or_default()
                        .push(tile_id.clone());
                    state.remaining_chunks += 1;
                }
            }
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ElevationTile, TileServiceError>>(128);

        tokio::spawn(async move {
            let mut tile_states = tile_states;

            // process chunks one by one and update only tiles that intersect current chunk
            for chunk in chunk_infos {
                let affected_tile_ids = chunk_to_tile_ids
                    .get(&chunk.id)
                    .cloned()
                    .unwrap_or_default();

                if affected_tile_ids.is_empty() {
                    continue;
                }

                // fetch elevations for current chunk
                let chunk_elevations = match elevation_provider
                    .elevations_in_bbox(chunk.bounds, Some(RESOLUTION_HINT))
                    .await
                {
                    Ok(v) => v,
                    Err(err) => {
                        tracing::error!(
                            error = ?err,
                            chunk_id = chunk.id,
                            ?chunk.bounds,
                            "failed to get elevations for chunk"
                        );
                        let _ = tx.send(Err(TileServiceError::Elevation)).await;
                        return;
                    }
                };

                // update aggregation state only for tiles selected for this chunk
                update_selected_tile_states_from_chunk(
                    &chunk_elevations,
                    affected_tile_ids.iter().map(String::as_str),
                    &mut tile_states,
                );

                tracing::info!(
                    chunk_id = chunk.id,
                    affected_tiles = affected_tile_ids.len(),
                    "processed chunk"
                );

                let mut completed_tile_ids = Vec::new();

                // decrease remaining chunk counters and detect tiles that are now fully calculated
                for tile_id in affected_tile_ids {
                    let Some(state) = tile_states.get_mut(&tile_id) else {
                        continue;
                    };

                    if state.remaining_chunks > 0 {
                        state.remaining_chunks -= 1;
                    }

                    if state.remaining_chunks == 0 {
                        completed_tile_ids.push(tile_id);
                    }
                }

                // build final tiles for completed ids, cache them, and emit into stream
                for tile_id in completed_tile_ids {
                    let Some(state) = tile_states.remove(&tile_id) else {
                        continue;
                    };

                    let tile = ElevationTile::new(tile_id.clone(), state.mean());

                    cache.insert(tile.id().to_owned(), tile.clone()).await;

                    if tx.send(Ok(tile)).await.is_err() {
                        tracing::debug!("client disconnected while streaming completed tiles");
                        return;
                    }
                }

                // yield between chunks
                tokio::task::yield_now().await;
            }
        });

        Ok(ReceiverStream::new(rx))
    }
}

#[derive(Debug, Clone, Copy)]
struct ChunkInfo {
    id: usize,
    bounds: Bounds,
}

#[derive(Debug, Clone)]
struct TileAggregationState {
    polygon: Polygon<f64>,
    bounds: Bounds,
    mean_elevation_accumulator: MeanElevationAccumulator,
    remaining_chunks: usize,
}

impl TileAggregationState {
    fn add(&mut self, value: Elevation) {
        self.mean_elevation_accumulator.add(value);
    }

    fn mean(&self) -> Option<Elevation> {
        self.mean_elevation_accumulator.mean()
    }
}

pub fn get_tile_ids_for_bbox(
    bbox: Bounds,
    zoom_level: u8,
) -> Result<Vec<String>, TileServiceError> {
    let resolution: Resolution = zoom_level.try_into().map_err(|err| {
        tracing::debug!(error = ?err, "invalid zoom level for h3 resolution");
        TileServiceError::ZoomLevel
    })?;

    let mut tiler = TilerBuilder::new(resolution)
        .containment_mode(ContainmentMode::Covers)
        .build();

    tiler.add(bbox.into()).map_err(|err| {
        tracing::debug!(error = ?err, "failed to add bbox to tiler");
        TileServiceError::BuildTiles
    })?;

    let tile_ids = tiler
        .into_coverage()
        .map(|tile| tile.to_string())
        .collect::<Vec<_>>();

    tracing::info!(tile_count = tile_ids.len(), "resolved tile ids for bbox");

    Ok(tile_ids)
}

/// Updates only selected tiles from one chunk.
fn update_selected_tile_states_from_chunk<'a>(
    elevations: &BboxElevations,
    tile_ids: impl IntoIterator<Item = &'a str>,
    tile_states: &mut HashMap<String, TileAggregationState>,
) {
    if elevations.width == 0 || elevations.height == 0 {
        return;
    }

    // geographic size of one cell of grid in returned elevation grid
    let lon_step =
        (elevations.bbox.max_lon() - elevations.bbox.min_lon()) / elevations.width as f64;
    let lat_step =
        (elevations.bbox.max_lat() - elevations.bbox.min_lat()) / elevations.height as f64;

    // update only tiles that affected by this chunk
    for tile_id in tile_ids {
        let Some(state) = tile_states.get_mut(tile_id) else {
            continue;
        };

        // find overlap between tile bounds and current chunk bbox
        // if there is no overlap, this chunk cannot contribute to tile
        let overlap = match state.bounds.intersection(&elevations.bbox) {
            Some(v) => v,
            None => continue,
        };

        // map overlap bbox into column range of chunk elevation grid
        let start_col = (((overlap.min_lon() - elevations.bbox.min_lon()) / lon_step).floor()
            as isize)
            .max(0) as usize;
        let end_col_exclusive = (((overlap.max_lon() - elevations.bbox.min_lon()) / lon_step).ceil()
            as isize)
            .min(elevations.width as isize)
            .max(0) as usize;

        // map overlap bbox into row range of chunk elevation grid
        let start_row = (((elevations.bbox.max_lat() - overlap.max_lat()) / lat_step).floor()
            as isize)
            .max(0) as usize;
        let end_row_exclusive = (((elevations.bbox.max_lat() - overlap.min_lat()) / lat_step).ceil()
            as isize)
            .min(elevations.height as isize)
            .max(0) as usize;

        // skip empty target window
        if start_col >= end_col_exclusive || start_row >= end_row_exclusive {
            continue;
        }

        for row in start_row..end_row_exclusive {
            // compute latitude of cell center for current row
            let lat = elevations.bbox.max_lat() - (row as f64 + 0.5) * lat_step;

            for col in start_col..end_col_exclusive {
                let idx = row * elevations.width + col;
                let Some(value) = elevations.values[idx] else {
                    continue;
                };

                // compute longitude of cell center for current column
                let lon = elevations.bbox.min_lon() + (col as f64 + 0.5) * lon_step;

                // cheap rectangular prefilter before exact polygon check
                if !state.bounds.contains_point(lon, lat) {
                    continue;
                }

                let point = Point::new(lon, lat);

                // exact containment check against H3 tile polygon
                if state.polygon.contains(&point) {
                    state.add(value);
                }
            }
        }
    }
}

/// Splits requested bbox in chunks.
fn split_bbox_into_chunks(
    bbox: Bounds,
    resolution_hint: ResolutionHint,
    max_cells_per_chunk: usize,
) -> Result<Vec<Bounds>, TileServiceError> {
    let (lon_resolution, lat_resolution) = match resolution_hint {
        ResolutionHint::Degrees {
            lon_resolution,
            lat_resolution,
        } => (lon_resolution, lat_resolution),
        _ => return Err(TileServiceError::ChunkResolution),
    };

    // compute full raster grid size for requested bbox at target resolution
    let full_width = ((bbox.max_lon() - bbox.min_lon()) / lon_resolution).ceil() as usize;
    let full_height = ((bbox.max_lat() - bbox.min_lat()) / lat_resolution).ceil() as usize;

    if full_width == 0 || full_height == 0 {
        return Ok(Vec::new());
    }

    // if requested bbox already fits into one chunk - return it as is
    if full_width * full_height <= max_cells_per_chunk {
        return Ok(vec![bbox]);
    }

    // approximate square chunk size in raster cells
    let chunk_side = (max_cells_per_chunk as f64).sqrt().floor() as usize;
    let chunk_width = chunk_side.max(1).min(full_width);
    let chunk_height = chunk_side.max(1).min(full_height);

    // compute geographic size of one output cell
    let lon_step = (bbox.max_lon() - bbox.min_lon()) / full_width as f64;
    let lat_step = (bbox.max_lat() - bbox.min_lat()) / full_height as f64;

    let mut chunks = Vec::new();

    let mut start_row = 0;
    while start_row < full_height {
        let end_row = (start_row + chunk_height).min(full_height);

        let mut start_col = 0;
        while start_col < full_width {
            let end_col = (start_col + chunk_width).min(full_width);

            // map chunk grid window back into geographic bounds
            let min_lon = bbox.min_lon() + start_col as f64 * lon_step;
            let max_lon = bbox.min_lon() + end_col as f64 * lon_step;
            let max_lat = bbox.max_lat() - start_row as f64 * lat_step;
            let min_lat = bbox.max_lat() - end_row as f64 * lat_step;

            let chunk = Bounds::try_new(min_lon, min_lat, max_lon, max_lat)
                .map_err(|_| TileServiceError::BuildTiles)?;

            chunks.push(chunk);
            start_col = end_col;
        }

        start_row = end_row;
    }

    Ok(chunks)
}
