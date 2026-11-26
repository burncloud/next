use crate::model::UnixMillis;

pub trait Clock {
    /// 返回的时刻是 UTC。
    fn now(&self) -> UnixMillis;
}
