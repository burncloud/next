use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;

use std::sync::Mutex;

use billing::errors::{BillingRejection, WalletError};
use billing::model::{HoldId, ModelUsage, PendingHold, TokenUsage};
use billing::ports::Pricing;
use kernel::model::{AccountId, Currency, KeyId, ModelId, Money, RequestId, UnixMillis};
use router::ports::ByteStream;
use runtime::errors::{ReconcileError, Rejection};
use runtime::events::{
    ActualCost, Fault, MeterFailed, SettlementCapped, Shortfall, Stage, UsageUnreadable,
    UsageUnreadableRatio,
};
use runtime::model::{ReconcileQuote, Reply, Suspended};
use runtime::ports::{Gateway, PriceSource, Reconciliation, Report, RequestIds, Spawn};

fn poll_once<T>(future: impl Future<Output = T>) -> Option<T> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Rejected(Rejection),
}

impl From<Rejection> for Failure {
    fn from(rejection: Rejection) -> Self {
        Self::Rejected(rejection)
    }
}

struct Once(Option<Vec<u8>>);

impl ByteStream for Once {
    type Error = ();

    fn next_chunk(&mut self) -> impl Future<Output = Option<Result<Vec<u8>, Self::Error>>> + Send {
        core::future::ready(self.0.take().map(Ok))
    }
}

struct Stub;

impl Gateway for Stub {
    type Error = Failure;
    type Body = Once;

    fn handle(
        &self,
        key: &str,
        idempotency_key: Option<&str>,
        body: &[u8],
    ) -> impl Future<Output = Result<Reply<Self::Body>, Self::Error>> + Send {
        let reply = match (key, idempotency_key) {
            ("", _) => Err(Rejection::Unauthenticated.into()),
            (_, Some(_)) => Err(Rejection::DuplicateRequest.into()),
            _ => Ok(Reply {
                status: 200,
                streaming: true,
                body: Once(Some(body.to_vec())),
            }),
        };
        core::future::ready(reply)
    }
}

#[test]
fn gateway_is_async_and_streams() {
    let gateway = Stub;
    assert!(matches!(
        poll_once(gateway.handle("", None, b"x")),
        Some(Err(Failure::Rejected(Rejection::Unauthenticated)))
    ));
    assert!(matches!(
        poll_once(gateway.handle("k", Some("i"), b"x")),
        Some(Err(Failure::Rejected(Rejection::DuplicateRequest)))
    ));
    let reply = poll_once(gateway.handle("k", None, b"x"));
    assert!(matches!(
        &reply,
        Some(Ok(Reply {
            status: 200,
            streaming: true,
            ..
        }))
    ));
    if let Some(Ok(mut reply)) = reply {
        assert_eq!(
            poll_once(reply.body.next_chunk()),
            Some(Some(Ok(b"x".to_vec())))
        );
        assert_eq!(poll_once(reply.body.next_chunk()), Some(None));
    }
}

#[test]
fn events_carry_account_and_actual_cost() {
    let held = Money {
        currency: Currency::Usd,
        minor_units: 10,
    };
    let unreadable = UsageUnreadable {
        request: RequestId(String::from("r")),
        account: AccountId(String::from("a")),
        key: KeyId(String::from("k")),
        held,
        at: UnixMillis(1),
    };
    let capped = SettlementCapped {
        request: unreadable.request.clone(),
        account: unreadable.account.clone(),
        key: unreadable.key.clone(),
        held,
        actual: ActualCost::Overflow,
        at: UnixMillis(2),
    };
    assert_eq!(capped.account, unreadable.account);
    assert_ne!(
        capped.actual,
        ActualCost::Priced(Money {
            currency: Currency::Usd,
            minor_units: 11,
        })
    );
    let ratio = UsageUnreadableRatio {
        decided: 100,
        unreadable: 1,
        window: 100,
        ratio_num: 1,
        ratio_den: 100,
        at: UnixMillis(3),
    };
    let failed = MeterFailed {
        request: unreadable.request.clone(),
        account: unreadable.account.clone(),
        key: unreadable.key.clone(),
        held,
        at: UnixMillis(4),
    };
    let fault = Fault {
        stage: Stage::Prices,
        request: None,
        account: Some(unreadable.account.clone()),
        attempts: 1,
        detail: String::new(),
    };
    let report = Reporter;
    report.usage_unreadable(&unreadable);
    report.usage_unreadable_ratio(&ratio);
    report.settlement_capped(&capped, Shortfall::Overflow);
    report.meter_failed(&failed);
    report.fault(&fault);
    assert!(matches!(Shortfall::Minor(2), Shortfall::Minor(2)));
    assert!(matches!(
        Shortfall::CurrencyMismatch,
        Shortfall::CurrencyMismatch
    ));
}

