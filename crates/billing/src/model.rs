use kernel::model::{AccountId, CreditId, KeyId, ModelId, Money, RequestId, UnixMillis};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenUsage {
    /// 不含缓存命中的输入 token。
    pub input_tokens: u64,
    /// 命中缓存的输入 token。
    pub cached_input_tokens: u64,
    /// 含推理 token。
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelUsage {
    pub model: ModelId,
    pub tokens: TokenUsage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Debit,
    Credit,
}

/// `Revenue` 和 `Funding` 不带账户，表示平台外部的对手方。一笔入账写两条金额相等的分录：借 `Funding`、贷 `Available(account)`，`reference` 都是同一个 `EntryRef::Credit`，`key` 为 `None`。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LedgerAccount {
    Available(AccountId),
    Held(AccountId),
    Revenue,
    Funding,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryRef {
    Request(RequestId),
    Credit(CreditId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entry {
    pub reference: EntryRef,
    pub account: LedgerAccount,
    pub key: Option<KeyId>,
    pub direction: Direction,
    pub amount: Money,
    pub at: UnixMillis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCap {
    pub since: UnixMillis,
    pub limit: Money,
}
