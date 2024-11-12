#[derive(Clone)]
pub struct AppStateGeneric<T, U> {
    pub pool: T,
    pub cache: U,
}