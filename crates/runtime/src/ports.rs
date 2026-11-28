use core::time::Duration;

use billing::model::{ModelUsage, PendingHold};
use billing::ports::Pricing;
use kernel::model::RequestId;
use router::ports::ByteStream;

use crate::errors::{ReconcileError, Rejection};
use crate::events::{
    Fault, MeterFailed, SettlementCapped, Shortfall, UsageUnreadable, UsageUnreadableRatio,
};
use crate::model::{ReconcileQuote, Reply, Suspended};

pub trait Gateway {
    type Error: From<Rejection>;
    type Body: ByteStream;

    /// 按顺序执行：认证、限流、确定 `RequestId`、估算冻结额并冻结、调用上游、边读边转发、结算。有 `idempotency_key` 时按账号用它，否则 `RequestIds::next_id` 生成，且每次都不能与之前重复。没有 `RequestsPerMinute` 时不限流。冻结额来自这一次请求的 `PriceSource::snapshot`，结算用同一份。没有价格、价格为 0，或快照含零价、非 `USD` 的坏行时拒绝，不冻结。`Wallet::hold` 成功时返回这一笔冻结的 `HoldId`。之后的 `settle` 和 `release` 必须带上它。`hold` 失败时没有 `HoldId`，不得调用 `release`。
    ///
    /// `Admission::Throttled`，或同一 `RequestId` 已经冻结或结算过时，不再冻结、不再调用上游。
    ///
    /// 上游正常结束指 `ByteStream` 返回 `None` 且 `UsageMeter::ended` 为真。结局只有一个：
    /// - 读到用量：用这笔 `HoldId` 按真实用量 `settle`；实际费用超过冻结额或计价溢出时按冻结额封顶结算，并发 `SettlementCapped` 与 `Shortfall`。上游没有正常结束但读到了用量，也这样结算。
    /// - 上游正常结束但读不到用量：不 `release`、不按 0 也不按冻结额结算，冻结挂起等对账，发 `UsageUnreadable`，照常返回响应体。这条进入 `Suspended::UsageUnreadable`。
    /// - 上游没有正常结束且没有用量，或响应头之前失败，或非 2xx：用这笔 `HoldId` `release`，不扣费。2xx 且 meter 的 `feed` 已报错时，释放之外再发 `MeterFailed`。
    ///
    /// 调用方丢弃 `Reply::body`（客户端断开）时，读完上游的任务交给 `Spawn`，不能因丢弃而取消，按上面的结局处理。结算或释放没有完成时，冻结留给对账：结算没有完成进入 `Suspended::SettleUnfinished`，释放没有完成进入 `Suspended::ReleaseUnfinished`。显式动作只走 `Reconciliation`，并且必须交回原来的 `HoldId`。
    ///
    /// `Lifetime` 的 `since` 是 `UnixMillis(0)`。`Daily`、`Monthly` 的 `since` 按 `Clock::now` 所在的 UTC 自然日、自然月起点算。
    ///
    /// # Errors
    ///
    /// `ApiKeys::authenticate` 返回 `None` 时为 `Rejection::Unauthenticated`。`ApiKeys::limit` 返回 `None` 时为 `Rejection::UnknownKey`。`Admission::Throttled` 时为 `Rejection::RateLimited`。同一 `RequestId` 已经冻结或结算过时为 `Rejection::DuplicateRequest`。`Wallet::hold` 返回余额不足、超限或币种不一致时，分别为 `Rejection::InsufficientBalance`、`Rejection::LimitExceeded`、`Rejection::CurrencyMismatch`。这些拒绝由 `Rejection` 转换成 `Self::Error`。
    ///
    /// `Adapter::prepare` 失败、没有价格、价格为 0、上游在响应头之前失败，以及其余 `BillingRejection`（如 `Overflow`、`HoldNotFound`、`HoldMismatch`）和 `WalletError::Storage`，作为 `Self::Error` 返回，不映射成 `Rejection`。还没调用上游、且 `hold` 已经返回 `HoldId` 时，用这个令牌 `release`。没有令牌时什么都不释放。
    fn handle(
        &self,
        key: &str,
        idempotency_key: Option<&str>,
        body: &[u8],
    ) -> impl core::future::Future<Output = Result<Reply<Self::Body>, Self::Error>> + Send;
}

/// 客户端断开后仍要跑完的任务交给它。
pub trait Spawn: Send + Sync + 'static {
    /// 任务必须跑到结束，不能因为调用方丢弃 `Reply::body` 而取消。
    fn spawn<F>(&self, task: F)
    where
        F: core::future::Future<Output = ()> + Send + 'static;

    /// 结算或释放存储失败后，两次重试之间等待用。
    fn sleep(&self, duration: Duration) -> impl core::future::Future<Output = ()> + Send;
}

/// 每个请求取一次快照，冻结和结算都用同一份。没有价格或价格为 0 时不得冻结。
pub trait PriceSource: Send + Sync + 'static {
    type Prices: Pricing<Usage = ModelUsage, Error: Send> + Send + 'static;
    type Error: core::fmt::Debug + Send;

    /// # Errors
    ///
    /// 价格表读不出来，或有零价、非 `USD` 等坏行时返回 `Self::Error`，不能用部分价格表继续。
    fn snapshot(
        &self,
    ) -> impl core::future::Future<Output = Result<Self::Prices, Self::Error>> + Send;
}

