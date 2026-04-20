use georaster_domain::Bounds;

const DEFAULT_PREVIEW_SAMPLING: GeorasterSampling = GeorasterSampling::FitWithin {
    max_width: 512,
    max_height: 512,
};

const DEFAULT_DETAILED_SAMPLING: GeorasterSampling = GeorasterSampling::FitWithin {
    max_width: 2048,
    max_height: 2048,
};

/// Describes how service should choose output grid size for raster query.
///
/// Raster queries are defined over geographic bounding box, but callers may
/// want to control shape of returned grid in different ways:
///
/// - by providing exact output width and height
/// - by specifying spatial resolution in query CRS units
/// - by providing maximum output dimensions and letting service preserve
///   aspect ratio
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeorasterSampling {
    /// Uses a service-defined preview sampling policy.
    ///
    /// Intended for fast, bounded, display-friendly queries.
    Preview,

    /// Uses a service-defined detailed sampling policy.
    ///
    /// Intended for higher-fidelity queries while still keeping output size practical.
    Detailed,

    /// Uses an exact output grid size.
    OutputSize { width: usize, height: usize },

    /// Uses an explicit spatial resolution.
    Resolution {
        x_resolution: f64,
        y_resolution: f64,
    },

    /// Fits the output grid within the provided maximum dimensions
    /// while preserving aspect ratio.
    FitWithin { max_width: usize, max_height: usize },
}

impl GeorasterSampling {
    pub fn bbox_dimensions(&self, bbox: &Bounds) -> (usize, usize) {
        match self {
            GeorasterSampling::Preview => DEFAULT_PREVIEW_SAMPLING.bbox_dimensions(bbox),
            GeorasterSampling::Detailed => DEFAULT_DETAILED_SAMPLING.bbox_dimensions(bbox),
            GeorasterSampling::OutputSize { width, height } => (*width, *height),
            GeorasterSampling::Resolution {
                x_resolution,
                y_resolution,
            } => {
                let width = ((bbox.max_lon() - bbox.min_lon()) / x_resolution)
                    .ceil()
                    .max(1.0) as usize;
                let height = ((bbox.max_lat() - bbox.min_lat()) / y_resolution)
                    .ceil()
                    .max(1.0) as usize;
                (width, height)
            }
            GeorasterSampling::FitWithin {
                max_width,
                max_height,
            } => fit_dimensions(*max_width, *max_height, bbox),
        }
    }
}

fn fit_dimensions(max_width: usize, max_height: usize, bbox: &Bounds) -> (usize, usize) {
    let bbox_width = bbox.max_lon() - bbox.min_lon();
    let bbox_height = bbox.max_lat() - bbox.min_lat();

    if bbox_width <= 0.0 || bbox_height <= 0.0 {
        return (1, 1);
    }

    let width_ratio = max_width as f64 / bbox_width;
    let height_ratio = max_height as f64 / bbox_height;
    let scale = width_ratio.min(height_ratio);

    let width = (bbox_width * scale).floor().max(1.0) as usize;
    let height = (bbox_height * scale).floor().max(1.0) as usize;

    (width, height)
}
