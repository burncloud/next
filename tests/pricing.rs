use next::money::{Currency, MoneyError};
use next::pricing::{PricingConfig, PricingError};
use serde_json::Value;

fn document(models: &str) -> String {
    format!(
        r#"{{"version":"7.0","updated_at":"2026-03-29T00:00:00Z","source":"test","models":{models}}}"#
    )
}

fn input_minor(config: &PricingConfig, model: &str, currency: Currency) -> i64 {
    config
        .models()
        .get(model)
        .unwrap()
        .pricing()
        .get(&currency)
        .unwrap()
        .input_price()
        .minor_units()
}

#[test]
fn reads_v1_numbers_and_decimal_strings_as_nanos() {
    let numbers = r#"{
        "version": "1.0",
        "updated_at": "2026-03-29T00:00:00Z",
        "source": "fixture",
        "models": {
            "gpt-4o": {
                "pricing": {
                    "USD": { "input_price": 2.5, "output_price": 10 }
                }
            }
        }
    }"#;
    let strings = numbers.replace("2.5", "\"2.5\"").replace("10", "\"10\"");
    let from_numbers = PricingConfig::from_json(numbers).unwrap();
    let from_strings = PricingConfig::from_json(&strings).unwrap();
    assert_eq!(
        input_minor(&from_numbers, "gpt-4o", Currency::Usd),
        2_500_000_000
    );
    assert_eq!(
        from_numbers
            .models()
            .get("gpt-4o")
            .unwrap()
            .pricing()
            .get(&Currency::Usd)
            .unwrap()
            .output_price()
            .minor_units(),
        10_000_000_000
    );
    assert_eq!(
        input_minor(&from_numbers, "gpt-4o", Currency::Usd),
        input_minor(&from_strings, "gpt-4o", Currency::Usd)
    );
}

#[test]
fn reads_v7_sections_and_normalises_them() {
    let json = document(
        r#"{"gemini-flash":{"USD":{"text":{"in":0.075,"out":0.3},"cache":{"read":0.0075,"write":0.09375},"batch":{"in":0.0375,"out":0.15},"image":{"out":0.002},"audio":{"out":0.6},"music":{"per":0.05},"tiered":[{"tier_start":0,"tier_end":200000,"in":0.075,"out":0.3},{"tier_start":200000,"in":0.15,"out":0.6}]},"CNY":{"text":{"in":1,"out":2}}}}"#,
    );
    let config = PricingConfig::from_json(&json).unwrap();
    assert_eq!(config.version(), "7.0");
    assert_eq!(
        input_minor(&config, "gemini-flash", Currency::Usd),
        75_000_000
    );
    assert_eq!(
        input_minor(&config, "gemini-flash", Currency::Cny),
        1_000_000_000
    );
    let model = config.models().get("gemini-flash").unwrap();
    let usd = model.pricing().get(&Currency::Usd).unwrap();
    assert_eq!(usd.image_output_price().unwrap().minor_units(), 2_000_000);
    assert_eq!(usd.audio_output_price().unwrap().minor_units(), 600_000_000);
    assert_eq!(usd.music_price().unwrap().minor_units(), 50_000_000);
    let cache = model.cache_pricing().unwrap().get(&Currency::Usd).unwrap();
    assert_eq!(cache.cache_read_input_price().minor_units(), 7_500_000);
    assert_eq!(
        cache.cache_creation_input_price().unwrap().minor_units(),
        93_750_000
    );
    let tiers = model.tiered_pricing().unwrap().get(&Currency::Usd).unwrap();
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers[1].end(), None);
    assert_eq!(tiers[0].end(), Some(200_000));

    let encoded = config.to_json().unwrap();
    let again = PricingConfig::from_json(&encoded).unwrap();
    assert_eq!(again.version(), "7.0");
    assert_eq!(
        input_minor(&again, "gemini-flash", Currency::Usd),
        input_minor(&config, "gemini-flash", Currency::Usd)
    );
    let value: Value = serde_json::from_str(&encoded).unwrap();
    assert!(value["models"]["gemini-flash"].get("pricing").is_some());
    assert!(value["models"]["gemini-flash"].get("USD").is_none());
    assert!(value["models"]["gemini-flash"]["pricing"]["USD"]["input_price"].is_string());
}

