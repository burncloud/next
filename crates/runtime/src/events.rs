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

/// 实际费用比冻结额多出的部分。`Minor` 是同一币种多出的 `u64` 最小单位，`Overflow` 是计价溢出，`CurrencyMismatch` 是币种不同。不用浮点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shortfall {
    Minor(u64),
    Overflow,
    CurrencyMismatch,
}

/// 编排失败发生在哪一步。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Authenticate,
    Limit,
    Admit,
    Prices,
    Hold,
    Upstream,
    Settle,
    Release,
}

/// `detail` 是底层错误的 `Debug` 输出，不含客户 key 和请求内容。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fault {
    pub stage: Stage,
    pub request: Option<RequestId>,
    pub account: Option<AccountId>,
    pub attempts: u32,
    pub detail: String,
}

/// meter 已在 `feed` 中报错，且上游未正常结束。计费仍是释放、不扣费。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MeterFailed {
    pub request: RequestId,
    pub account: AccountId,
    pub key: KeyId,
    pub held: Money,
    pub at: UnixMillis,
}

/// 已决请求中读不出用量的比例从低于阈值升到达到阈值。分子和分母都是整数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UsageUnreadableRatio {
    pub decided: u64,
    pub unreadable: u64,
    pub window: u64,
    pub ratio_num: u64,
    pub ratio_den: u64,
    pub at: UnixMillis,
}
