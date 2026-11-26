use kernel::model::{AccountId, Currency, KeyId, Money, RequestId, UnixMillis};

use crate::model::Entry;

pub trait Pricing {
    type Usage;
    type Error;

    /// # Errors
    ///
    /// 用量无法计价时返回 `Self::Error`。
    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error>;
}

pub trait Wallet {
    type Error;

    /// # Errors
    ///
    /// 读不到该账号该币种的余额时返回 `Self::Error`。
    fn balance(&self, account: &AccountId, currency: Currency) -> Result<Money, Self::Error>;

    /// 密钥限额的币种和这笔冻结的币种不一致时拒绝，不换算。`Daily` 和 `Monthly` 的周期起点是 UTC 自然日、自然月；`since` 由调用方按 UTC 计算。
    ///
    /// # Errors
    ///
    /// 可用余额不足，或同一个 `request` 已经冻结过时，返回 `Self::Error`。
    fn hold(
        &self,
        request: &RequestId,
        account: &AccountId,
        key: &KeyId,
        amount: Money,
    ) -> Result<(), Self::Error>;

    /// 按实际金额结算这笔冻结，结算后冻结结束，没用完的部分退回可用余额。
    ///
    /// # Errors
    ///
    /// 没有这个 `request` 的冻结，或金额超过冻结额时，返回 `Self::Error`。同一个 `request` 再结算一次也返回 `Self::Error`，不能扣第二次。
    fn settle(&self, request: &RequestId, amount: Money) -> Result<(), Self::Error>;

    /// # Errors
    ///
    /// 没有这个 `request` 的冻结时返回 `Self::Error`。
    fn release(&self, request: &RequestId) -> Result<(), Self::Error>;

    /// # Errors
    ///
    /// 读取失败时返回 `Self::Error`。
    fn spent(
        &self,
        key: &KeyId,
        currency: Currency,
        since: UnixMillis,
    ) -> Result<Money, Self::Error>;
}

pub trait Ledger {
    type Error;

    /// # Errors
    ///
    /// 借贷不平，或条目混用币种时，返回 `Self::Error`。
    fn record(&self, entries: &[Entry]) -> Result<(), Self::Error>;
}
