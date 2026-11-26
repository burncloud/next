#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rejection {
    Unauthenticated,
    UnknownKey,
    RateLimited,
    DuplicateRequest,
    CurrencyMismatch,
    LimitExceeded,
    InsufficientBalance,
}
