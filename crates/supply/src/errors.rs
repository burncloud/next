#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatalogRejection {
    UnknownModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CatalogError<E> {
    Rejected(CatalogRejection),
    Storage(E),
}
