use kernel::model::AccountId;

pub trait ApiKeys {
    type Error;

    /// # Errors
    ///
    /// 校验过程失败时返回 `Self::Error`。密钥不存在、密钥未启用或账号未启用时返回 `Ok(None)`。
    fn authenticate(&self, key: &str) -> Result<Option<AccountId>, Self::Error>;
}
