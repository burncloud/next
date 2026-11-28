use billing::errors::WalletError;

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

/// 对账失败。`Wallet` 是 `Wallet::settle` 或 `Wallet::release` 的原样结果，其中令牌不符为 `BillingRejection::HoldMismatch`，不结算、不释放。拒绝和存储失败都不改挂起名单。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReconcileError<E> {
    NotSuspended,
    WrongAction,
    AmountChanged,
    MissingPrice,
    ZeroPrice,
    Wallet(WalletError<E>),
}
