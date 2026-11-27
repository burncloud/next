use kernel::model::{KeyId, UnixMillis};

use crate::model::{Admission, KeyLimit, Principal, RequestsPerMinute};

pub trait ApiKeys {
    type Error;

    /// # Errors
    ///
    /// 校验过程失败时返回 `Self::Error`。密钥不存在、密钥未启用或账号未启用时返回 `Ok(None)`。
    fn authenticate(
        &self,
        key: &str,
    ) -> impl core::future::Future<Output = Result<Option<Principal>, Self::Error>> + Send;

    /// # Errors
    ///
    /// 读取限额失败时返回 `Self::Error`。没有这个 `KeyId` 时返回 `Ok(None)`。
    fn limit(
        &self,
        key: &KeyId,
    ) -> impl core::future::Future<Output = Result<Option<KeyLimit>, Self::Error>> + Send;
}

pub trait RateLimiter {
    type Error;

    /// 按密钥统计 `now` 往前 60 秒内放行过的请求数（滑动窗口）。未达 `limit` 时原子地计入本次并返回 `Admission::Allowed`；已达到时返回 `Admission::Throttled`，不计入。
    ///
    /// # Errors
    ///
    /// 读写计数失败时返回 `Self::Error`，不能当成放行。
    fn admit(
        &self,
        key: &KeyId,
        limit: RequestsPerMinute,
        now: UnixMillis,
    ) -> impl core::future::Future<Output = Result<Admission, Self::Error>> + Send;
}
