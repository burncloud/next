use kernel::model::{AccountId, KeyId, Money, RequestId, UnixMillis};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Debit,
    Credit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LedgerAccount {
    Available(AccountId),
    Held(AccountId),
    Revenue,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entry {
    pub request: RequestId,
    pub account: LedgerAccount,
    pub key: KeyId,
    pub direction: Direction,
    pub amount: Money,
    pub at: UnixMillis,
}
