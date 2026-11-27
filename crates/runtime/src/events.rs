use kernel::model::{AccountId, KeyId, Money, RequestId, UnixMillis};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UsageUnreadable {
    pub request: RequestId,
    pub account: AccountId,
    pub key: KeyId,
    pub held: Money,
    pub at: UnixMillis,
}

/// `Overflow` 表示计价溢出，实际额超出 `u64`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActualCost {
    Priced(Money),
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettlementCapped {
    pub request: RequestId,
    pub account: AccountId,
    pub key: KeyId,
    pub held: Money,
    pub actual: ActualCost,
    pub at: UnixMillis,
}
