use crate::errors::Rejection;

pub trait Gateway {
    type Error: From<Rejection>;

    /// 按顺序执行：认证、限流、确定 `RequestId`、估算冻结额并冻结、调用上游、结算。有 `idempotency_key` 时按账号用它，否则网关生成。没有 `RequestsPerMinute` 时不限流。
    ///
    /// `Admission::Throttled`，或同一 `RequestId` 已经冻结或结算过时，不再冻结、不再调用上游。上游失败时 `release` 这笔冻结。实际费用超过冻结额时按冻结额封顶结算，并发 `SettlementCapped`。缺用量时不 `release`、不按 0 也不按冻结额结算，冻结挂起等对账，发 `UsageUnreadable`，照常返回响应体。响应体是 `Vec<u8>`，不支持流式，流式以后另加接口。
    ///
    /// `Lifetime` 的 `since` 是 `UnixMillis(0)`。`Daily`、`Monthly` 的 `since` 按 `Clock::now` 所在的 UTC 自然日、自然月起点算。
    ///
    /// # Errors
    ///
    /// `ApiKeys::authenticate` 返回 `None` 时为 `Rejection::Unauthenticated`。`ApiKeys::limit` 返回 `None` 时为 `Rejection::UnknownKey`。`Admission::Throttled` 时为 `Rejection::RateLimited`。同一 `RequestId` 已经冻结或结算过时为 `Rejection::DuplicateRequest`。`Wallet::hold` 返回余额不足、超限或币种不一致时，分别为 `Rejection::InsufficientBalance`、`Rejection::LimitExceeded`、`Rejection::CurrencyMismatch`。这些拒绝由 `Rejection` 转换成 `Self::Error`。
    ///
    /// 其余 `BillingRejection`（如 `Overflow`、`HoldNotFound`）以及 `WalletError::Storage` 按内部错误作为 `Self::Error` 返回，不映射成 `Rejection`。
    fn handle(
        &self,
        key: &str,
        idempotency_key: Option<&str>,
        body: &[u8],
    ) -> Result<Vec<u8>, Self::Error>;
}
