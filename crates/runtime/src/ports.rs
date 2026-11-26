use crate::errors::Rejection;

pub trait Gateway {
    type Error: From<Rejection>;

    /// 一次请求的扣费流程如下。
    ///
    /// 认证：`ApiKeys::authenticate` 返回 `None` 时为 `Rejection::Unauthenticated`；`limit` 返回 `None` 时为 `Rejection::UnknownKey`。
    ///
    /// `RequestId`：带 `idempotency_key` 时用它，并按账号隔离；没带时网关生成。同一 `RequestId` 重试不重复扣费。
    ///
    /// 冻结额：`Adapter::estimate_usage` 加 `Pricing` 定冻结额；没带 `max_tokens` 时按模型最大输出上限估算。
    ///
    /// 限额：有 `SpendLimit` 时构造 `KeyCap`。`Lifetime` 的 `since` 是 `UnixMillis(0)`。`Daily`、`Monthly` 的 `since` 按 `Clock::now` 所在的 UTC 自然日、自然月起点算。限额币种与钱包币种不一致时为 `Rejection::CurrencyMismatch`，不换算。超限为 `Rejection::LimitExceeded`。余额不足为 `Rejection::InsufficientBalance`。
    ///
    /// 上游：`Upstream::call` 失败时 `release` 这笔冻结。响应体是 `Vec<u8>`，不支持流式响应，流式以后另加接口。
    ///
    /// 结算：`read_usage` 加 `Pricing` 得实际费用。实际费用超过冻结额时按冻结额封顶结算，并发 `SettlementCapped`（带 `held` 和 `actual`）进对账。
    ///
    /// 缺用量：`read_usage` 或 `Pricing` 失败时不 `release`、不按 0 结算、也不自动全额结算，冻结挂起等对账，发 `UsageUnreadable`，返回 `Rejection::UsageUnreadable`。
    ///
    /// # Errors
    ///
    /// 被拒绝时返回由 `Rejection` 转换来的 `Self::Error`。上游或存储失败时返回 `Self::Error`。
    fn handle(
        &self,
        key: &str,
        idempotency_key: Option<&str>,
        body: &[u8],
    ) -> Result<Vec<u8>, Self::Error>;
}
