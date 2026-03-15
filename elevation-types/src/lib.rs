pub struct Elevation(f64);

pub trait MetadataStorage {
    fn save_metadata(&self);

    fn get_metadata(&self);
}