/// 请求没带 `Idempotency-Key` 时生成 `RequestId`。
pub trait RequestIds: Send + Sync + 'static {
    /// 每次返回的值都不能与之前的重复。
    fn next_id(&self) -> RequestId;
}

/// 首期由实现方写结构化日志：`usage_unreadable`、`usage_unreadable_ratio`、`settlement_capped`、`meter_failed` 和 `fault`。本 crate 不写日志。
pub trait Report: Send + Sync + 'static {
    /// 上游正常结束但读不到用量。冻结挂起，不释放，不按 0 结算。
    fn usage_unreadable(&self, event: &UsageUnreadable);

    /// 已决请求中读不出用量的比例升到阈值。
    fn usage_unreadable_ratio(&self, event: &UsageUnreadableRatio);

    /// 实际费用超过冻结额、计价溢出，或币种不一致时，按冻结额封顶结算。
    fn settlement_capped(&self, event: &SettlementCapped, shortfall: Shortfall);

    /// 上游没有正常结束，且 meter 的 `feed` 已报错。释放冻结，不扣费。
    fn meter_failed(&self, event: &MeterFailed);

    /// 编排中的内部失败。
    fn fault(&self, fault: &Fault);
}

/// 列出挂起的冻结，并带着原来的 `HoldId` 显式结算或释放。
///
/// 不接受时间上界，不自动过期，列出时不结算、不释放。挂起多久、要不要自动处理，不在这里决定。账本只插入，不改已有行；只使用 `USD`。
pub trait Reconciliation: Send + Sync + 'static {
    type Error: Send;

    /// 返回仍挂起的冻结，按 `created_at`、账户、请求排序。没有则空列表。
    ///
    /// 只包括 `Suspended` 的三种。不包括仍在读取或转发的冻结，不包括上游失败或断开且没有用量时已经释放的冻结，也不包括客户端断开后已经按实际用量结算的冻结。每条 `pending.hold` 都是 `Wallet::hold` 返回的 `HoldId`。
    ///
    /// # Errors
    ///
    /// 读不到名单时返回 `ReconcileError::Wallet(WalletError::Storage)`。
    fn suspended(
        &self,
    ) -> impl core::future::Future<Output = Result<Vec<Suspended>, ReconcileError<Self::Error>>> + Send;

    /// 显式结算。必须把 `pending.hold` 原样交给 `Wallet::settle`，并带上同一条的账户和请求。不能只按账户和请求结算。
    ///
    /// `UsageUnreadable` 与 `SettleUnfinished` 在没有价格或价格为 0 时都拒绝，分别返回 `MissingPrice`、`ZeroPrice`，不按 0 结算，名单保留。`SettleUnfinished` 的 `Priced` 只能等于名单上记录的金额。币种不是 `USD` 时返回 `BillingRejection::CurrencyMismatch`。不向 `Wallet::settle` 提交大于名单里该条冻结额的金额。超额如何处置不在这里决定。`ReleaseUnfinished` 不能改成结算。比较用的冻结额是名单里匹配到的那一条，不是调用方传入的 `pending.amount`。
    ///
    /// # Errors
    ///
    /// 账户和请求不在名单里时返回 `ReconcileError::NotSuspended`。`HoldId` 不符时返回 `ReconcileError::Wallet(WalletError::Rejected(BillingRejection::HoldMismatch))`，不结算、不释放。`ReleaseUnfinished` 返回 `ReconcileError::WrongAction`。`SettleUnfinished` 的 `Priced` 与名单金额不同时返回 `ReconcileError::AmountChanged`。没有价格、价格为 0 时，`UsageUnreadable` 与 `SettleUnfinished` 都返回 `ReconcileError::MissingPrice`、`ReconcileError::ZeroPrice`。非 `USD` 返回 `ReconcileError::Wallet(WalletError::Rejected(BillingRejection::CurrencyMismatch))`。大于名单里该条冻结额时返回 `BillingRejection::SettlementExceedsHold`。`HoldNotFound`、`Overflow` 和存储失败放在 `ReconcileError::Wallet`。这些情况都不改名单。`AlreadySettled` 与 `AlreadyReleased` 表示这笔冻结在钱包里已经结束，该条不再挂起，之后不得再出现在 `suspended` 里。
    fn settle(
        &self,
        pending: &PendingHold,
        quote: ReconcileQuote,
    ) -> impl core::future::Future<Output = Result<(), ReconcileError<Self::Error>>> + Send;

    /// 显式释放，不扣费。必须把 `pending.hold` 原样交给 `Wallet::release`。
    ///
    /// `Suspended::ReleaseUnfinished` 和 `Suspended::UsageUnreadable` 可以释放。`Suspended::SettleUnfinished` 不能改成释放。不因时间过去而调用。上游失败或断开且没有用量时，网关已经释放，那些冻结不经过本方法。
    ///
    /// # Errors
    ///
    /// 不在名单里时返回 `ReconcileError::NotSuspended`。`HoldId` 不符时返回 `ReconcileError::Wallet(WalletError::Rejected(BillingRejection::HoldMismatch))`，不释放。`SettleUnfinished` 返回 `ReconcileError::WrongAction`。`HoldNotFound` 和存储失败放在 `ReconcileError::Wallet`，名单不变。`AlreadyReleased` 与 `AlreadySettled` 表示这笔冻结在钱包里已经结束，该条不再挂起，之后不得再出现在 `suspended` 里。
    fn release(
        &self,
        pending: &PendingHold,
    ) -> impl core::future::Future<Output = Result<(), ReconcileError<Self::Error>>> + Send;
}
