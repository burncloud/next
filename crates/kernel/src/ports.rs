use crate::model::UnixMillis;

pub trait Clock {
    /// 返回当前时刻（Unix 毫秒，UTC）。
    fn now(&self) -> UnixMillis;
}
