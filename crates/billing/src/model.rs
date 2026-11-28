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

/// 钱包为这一次冻结生成的归属令牌。每次成功的 `Wallet::hold` 返回一个新值。调用方只把它原样交还给 `settle` 和 `release`。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HoldId(pub String);

/// 一笔尚未结算、也尚未释放的冻结。`hold` 是 `Wallet::hold` 当时返回的 `HoldId`，不是 `key`。调用方把它原样交给 `settle` 或 `release`。`created_at` 是冻结写入时的时刻。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingHold {
    pub hold: HoldId,
    pub account: AccountId,
    pub request: RequestId,
    pub key: KeyId,
    pub amount: Money,
    pub created_at: UnixMillis,
}

/// 开户结果。同一账户同一币种再开一次是 `AlreadyOpen`，余额不变，也不写账本行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opened {
    Created,
    AlreadyOpen,
}

/// 入账结果。同一账户、同一 `CreditId`、同一金额再入一次是 `AlreadyApplied`，不再追加分录。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Credited {
    Applied,
    AlreadyApplied,
}
