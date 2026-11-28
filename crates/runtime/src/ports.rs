use router::ports::ByteStream;

use crate::errors::Rejection;
use crate::model::Reply;

pub trait Gateway {
    type Error: From<Rejection>;
    type Body: ByteStream;

    /// 按顺序执行：认证、限流、确定 `RequestId`、估算冻结额并冻结、调用上游、边读边转发、结算。有 `idempotency_key` 时按账号用它，否则实现生成。没有 `RequestsPerMinute` 时不限流。`Wallet::hold` 成功时返回这一笔冻结的 `HoldId`。之后的 `settle` 和 `release` 必须带上它。`hold` 失败时没有 `HoldId`，不得调用 `release`。
    ///
    /// `Admission::Throttled`，或同一 `RequestId` 已经冻结或结算过时，不再冻结、不再调用上游。
    ///
    /// 上游正常结束指 `ByteStream` 返回 `None` 且 `UsageMeter::ended` 为真。结局只有一个：
    /// - 读到用量：用这笔 `HoldId` 按真实用量 `settle`；实际费用超过冻结额或计价溢出时按冻结额封顶结算，并发 `SettlementCapped`。上游没有正常结束但读到了用量，也这样结算。
    /// - 上游正常结束但读不到用量：不 `release`、不按 0 也不按冻结额结算，冻结挂起等对账，发 `UsageUnreadable`，照常返回响应体。
    /// - 上游没有正常结束且没有用量，或响应头之前失败，或非 2xx：用这笔 `HoldId` `release`，不扣费。
    ///
    /// 调用方丢弃 `Reply::body`（客户端断开）时，实现仍读完上游，按上面的结局处理。已经调用上游之后的内部错误不 `release`，冻结留给对账。
    ///
    /// `Lifetime` 的 `since` 是 `UnixMillis(0)`。`Daily`、`Monthly` 的 `since` 按 `Clock::now` 所在的 UTC 自然日、自然月起点算。
    ///
    /// # Errors
    ///
    /// `ApiKeys::authenticate` 返回 `None` 时为 `Rejection::Unauthenticated`。`ApiKeys::limit` 返回 `None` 时为 `Rejection::UnknownKey`。`Admission::Throttled` 时为 `Rejection::RateLimited`。同一 `RequestId` 已经冻结或结算过时为 `Rejection::DuplicateRequest`。`Wallet::hold` 返回余额不足、超限或币种不一致时，分别为 `Rejection::InsufficientBalance`、`Rejection::LimitExceeded`、`Rejection::CurrencyMismatch`。这些拒绝由 `Rejection` 转换成 `Self::Error`。
    ///
    /// `Adapter::prepare` 失败、没有价格、上游在响应头之前失败，以及其余 `BillingRejection`（如 `Overflow`、`HoldNotFound`、`HoldMismatch`）和 `WalletError::Storage`，作为 `Self::Error` 返回，不映射成 `Rejection`。还没调用上游、且 `hold` 已经返回 `HoldId` 时，用这个令牌 `release`。没有令牌时什么都不释放。
    fn handle(
        &self,
        key: &str,
        idempotency_key: Option<&str>,
        body: &[u8],
    ) -> impl core::future::Future<Output = Result<Reply<Self::Body>, Self::Error>> + Send;
}
