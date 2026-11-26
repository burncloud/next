use core::future::Future;
use core::pin::pin;
use core::task::{Context, Waker};

use kernel::model::{AccountId, Currency, KeyId, Money, RequestId, UnixMillis};

use billing::errors::{BillingRejection, WalletError};
use billing::model::{Entry, KeyCap};
use billing::ports::{Ledger, Pricing, Wallet};

struct Stub;

impl Pricing for Stub {
    type Usage = ();
    type Error = ();

    fn price(&self, _usage: &Self::Usage) -> Result<Money, Self::Error> {
        Ok(Money {
            currency: Currency::Usd,
            minor_units: 0,
        })
    }
}

impl Wallet for Stub {
    type Error = ();

    fn balance(
        &self,
        _account: &AccountId,
        _currency: Currency,
    ) -> impl Future<Output = Result<Money, WalletError<Self::Error>>> + Send {
        core::future::pending()
    }

    fn hold(
        &self,
        _request: &RequestId,
        _account: &AccountId,
        _key: &KeyId,
        _cap: Option<KeyCap>,
        _amount: Money,
    ) -> impl Future<Output = Result<(), WalletError<Self::Error>>> + Send {
        core::future::pending()
    }

    fn settle(
        &self,
        _account: &AccountId,
        _request: &RequestId,
        _amount: Money,
    ) -> impl Future<Output = Result<(), WalletError<Self::Error>>> + Send {
        core::future::pending()
    }

    fn release(
        &self,
        _account: &AccountId,
        _request: &RequestId,
    ) -> impl Future<Output = Result<(), WalletError<Self::Error>>> + Send {
        core::future::pending()
    }

    fn held(
        &self,
        _account: &AccountId,
        _request: &RequestId,
    ) -> impl Future<Output = Result<Money, WalletError<Self::Error>>> + Send {
        core::future::pending()
    }

    fn spent(
        &self,
        _key: &KeyId,
        _currency: Currency,
        _since: UnixMillis,
    ) -> impl Future<Output = Result<Money, WalletError<Self::Error>>> + Send {
        core::future::pending()
    }
}

impl Ledger for Stub {
    type Error = ();

    fn record(
        &self,
        _entries: &[Entry],
    ) -> impl Future<Output = Result<(), WalletError<Self::Error>>> + Send {
        core::future::pending()
    }
}

fn drive(future: impl Future + Send) {
    let mut future = pin!(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let _polled = future.as_mut().poll(&mut context);
}

fn rejection_code(reason: BillingRejection) -> u8 {
    match reason {
        BillingRejection::InsufficientBalance => 0,
        BillingRejection::DuplicateRequest => 1,
        BillingRejection::LimitExceeded => 2,
        BillingRejection::CurrencyMismatch => 3,
        BillingRejection::HoldNotFound => 4,
        BillingRejection::SettlementExceedsHold => 5,
        BillingRejection::AlreadySettled => 6,
        BillingRejection::AlreadyReleased => 7,
        BillingRejection::Overflow => 8,
    }
}

#[test]
fn wallet_ledger_and_pricing_signatures_are_usable() {
    let wallet = Stub;
    let account = AccountId(String::new());
    let request = RequestId(String::new());
    let key = KeyId(String::new());
    let amount = Money {
        currency: Currency::Usd,
        minor_units: 0,
    };
    let cap = KeyCap {
        since: UnixMillis(0),
        limit: amount,
    };
    drive(wallet.balance(&account, Currency::Usd));
    drive(wallet.hold(&request, &account, &key, Some(cap), amount));
    drive(wallet.settle(&account, &request, amount));
    drive(wallet.release(&account, &request));
    drive(wallet.held(&account, &request));
    drive(wallet.spent(&key, Currency::Usd, UnixMillis(0)));
    drive(Ledger::record(&wallet, &[]));
    assert!(Pricing::price(&wallet, &()).is_ok());
    assert_eq!(rejection_code(BillingRejection::DuplicateRequest), 1);
    assert!(matches!(
        WalletError::<()>::Rejected(BillingRejection::HoldNotFound),
        WalletError::Rejected(BillingRejection::HoldNotFound)
    ));
    assert!(matches!(
        WalletError::<()>::Storage(()),
        WalletError::Storage(())
    ));
}