struct Prices;

impl Pricing for Prices {
    type Usage = ModelUsage;
    type Error = ();

    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error> {
        let tokens = &usage.tokens;
        let Some(units) = tokens
            .input_tokens
            .checked_add(tokens.cached_input_tokens)
            .and_then(|sum| sum.checked_add(tokens.output_tokens))
            .filter(|units| *units > 0 && !usage.model.0.is_empty())
        else {
            return Err(());
        };
        Ok(Money {
            currency: Currency::Usd,
            minor_units: units,
        })
    }
}

struct Source(bool);

impl PriceSource for Source {
    type Prices = Prices;
    type Error = ();

    fn snapshot(&self) -> impl Future<Output = Result<Self::Prices, Self::Error>> + Send {
        core::future::ready(if self.0 { Err(()) } else { Ok(Prices) })
    }
}

struct Ids(Mutex<u64>);

impl RequestIds for Ids {
    fn next_id(&self) -> RequestId {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = *guard;
        *guard = id.saturating_add(1);
        RequestId(id.to_string())
    }
}

struct Spawner;

impl Spawn for Spawner {
    fn spawn<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        assert_eq!(poll_once(task), Some(()));
    }

    fn sleep(&self, _duration: Duration) -> impl Future<Output = ()> + Send {
        core::future::ready(())
    }
}

struct Reporter;

impl Report for Reporter {
    fn usage_unreadable(&self, event: &UsageUnreadable) {
        assert_eq!(event.held.minor_units, 10);
    }

    fn usage_unreadable_ratio(&self, event: &UsageUnreadableRatio) {
        assert_eq!((event.ratio_num, event.ratio_den), (1, 100));
    }

    fn settlement_capped(&self, event: &SettlementCapped, shortfall: Shortfall) {
        assert_eq!(
            (event.held.minor_units, shortfall),
            (10, Shortfall::Overflow)
        );
    }

    fn meter_failed(&self, event: &MeterFailed) {
        assert_eq!(event.held.currency, Currency::Usd);
    }

    fn fault(&self, fault: &Fault) {
        assert_eq!((fault.stage, fault.detail.as_str()), (Stage::Prices, ""));
    }
}

fn usage(model: &str, input_tokens: u64) -> ModelUsage {
    ModelUsage {
        model: ModelId(String::from(model)),
        tokens: TokenUsage {
            input_tokens,
            cached_input_tokens: 0,
            output_tokens: 0,
        },
    }
}

#[test]
fn missing_or_zero_price_is_rejected() {
    assert!(matches!(poll_once(Source(true).snapshot()), Some(Err(()))));
    assert!(matches!(
        poll_once(Source(false).snapshot()),
        Some(Ok(Prices))
    ));
    let prices = Prices;
    assert!(prices.price(&usage("", 3)).is_err());
    assert!(prices.price(&usage("m", 0)).is_err());
    assert_eq!(
        prices.price(&usage("m", 3)),
        Ok(Money {
            currency: Currency::Usd,
            minor_units: 3,
        })
    );
}

#[test]
fn request_ids_do_not_repeat_and_spawn_finishes() {
    let ids = Ids(Mutex::new(1));
    assert_ne!(ids.next_id(), ids.next_id());
    Spawner.spawn(core::future::ready(()));
    assert_eq!(poll_once(Spawner.sleep(Duration::from_millis(1))), Some(()));
}

fn usd(minor_units: u64) -> Money {
    Money {
        currency: Currency::Usd,
        minor_units,
    }
}

fn hold(id: &str, account: &str, request: &str, at: u64, minor: u64) -> PendingHold {
    PendingHold {
        hold: HoldId(String::from(id)),
        account: AccountId(String::from(account)),
        request: RequestId(String::from(request)),
        key: KeyId(String::from("k")),
        amount: usd(minor),
        created_at: UnixMillis(at),
    }
}

fn pending_of(item: &Suspended) -> &PendingHold {
    match item {
        Suspended::UsageUnreadable(pending)
        | Suspended::ReleaseUnfinished(pending)
        | Suspended::SettleUnfinished { pending, .. } => pending,
    }
}

