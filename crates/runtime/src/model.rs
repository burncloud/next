#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reply<B> {
    pub status: u16,
    pub streaming: bool,
    pub body: B,
}
