#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rejection {
    Unauthenticated,
    UnknownKey,
    DuplicateRequest,
    CurrencyMismatch,
    LimitExceeded,
    InsufficientBalance,
}
