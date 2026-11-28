use billing::model::PendingHold;
use kernel::model::Money;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reply<B> {
    pub status: u16,
    pub streaming: bool,
    pub body: B,
}

/// 显式结算时看到的价格。没有价格或价格为 0 不能拿去结算。金额是 `Money` 的整数最小单位，不用浮点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReconcileQuote {
    Missing,
    Zero,
    Priced(Money),
}

/// 一笔留给对账的冻结，带着 `Wallet::hold` 返回的 `HoldId`。不是过期，列出时不结算、不释放。`UsageUnreadable` 保持挂起，直到显式结算或释放。`SettleUnfinished` 的 `amount` 是当时要交给 `Wallet::settle` 的金额，这里不重算、不改写。`ReleaseUnfinished` 不扣费。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Suspended {
    UsageUnreadable(PendingHold),
    SettleUnfinished { pending: PendingHold, amount: Money },
    ReleaseUnfinished(PendingHold),
}
