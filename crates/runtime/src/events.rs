use kernel::model::{KeyId, Money, RequestId, UnixMillis};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UsageUnreadable {
    pub request: RequestId,
    pub key: KeyId,
    pub held: Money,
    pub at: UnixMillis,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettlementCapped {
    pub request: RequestId,
    pub key: KeyId,
    pub held: Money,
    pub actual: Money,
    pub at: UnixMillis,
}
