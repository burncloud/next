use kernel::model::{AccountId, KeyId, Money};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitPeriod {
    Lifetime,
    Daily,
    Monthly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpendLimit {
    pub amount: Money,
    pub period: LimitPeriod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestsPerMinute(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyLimit {
    pub spend: Option<SpendLimit>,
    pub requests_per_minute: Option<RequestsPerMinute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Principal {
    pub account: AccountId,
    pub key: KeyId,
}