#[test]
fn v7_rejects_unknown_and_previously_dropped_fields() {
    let image_out = document(
        r#"{"m":{"USD":{"text":{"in":1,"out":2},"batch":{"in":1,"out":2,"image_out":3}}}}"#,
    );
    assert!(PricingConfig::from_json(&image_out).is_err());

    let video = document(r#"{"m":{"USD":{"text":{"in":0,"out":0},"video":{"sec":0.025}}}}"#);
    assert!(PricingConfig::from_json(&video).is_err());

    let extra = document(r#"{"m":{"USD":{"text":{"in":1,"out":2,"extra":1}}}}"#);
    assert!(PricingConfig::from_json(&extra).is_err());

    let missing_text = document(r#"{"m":{"USD":{"cache":{"read":1}}}}"#);
    assert!(PricingConfig::from_json(&missing_text).is_err());

    let missing_out = document(r#"{"m":{"USD":{"text":{"in":1}}}}"#);
    assert!(PricingConfig::from_json(&missing_out).is_err());
}

#[test]
fn rejects_lowercase_currency_negative_amounts_and_bad_tiers() {
    let lower = document(r#"{"m":{"usd":{"text":{"in":1,"out":2}}}}"#);
    assert!(matches!(
        PricingConfig::from_json(&lower),
        Err(PricingError::Currency)
    ));

    let negative = document(r#"{"m":{"USD":{"text":{"in":-1,"out":2}}}}"#);
    assert!(matches!(
        PricingConfig::from_json(&negative),
        Err(PricingError::Money(MoneyError::Negative))
    ));

    let nan = document(r#"{"m":{"USD":{"text":{"in":"NaN","out":1}}}}"#);
    assert!(matches!(
        PricingConfig::from_json(&nan),
        Err(PricingError::Money(MoneyError::NotANumber))
    ));

    let infinite = document(r#"{"m":{"USD":{"text":{"in":"Infinity","out":1}}}}"#);
    assert!(matches!(
        PricingConfig::from_json(&infinite),
        Err(PricingError::Money(MoneyError::Infinite))
    ));

    let tied = document(
        r#"{"m":{"USD":{"text":{"in":1,"out":2},"tiered":[{"tier_start":10,"tier_end":10,"in":1,"out":2}]}}}"#,
    );
    assert!(matches!(
        PricingConfig::from_json(&tied),
        Err(PricingError::InvalidTier)
    ));

    let negative_start = document(
        r#"{"m":{"USD":{"text":{"in":1,"out":2},"tiered":[{"tier_start":-1,"in":1,"out":2}]}}}"#,
    );
    assert!(matches!(
        PricingConfig::from_json(&negative_start),
        Err(PricingError::InvalidTier)
    ));
}

#[test]
fn rejects_mixed_layouts_and_keeps_streaming_default() {
    let mixed = r#"{
        "version": "1.0",
        "updated_at": "2026-03-29T00:00:00Z",
        "source": "test",
        "models": {
            "m": {
                "pricing": { "USD": { "input_price": 1, "output_price": 2 } },
                "USD": { "text": { "in": 1, "out": 2 } }
            }
        }
    }"#;
    assert!(matches!(
        PricingConfig::from_json(mixed),
        Err(PricingError::MixedLayout)
    ));

    let metadata = r#"{
        "version": "1.0",
        "updated_at": "2026-03-29T00:00:00Z",
        "source": "test",
        "models": {
            "m": {
                "pricing": { "USD": { "input_price": 1, "output_price": 2 } },
                "metadata": { "context_window": 128000 }
            }
        }
    }"#;
    let config = PricingConfig::from_json(metadata).unwrap();
    let metadata = config.models().get("m").unwrap().metadata().unwrap();
    assert!(metadata.supports_streaming);
    assert!(!metadata.supports_vision);
    assert_eq!(metadata.context_window, Some(128_000));
}