fn wallet_reject(rejection: BillingRejection) -> ReconcileError<()> {
    ReconcileError::Wallet(WalletError::Rejected(rejection))
}

fn locate(rows: &[Suspended], pending: &PendingHold) -> Result<usize, ReconcileError<()>> {
    let Some(index) = rows.iter().position(|item| {
        let row = pending_of(item);
        row.account == pending.account && row.request == pending.request
    }) else {
        return Err(ReconcileError::NotSuspended);
    };
    if pending_of(&rows[index]).hold == pending.hold {
        Ok(index)
    } else {
        Err(wallet_reject(BillingRejection::HoldMismatch))
    }
}

fn priced_ok(held: Money, priced: Money) -> Result<(), ReconcileError<()>> {
    if priced.currency != Currency::Usd || held.currency != Currency::Usd {
        return Err(wallet_reject(BillingRejection::CurrencyMismatch));
    }
    if priced.minor_units == 0 {
        return Err(ReconcileError::ZeroPrice);
    }
    if priced.minor_units > held.minor_units {
        return Err(wallet_reject(BillingRejection::SettlementExceedsHold));
    }
    Ok(())
}

fn apply_settle(
    rows: &mut Vec<Suspended>,
    pending: &PendingHold,
    quote: ReconcileQuote,
) -> Result<(), ReconcileError<()>> {
    let index = locate(rows, pending)?;
    let held = pending_of(&rows[index]).amount;
    match &rows[index] {
        Suspended::ReleaseUnfinished(_) => Err(ReconcileError::WrongAction),
        Suspended::SettleUnfinished { amount, .. } => match quote {
            ReconcileQuote::Missing => Err(ReconcileError::MissingPrice),
            ReconcileQuote::Zero => Err(ReconcileError::ZeroPrice),
            ReconcileQuote::Priced(priced) if priced == *amount => priced_ok(held, priced),
            ReconcileQuote::Priced(_) => Err(ReconcileError::AmountChanged),
        },
        Suspended::UsageUnreadable(_) => match quote {
            ReconcileQuote::Missing => Err(ReconcileError::MissingPrice),
            ReconcileQuote::Zero => Err(ReconcileError::ZeroPrice),
            ReconcileQuote::Priced(priced) => priced_ok(held, priced),
        },
    }?;
    rows.remove(index);
    Ok(())
}

fn apply_release(
    rows: &mut Vec<Suspended>,
    pending: &PendingHold,
) -> Result<(), ReconcileError<()>> {
    let index = locate(rows, pending)?;
    match &rows[index] {
        Suspended::SettleUnfinished { .. } => Err(ReconcileError::WrongAction),
        Suspended::UsageUnreadable(_) | Suspended::ReleaseUnfinished(_) => {
            rows.remove(index);
            Ok(())
        }
    }
}

fn sorted(rows: &[Suspended]) -> Vec<Suspended> {
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| {
        let left = pending_of(left);
        let right = pending_of(right);
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.account.0.cmp(&right.account.0))
            .then_with(|| left.request.0.cmp(&right.request.0))
    });
    rows
}

struct Books(Mutex<Vec<Suspended>>);

impl Reconciliation for Books {
    type Error = ();

    fn suspended(&self) -> impl Future<Output = Result<Vec<Suspended>, ReconcileError<()>>> + Send {
        let rows = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        core::future::ready(Ok(sorted(&rows)))
    }

    fn settle(
        &self,
        pending: &PendingHold,
        quote: ReconcileQuote,
    ) -> impl Future<Output = Result<(), ReconcileError<()>>> + Send {
        let mut rows = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        core::future::ready(apply_settle(&mut rows, pending, quote))
    }

    fn release(
        &self,
        pending: &PendingHold,
    ) -> impl Future<Output = Result<(), ReconcileError<()>>> + Send {
        let mut rows = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        core::future::ready(apply_release(&mut rows, pending))
    }
}

fn sample_books() -> Books {
    Books(Mutex::new(vec![
        Suspended::SettleUnfinished {
            pending: hold("s", "b", "rs", 20, 40),
            amount: usd(15),
        },
        Suspended::UsageUnreadable(hold("u", "a", "ru", 10, 50)),
        Suspended::UsageUnreadable(hold("v", "a", "rv", 11, 30)),
        Suspended::ReleaseUnfinished(hold("r", "a", "rr", 15, 7)),
    ]))
}

