use core::future::Future;
use core::pin::pin;
use core::task::{Context, Waker};

use kernel::model::{AccountId, CreditId, Currency, KeyId, ModelId, Money, RequestId, UnixMillis};

use billing::errors::{BillingRejection, WalletError};
use billing::model::{Direction, Entry, EntryRef, KeyCap, LedgerAccount, ModelUsage, TokenUsage};
use billing::ports::{Ledger, Pricing, Wallet};

struct Stub;

impl Pricing for Stub {
    type Usage = ModelUsage;
    type Error = ();

    fn price(&self, usage: &Self::Usage) -> Result<Money, Self::Error> {
        if usage.model.0.is_empty() {
            return Err(());
        }
        Ok(Money {
            currency: Currency::Usd,
            minor_units: usage.tokens.input_tokens
                + usage.tokens.cached_input_tokens
                + usage.tokens.output_tokens,
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
        _account: &AccountId,
        _request: &RequestId,
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
        BillingRejection::Unbalanced => 8,
        BillingRejection::Overflow => 9,
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
    drive(wallet.hold(&account, &request, &key, Some(cap), amount));
    drive(wallet.settle(&account, &request, amount));
    drive(wallet.release(&account, &request));
    drive(wallet.held(&account, &request));
    drive(wallet.spent(&key, Currency::Usd, UnixMillis(0)));
    drive(Ledger::record(&wallet, &[]));
    let usage = ModelUsage {
        model: ModelId(String::from("gpt-4o-mini")),
        tokens: TokenUsage {
            input_tokens: 1,
            cached_input_tokens: 2,
            output_tokens: 4,
        },
    };
    assert_eq!(
        Pricing::price(&wallet, &usage),
        Ok(Money {
            currency: Currency::Usd,
            minor_units: 7,
        })
    );
    let unpriced = ModelUsage {
        model: ModelId(String::new()),
        ..usage
    };
    assert_eq!(Pricing::price(&wallet, &unpriced), Err(()));
    assert_ne!(
        ModelId(String::from("gpt-4o")),
        ModelId(String::from("GPT-4o"))
    );
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

fn ledger_account_code(account: &LedgerAccount) -> u8 {
    match account {
        LedgerAccount::Available(_) => 0,
        LedgerAccount::Held(_) => 1,
        LedgerAccount::Revenue => 2,
        LedgerAccount::Funding => 3,
    }
}

fn reference_code(reference: &EntryRef) -> u8 {
    match reference {
        EntryRef::Request(_) => 0,
        EntryRef::Credit(_) => 1,
    }
}

#[test]
fn credit_debits_funding_and_credits_available() {
    let amount = Money {
        currency: Currency::Usd,
        minor_units: 500,
    };
    let customer = AccountId(String::from("a"));
    let funding = Entry {
        reference: EntryRef::Credit(CreditId(String::from("c-1"))),
        account: LedgerAccount::Funding,
        key: None,
        direction: Direction::Debit,
        amount,
        at: UnixMillis(1),
    };
    let available = Entry {
        account: LedgerAccount::Available(customer.clone()),
        direction: Direction::Credit,
        ..funding.clone()
    };
    assert_eq!(ledger_account_code(&funding.account), 3);
    assert_eq!(ledger_account_code(&available.account), 0);
    assert_ne!(funding.account, LedgerAccount::Revenue);
    assert_ne!(funding.account, LedgerAccount::Available(customer));
    assert_eq!(reference_code(&funding.reference), 1);
    assert_eq!(funding.reference, available.reference);
    assert_eq!(funding.amount, available.amount);
    assert_ne!(funding.direction, available.direction);
    drive(Ledger::record(&Stub, &[funding, available]));
}

#[test]
fn credit_and_request_references_never_collide() {
    let request = EntryRef::Request(RequestId(String::from("x")));
    let credit = EntryRef::Credit(CreditId(String::from("x")));
    assert_eq!(reference_code(&request), 0);
    assert_eq!(reference_code(&credit), 1);
    assert_ne!(request, credit);
}
