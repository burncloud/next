use kernel::model::{AccountId, Currency, KeyId, Money, RequestId, UnixMillis};

use crate::errors::WalletError;
use crate::model::{Entry, HoldId, KeyCap};

pub trait Pricing {
    type Usage;
    type Error;

    /// # Errors
    ///
    /// 没配价格的模型、或金额溢出时返回 `Self::Error`，不按 0 计价，不回退到其他模型的价格。
    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error>;
}

pub trait Wallet {
    type Error;

    /// # Errors
    ///
    /// 读不到该账号在该 `currency` 下的余额时返回 `WalletError::Storage`。
    fn balance(
        &self,
        account: &AccountId,
        currency: Currency,
    ) -> impl core::future::Future<Output = Result<Money, WalletError<Self::Error>>> + Send;

    /// 带 `cap` 时，在同一次冻结里原子地检查：该密钥自 `cap.since` 起已结算的花费，加上当前冻结，再加上 `amount`，不超过 `cap.limit`。`cap.limit` 与 `amount` 币种不同时直接拒绝，不换算。幂等按 `(account, request)` 隔离，两个账号可以有同一个 `RequestId`。成功时返回钱包生成的 `HoldId`，每次冻结一个新值。失败时没有 `HoldId`，调用方不得调用 `release`。
    ///
    /// # Errors
    ///
    /// 可用余额不足时返回 `WalletError::Rejected(BillingRejection::InsufficientBalance)`。同一个 `account` 和 `request` 已经冻结过时返回 `WalletError::Rejected(BillingRejection::DuplicateRequest)`。超过限额时返回 `WalletError::Rejected(BillingRejection::LimitExceeded)`。币种不一致时返回 `WalletError::Rejected(BillingRejection::CurrencyMismatch)`。金额相加溢出时返回 `WalletError::Rejected(BillingRejection::Overflow)`。存储失败时返回 `WalletError::Storage`。
    fn hold(
        &self,
        account: &AccountId,
        request: &RequestId,
        key: &KeyId,
        cap: Option<KeyCap>,
        amount: Money,
    ) -> impl core::future::Future<Output = Result<HoldId, WalletError<Self::Error>>> + Send;

    /// 结算后冻结结束，没用完的部分退回可用余额。调用方先把实际金额封顶到冻结额。`hold` 是 `Wallet::hold` 返回的那一笔 `HoldId`。
    ///
    /// # Errors
    ///
    /// 没有这个 `account` 和 `request` 的记录时返回 `WalletError::Rejected(BillingRejection::HoldNotFound)`。有未结束的冻结、但 `hold` 不是它的 `HoldId` 时返回 `WalletError::Rejected(BillingRejection::HoldMismatch)`，不结算、不释放。`amount.currency` 与冻结币种不同时返回 `WalletError::Rejected(BillingRejection::CurrencyMismatch)`。实际金额超过冻结额时返回 `WalletError::Rejected(BillingRejection::SettlementExceedsHold)`。已经结算过时返回 `WalletError::Rejected(BillingRejection::AlreadySettled)`。这笔冻结已经释放过时返回 `WalletError::Rejected(BillingRejection::AlreadyReleased)`。金额相加溢出时返回 `WalletError::Rejected(BillingRejection::Overflow)`。存储失败时返回 `WalletError::Storage`。没有未结束的冻结时不比较令牌。
    fn settle(
        &self,
        account: &AccountId,
        request: &RequestId,
        hold: &HoldId,
        amount: Money,
    ) -> impl core::future::Future<Output = Result<(), WalletError<Self::Error>>> + Send;

    /// 用 `hold` 释放 `Wallet::hold` 返回的那一笔未结束冻结。`hold` 失败时没有 `HoldId`，不得调用本方法。
    ///
    /// # Errors
    ///
    /// 没有这个 `account` 和 `request` 的记录时返回 `WalletError::Rejected(BillingRejection::HoldNotFound)`。有未结束的冻结、但 `hold` 不是它的 `HoldId` 时返回 `WalletError::Rejected(BillingRejection::HoldMismatch)`，不释放。已经释放过时返回 `WalletError::Rejected(BillingRejection::AlreadyReleased)`。已经结算过时返回 `WalletError::Rejected(BillingRejection::AlreadySettled)`。存储失败时返回 `WalletError::Storage`。没有未结束的冻结时不比较令牌。
    fn release(
        &self,
        account: &AccountId,
        request: &RequestId,
        hold: &HoldId,
    ) -> impl core::future::Future<Output = Result<(), WalletError<Self::Error>>> + Send;

    /// # Errors
    ///
    /// 没有这个 `account` 和 `request` 的冻结时返回 `WalletError::Rejected(BillingRejection::HoldNotFound)`。存储失败时返回 `WalletError::Storage`。
    fn held(
        &self,
        account: &AccountId,
        request: &RequestId,
    ) -> impl core::future::Future<Output = Result<Money, WalletError<Self::Error>>> + Send;

    /// 按传入的 `since` 统计该密钥在 `currency` 下已结算的花费，加上当前冻结中的金额。
    ///
    /// # Errors
    ///
    /// 金额相加溢出时返回 `WalletError::Rejected(BillingRejection::Overflow)`。读不到这些花费时返回 `WalletError::Storage`。
    fn spent(
        &self,
        key: &KeyId,
        currency: Currency,
        since: UnixMillis,
    ) -> impl core::future::Future<Output = Result<Money, WalletError<Self::Error>>> + Send;
}

pub trait Ledger {
    type Error;

    /// # Errors
    ///
    /// 借贷不平返回 `WalletError::Rejected(BillingRejection::Unbalanced)`。金额相加溢出返回 `WalletError::Rejected(BillingRejection::Overflow)`。混用币种返回 `WalletError::Rejected(BillingRejection::CurrencyMismatch)`。存储失败返回 `WalletError::Storage`。
    fn record(
        &self,
        entries: &[Entry],
    ) -> impl core::future::Future<Output = Result<(), WalletError<Self::Error>>> + Send;
}