fn requests_of(rows: &[Suspended]) -> Vec<&str> {
    rows.iter()
        .map(|item| pending_of(item).request.0.as_str())
        .collect()
}

fn rows_of(books: &Books) -> Vec<Suspended> {
    match poll_once(books.suspended()) {
        Some(Ok(rows)) => rows,
        Some(Err(_)) | None => Vec::new(),
    }
}

fn settle_is(
    books: &Books,
    pending: &PendingHold,
    quote: ReconcileQuote,
    expected: Result<(), ReconcileError<()>>,
) {
    assert_eq!(poll_once(books.settle(pending, quote)), Some(expected));
}

fn release_is(books: &Books, pending: &PendingHold, expected: Result<(), ReconcileError<()>>) {
    assert_eq!(poll_once(books.release(pending)), Some(expected));
}

const OPEN: [&str; 4] = ["ru", "rv", "rr", "rs"];

#[test]
fn reconciliation_keeps_the_hold_id() {
    let books = sample_books();
    let rows = rows_of(&books);
    assert_eq!(requests_of(&rows), OPEN);
    assert_eq!(rows_of(&books), rows);
    let unreadable = pending_of(&rows[0]).clone();
    let priced = pending_of(&rows[1]).clone();
    let release = pending_of(&rows[2]).clone();
    let settle = pending_of(&rows[3]).clone();
    let mut forged = unreadable.clone();
    forged.hold = HoldId(String::from("other"));
    let unknown = hold("x", "z", "rx", 1, 3);
    let mismatch = wallet_reject(BillingRejection::HoldMismatch);
    settle_is(
        &books,
        &unknown,
        ReconcileQuote::Priced(usd(1)),
        Err(ReconcileError::NotSuspended),
    );
    settle_is(
        &books,
        &forged,
        ReconcileQuote::Priced(usd(10)),
        Err(mismatch.clone()),
    );
    release_is(&books, &forged, Err(mismatch));
    settle_is(
        &books,
        &unreadable,
        ReconcileQuote::Missing,
        Err(ReconcileError::MissingPrice),
    );
    settle_is(
        &books,
        &unreadable,
        ReconcileQuote::Zero,
        Err(ReconcileError::ZeroPrice),
    );
    settle_is(
        &books,
        &unreadable,
        ReconcileQuote::Priced(usd(0)),
        Err(ReconcileError::ZeroPrice),
    );
    settle_is(
        &books,
        &unreadable,
        ReconcileQuote::Priced(Money {
            currency: Currency::Cny,
            minor_units: 10,
        }),
        Err(wallet_reject(BillingRejection::CurrencyMismatch)),
    );
    settle_is(
        &books,
        &unreadable,
        ReconcileQuote::Priced(usd(51)),
        Err(wallet_reject(BillingRejection::SettlementExceedsHold)),
    );
    let mut raised = unreadable.clone();
    raised.amount = usd(80);
    settle_is(
        &books,
        &raised,
        ReconcileQuote::Priced(usd(60)),
        Err(wallet_reject(BillingRejection::SettlementExceedsHold)),
    );
    settle_is(
        &books,
        &settle,
        ReconcileQuote::Missing,
        Err(ReconcileError::MissingPrice),
    );
    settle_is(
        &books,
        &settle,
        ReconcileQuote::Zero,
        Err(ReconcileError::ZeroPrice),
    );
    release_is(&books, &settle, Err(ReconcileError::WrongAction));
    settle_is(
        &books,
        &settle,
        ReconcileQuote::Priced(usd(14)),
        Err(ReconcileError::AmountChanged),
    );
    settle_is(
        &books,
        &release,
        ReconcileQuote::Priced(usd(1)),
        Err(ReconcileError::WrongAction),
    );
    assert_eq!(requests_of(&rows_of(&books)), OPEN);
    settle_is(&books, &priced, ReconcileQuote::Priced(usd(10)), Ok(()));
    release_is(&books, &unreadable, Ok(()));
    settle_is(&books, &settle, ReconcileQuote::Priced(usd(15)), Ok(()));
    release_is(&books, &release, Ok(()));
    assert_eq!(rows_of(&books), Vec::new());
    release_is(&books, &unreadable, Err(ReconcileError::NotSuspended));
}
