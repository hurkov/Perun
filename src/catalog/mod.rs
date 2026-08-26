mod errors;
mod model;
mod persistence;
mod store;

pub use errors::{CatalogError, LookupError};
pub use model::SoundMeta;
pub use persistence::init;
pub use store::{Store, find, list, new_store, register, remove, rename};
