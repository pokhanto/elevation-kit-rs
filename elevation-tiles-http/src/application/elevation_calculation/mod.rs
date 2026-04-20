use georaster_domain::RasterValue;

mod mean;
pub use mean::MeanElevationCalculationStrategy;

pub trait ElevationCalculationStrategy {
    type State;

    fn key(&self) -> &'static str;

    fn new_state(&self) -> Self::State;

    fn update(&self, state: &mut Self::State, value: RasterValue);

    fn finalize(&self, state: Self::State) -> Option<RasterValue>;
}
