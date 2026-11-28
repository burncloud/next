use kernel::model::ModelId;

use crate::errors::CatalogError;
use crate::model::ModelRoute;

pub trait Catalog {
    type Error;

    /// 按 `model` 原样精确匹配到一个渠道。返回的 `ModelRoute::model` 与传入值逐字相同。不做前缀匹配，不把别名改写成另一个模型，也不改选别的渠道。
    ///
    /// # Errors
    ///
    /// 目录里没有这个模型时返回 `CatalogError::Rejected(CatalogRejection::UnknownModel)`。读取目录失败时返回 `CatalogError::Storage`。
    fn lookup(
        &self,
        model: &ModelId,
    ) -> impl core::future::Future<Output = Result<ModelRoute, CatalogError<Self::Error>>> + Send;
}
