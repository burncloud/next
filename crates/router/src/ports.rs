use billing::model::TokenUsage;

use crate::model::{Prepared, UpstreamReply};

pub trait ByteStream: Send {
    type Error;

    /// 返回 `None` 表示传输层读完，`Some(Err)` 表示读取中断。
    fn next_chunk(
        &mut self,
    ) -> impl core::future::Future<Output = Option<Result<Vec<u8>, Self::Error>>> + Send;
}

pub trait UsageMeter: Send {
    type Error;

    /// 按到达顺序传入上游响应体，字节可以在任意位置切开。返回 `Err` 之后，调用方仍要继续传入后续字节、继续向客户端转发、继续读完上游，不能因此中断；之后的 `feed` 和 `finish` 都返回 `Err`。
    ///
    /// # Errors
    ///
    /// 已传入的字节无法按协议读出用量时返回 `Self::Error`。
    fn feed(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    /// 已经读到协议层的结束标记，例如 SSE 的 `[DONE]`，或完整的非流式响应体。
    fn ended(&self) -> bool;

    /// 上游结束后调用一次，不论是否正常结束。上游正常结束（`ByteStream` 返回 `None` 且 `ended` 为真）时返回 `Err`，冻结挂起等对账并发 `UsageUnreadable`，不 `release`。
    ///
    /// # Errors
    ///
    /// 没有用量、用量读不出、缓存命中数大于上游给的输入总数、出现不支持的计费项，或之前的 `feed` 返回过 `Err` 时，返回 `Self::Error`，不能当成 0。
    fn finish(self) -> Result<TokenUsage, Self::Error>;
}

pub trait Adapter {
    type Error;
    type Meter: UsageMeter;

    /// `limit` 是冻结用的用量上限：`cached_input_tokens` 为 0；`output_tokens` 取请求的最大输出，请求没带时取实现给该模型配置的最大输出上限。`upstream_body` 是发给上游的请求体。
    ///
    /// # Errors
    ///
    /// 请求体读不出来、没配该模型的最大输出上限，或请求了不支持的计费档位或参数时，返回 `Self::Error`，不能按 0、默认值或其他模型估算。
    fn prepare(&self, body: &[u8]) -> Result<Prepared<Self::Meter>, Self::Error>;
}

pub trait Upstream {
    type Error;
    type Body: ByteStream;

    /// 非 2xx 不算错误，状态码放在 `UpstreamReply::status`。
    ///
    /// # Errors
    ///
    /// 收到响应头之前失败（连接失败、超时）时返回 `Self::Error`。
    fn call(
        &self,
        body: Vec<u8>,
        streaming: bool,
    ) -> impl core::future::Future<Output = Result<UpstreamReply<Self::Body>, Self::Error>> + Send;
}
