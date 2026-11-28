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
    HoldMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WalletError<E> {
    Rejected(BillingRejection),
    Storage(E),
}

/// 开户或入账被拒绝的原因。接受哪些币种不在这里规定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FundingRejection {
    UnsupportedCurrency,
    EmptyCreditId,
    ZeroAmount,
    NotOpened,
    ConflictingCredit,
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FundingError<E> {
    Rejected(FundingRejection),
    Storage(E),
}
