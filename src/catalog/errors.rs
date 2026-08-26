#[derive(Debug)]
pub enum CatalogError {
    DuplicateTitle,
    NotFound,
    SaveFailed,
}

#[derive(Debug)]
pub enum LookupError {
    InvalidSelector,
    NotFound,
}
