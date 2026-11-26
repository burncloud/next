#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rejection {
    Unauthenticated,
    UnknownKey,
    CurrencyMismatch,
    LimitExceeded,
    InsufficientBalance,
    UsageUnreadable,
}
