#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BillingRejection {
    InsufficientBalance,
    DuplicateRequest,
    LimitExceeded,
    CurrencyMismatch,
    HoldNotFound,
    SettlementExceedsHold,
    AlreadySettled,
    AlreadyReleased,
    Unbalanced,
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WalletError<E> {
    Rejected(BillingRejection),
    Storage(E),
}
