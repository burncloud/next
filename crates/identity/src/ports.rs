use kernel::model::KeyId;

use crate::model::{KeyLimit, Principal};

pub trait ApiKeys {
    type Error;

    /// # Errors
    ///
    /// 校验过程失败时返回 `Self::Error`。密钥不存在、密钥未启用或账号未启用时返回 `Ok(None)`。
    fn authenticate(&self, key: &str) -> Result<Option<Principal>, Self::Error>;

    /// # Errors
    ///
    /// 读取限额失败时返回 `Self::Error`。没有这个 `KeyId` 时返回 `Ok(None)`。
    fn limit(&self, key: &KeyId) -> Result<Option<KeyLimit>, Self::Error>;
}
