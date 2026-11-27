pub const MINOR_UNITS_PER_MAJOR: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Currency {
    Usd,
    Cny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Money {
    pub currency: Currency,
    pub minor_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

/// 按原样精确匹配，区分大小写，不做前缀或别名回退。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnixMillis(pub u64);
