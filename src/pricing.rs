use crate::money::{Currency, Money, MoneyError};
use chrono::{DateTime, Utc};
use serde::de::Error as DeserializeError;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct PricingConfig {
    version: String,
    updated_at: DateTime<Utc>,
    source: String,
    models: BTreeMap<String, ModelPricing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ModelPricing {
    pricing: BTreeMap<Currency, CurrencyPricing>,
    tiered_pricing: Option<BTreeMap<Currency, Vec<TieredPriceConfig>>>,
    cache_pricing: Option<BTreeMap<Currency, CachePricingConfig>>,
    batch_pricing: Option<BTreeMap<Currency, BatchPricingConfig>>,
    voices_pricing: Option<BTreeMap<Currency, VoicesPricingConfig>>,
    video_pricing: Option<BTreeMap<Currency, VideoPricingConfig>>,
    asr_pricing: Option<BTreeMap<Currency, AsrPricingConfig>>,
    realtime_pricing: Option<BTreeMap<Currency, RealtimePricingConfig>>,
    metadata: Option<ModelMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct CurrencyPricing {
    input_price: Money,
    output_price: Money,
    source: Option<String>,
    image_output_price: Option<Money>,
    audio_output_price: Option<Money>,
    music_price: Option<Money>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct TieredPriceConfig {
    start: i64,
    end: Option<i64>,
    input_price: Money,
    output_price: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct CachePricingConfig {
    cache_read_input_price: Money,
    cache_creation_input_price: Option<Money>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct BatchPricingConfig {
    batch_input_price: Money,
    batch_output_price: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct VoicesPricingConfig {
    voices: BTreeMap<String, Money>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct VideoPricingConfig {
    resolutions: BTreeMap<String, Money>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct AsrPricingConfig {
    per_minute: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct RealtimePricingConfig {
    audio_input: Option<Money>,
    audio_output: Option<Money>,
    image_input: Option<Money>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[must_use]
pub struct ModelMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_function_calling: bool,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
}

#[derive(Debug, Error)]
#[must_use]
pub enum PricingError {
    #[error(transparent)]
    Money(#[from] MoneyError),
    #[error("invalid currency code")]
    Currency,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid tier")]
    InvalidTier,
    #[error("mixed pricing document layout")]
    MixedLayout,
}

impl PricingConfig {
    pub fn from_json(json: &str) -> Result<Self, PricingError> {
        let value: Value = serde_json::from_str(json)?;
        let version = value
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("1.0");
        let flat = match detect_layout(&value) {
            Layout::Normalised => false,
            Layout::Flat => true,
            Layout::Ambiguous => version_major(version) >= 7,
            Layout::Mixed => return Err(PricingError::MixedLayout),
        };
        if flat {
            convert_v7(serde_json::from_str(json)?)
        } else {
            convert_v1(serde_json::from_str(json)?)
        }
    }

    pub fn to_json(&self) -> Result<String, PricingError> {
        let mut root = Map::new();
        root.insert("version".to_owned(), Value::String(self.version.clone()));
        root.insert(
            "updated_at".to_owned(),
            serde_json::to_value(self.updated_at)?,
        );
        root.insert("source".to_owned(), Value::String(self.source.clone()));
        let mut models = Map::new();
        for (name, model) in &self.models {
            models.insert(name.clone(), model_value(model)?);
        }
        root.insert("models".to_owned(), Value::Object(models));
        Ok(serde_json::to_string_pretty(&Value::Object(root))?)
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn models(&self) -> &BTreeMap<String, ModelPricing> {
        &self.models
    }
}

impl ModelPricing {
    #[must_use]
    pub fn pricing(&self) -> &BTreeMap<Currency, CurrencyPricing> {
        &self.pricing
    }

    #[must_use]
    pub fn tiered_pricing(&self) -> Option<&BTreeMap<Currency, Vec<TieredPriceConfig>>> {
        self.tiered_pricing.as_ref()
    }

    #[must_use]
    pub fn cache_pricing(&self) -> Option<&BTreeMap<Currency, CachePricingConfig>> {
        self.cache_pricing.as_ref()
    }

    #[must_use]
    pub fn batch_pricing(&self) -> Option<&BTreeMap<Currency, BatchPricingConfig>> {
        self.batch_pricing.as_ref()
    }

    #[must_use]
    pub fn voices_pricing(&self) -> Option<&BTreeMap<Currency, VoicesPricingConfig>> {
        self.voices_pricing.as_ref()
    }

    #[must_use]
    pub fn video_pricing(&self) -> Option<&BTreeMap<Currency, VideoPricingConfig>> {
        self.video_pricing.as_ref()
    }

    #[must_use]
    pub fn asr_pricing(&self) -> Option<&BTreeMap<Currency, AsrPricingConfig>> {
        self.asr_pricing.as_ref()
    }

    #[must_use]
    pub fn realtime_pricing(&self) -> Option<&BTreeMap<Currency, RealtimePricingConfig>> {
        self.realtime_pricing.as_ref()
    }

    #[must_use]
    pub fn metadata(&self) -> Option<&ModelMetadata> {
        self.metadata.as_ref()
    }
}

impl CurrencyPricing {
    pub const fn input_price(&self) -> Money {
        self.input_price
    }

    pub const fn output_price(&self) -> Money {
        self.output_price
    }

    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    #[must_use]
    pub const fn image_output_price(&self) -> Option<Money> {
        self.image_output_price
    }

    #[must_use]
    pub const fn audio_output_price(&self) -> Option<Money> {
        self.audio_output_price
    }

    #[must_use]
    pub const fn music_price(&self) -> Option<Money> {
        self.music_price
    }
}

impl TieredPriceConfig {
    #[must_use]
    pub const fn start(self) -> i64 {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> Option<i64> {
        self.end
    }

    pub const fn input_price(self) -> Money {
        self.input_price
    }

    pub const fn output_price(self) -> Money {
        self.output_price
    }
}

impl CachePricingConfig {
    pub const fn cache_read_input_price(self) -> Money {
        self.cache_read_input_price
    }

    #[must_use]
    pub const fn cache_creation_input_price(self) -> Option<Money> {
        self.cache_creation_input_price
    }
}

impl BatchPricingConfig {
    pub const fn batch_input_price(self) -> Money {
        self.batch_input_price
    }

    pub const fn batch_output_price(self) -> Money {
        self.batch_output_price
    }
}

impl VoicesPricingConfig {
    #[must_use]
    pub fn voices(&self) -> &BTreeMap<String, Money> {
        &self.voices
    }
}

impl VideoPricingConfig {
    #[must_use]
    pub fn resolutions(&self) -> &BTreeMap<String, Money> {
        &self.resolutions
    }
}

impl AsrPricingConfig {
    pub const fn per_minute(self) -> Money {
        self.per_minute
    }
}

impl RealtimePricingConfig {
    #[must_use]
    pub const fn audio_input(self) -> Option<Money> {
        self.audio_input
    }

    #[must_use]
    pub const fn audio_output(self) -> Option<Money> {
        self.audio_output
    }

    #[must_use]
    pub const fn image_input(self) -> Option<Money> {
        self.image_input
    }
}

fn default_true() -> bool {
    true
}

fn default_version() -> String {
    "1.0".to_owned()
}

#[derive(Clone, Copy)]
enum Layout {
    Normalised,
    Flat,
    Ambiguous,
    Mixed,
}

fn version_major(version: &str) -> u32 {
    version
        .split('.')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(1)
}

fn detect_layout(value: &Value) -> Layout {
    const V1_PRICING_KEYS: [&str; 8] = [
        "pricing",
        "tiered_pricing",
        "cache_pricing",
        "batch_pricing",
        "voices_pricing",
        "video_pricing",
        "asr_pricing",
        "realtime_pricing",
    ];
    const V7_KEYS: [&str; 8] = [
        "text", "cache", "batch", "image", "audio", "video", "music", "tiered",
    ];
    let Some(models) = value.get("models").and_then(Value::as_object) else {
        return Layout::Ambiguous;
    };
    let mut saw_v1_pricing = false;
    let mut saw_v1_metadata = false;
    let mut saw_v7 = false;
    for entry in models.values() {
        let Some(fields) = entry.as_object() else {
            continue;
        };
        if V1_PRICING_KEYS.iter().any(|key| fields.contains_key(*key)) {
            saw_v1_pricing = true;
        }
        if fields.contains_key("metadata") {
            saw_v1_metadata = true;
        }
        for (field_name, currency_block) in fields {
            if field_name == "metadata" || V1_PRICING_KEYS.contains(&field_name.as_str()) {
                continue;
            }
            let Some(block) = currency_block.as_object() else {
                continue;
            };
            if V7_KEYS.iter().any(|key| block.contains_key(*key)) {
                saw_v7 = true;
            }
        }
    }
    if saw_v1_pricing && saw_v7 {
        Layout::Mixed
    } else if saw_v7 {
        Layout::Flat
    } else if saw_v1_pricing || saw_v1_metadata {
        Layout::Normalised
    } else {
        Layout::Ambiguous
    }
}

#[derive(Debug)]
struct DecimalAmount(String);

impl DecimalAmount {
    fn text(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DecimalAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        let text = raw.get().trim();
        if text.starts_with('"') {
            let parsed = serde_json::from_str::<String>(text).map_err(DeserializeError::custom)?;
            return Ok(Self(parsed));
        }
        if text == "null" {
            return Err(DeserializeError::custom(MoneyError::InvalidDecimal));
        }
        Ok(Self(text.to_owned()))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV1 {
    #[serde(default = "default_version")]
    version: String,
    updated_at: DateTime<Utc>,
    source: String,
    models: BTreeMap<String, WireModel>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireModel {
    #[serde(default)]
    pricing: BTreeMap<String, WireCurrencyPricing>,
    #[serde(default)]
    tiered_pricing: Option<BTreeMap<String, Vec<WireTier>>>,
    #[serde(default)]
    cache_pricing: Option<BTreeMap<String, WireCache>>,
    #[serde(default)]
    batch_pricing: Option<BTreeMap<String, WireBatch>>,
    #[serde(default)]
    voices_pricing: Option<BTreeMap<String, WireVoices>>,
    #[serde(default)]
    video_pricing: Option<BTreeMap<String, WireVideo>>,
    #[serde(default)]
    asr_pricing: Option<BTreeMap<String, WireAsr>>,
    #[serde(default)]
    realtime_pricing: Option<BTreeMap<String, WireRealtime>>,
    #[serde(default)]
    metadata: Option<ModelMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCurrencyPricing {
    input_price: DecimalAmount,
    output_price: DecimalAmount,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    image_output_price: Option<DecimalAmount>,
    #[serde(default)]
    audio_output_price: Option<DecimalAmount>,
    #[serde(default)]
    music_price: Option<DecimalAmount>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTier {
    tier_start: i64,
    #[serde(default)]
    tier_end: Option<i64>,
    input_price: DecimalAmount,
    output_price: DecimalAmount,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCache {
    cache_read_input_price: DecimalAmount,
    #[serde(default)]
    cache_creation_input_price: Option<DecimalAmount>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBatch {
    batch_input_price: DecimalAmount,
    batch_output_price: DecimalAmount,
}

#[derive(Debug, Deserialize)]
struct WireVoices {
    #[serde(flatten)]
    voices: BTreeMap<String, DecimalAmount>,
}

#[derive(Debug, Deserialize)]
struct WireVideo {
    #[serde(flatten)]
    resolutions: BTreeMap<String, DecimalAmount>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireAsr {
    per_minute: DecimalAmount,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRealtime {
    #[serde(default)]
    audio_input: Option<DecimalAmount>,
    #[serde(default)]
    audio_output: Option<DecimalAmount>,
    #[serde(default)]
    image_input: Option<DecimalAmount>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7 {
    #[serde(default = "default_version")]
    version: String,
    updated_at: DateTime<Utc>,
    source: String,
    models: BTreeMap<String, BTreeMap<String, WireV7Currency>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7Currency {
    text: WireV7Text,
    #[serde(default)]
    cache: Option<WireV7Cache>,
    #[serde(default)]
    batch: Option<WireV7Batch>,
    #[serde(default)]
    image: Option<WireV7Out>,
    #[serde(default)]
    audio: Option<WireV7Out>,
    #[serde(default)]
    music: Option<WireV7Music>,
    #[serde(default)]
    tiered: Option<Vec<WireV7Tier>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7Text {
    #[serde(rename = "in")]
    input: DecimalAmount,
    #[serde(rename = "out")]
    output: DecimalAmount,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7Cache {
    read: DecimalAmount,
    #[serde(default)]
    write: Option<DecimalAmount>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7Batch {
    #[serde(rename = "in")]
    input: DecimalAmount,
    #[serde(rename = "out")]
    output: DecimalAmount,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7Out {
    #[serde(rename = "out")]
    output: DecimalAmount,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7Music {
    per: DecimalAmount,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireV7Tier {
    tier_start: i64,
    #[serde(default)]
    tier_end: Option<i64>,
    #[serde(rename = "in")]
    input: DecimalAmount,
    #[serde(rename = "out")]
    output: DecimalAmount,
}

fn amount(currency: Currency, value: &DecimalAmount) -> Result<Money, PricingError> {
    Ok(Money::parse_decimal(currency, value.text())?)
}

fn optional_amount(
    currency: Currency,
    value: Option<&DecimalAmount>,
) -> Result<Option<Money>, PricingError> {
    match value {
        Some(value) => Ok(Some(amount(currency, value)?)),
        None => Ok(None),
    }
}

fn checked_tier(
    currency: Currency,
    start: i64,
    end: Option<i64>,
    input: &DecimalAmount,
    output: &DecimalAmount,
) -> Result<TieredPriceConfig, PricingError> {
    if start < 0 {
        return Err(PricingError::InvalidTier);
    }
    if let Some(end) = end {
        if end <= start {
            return Err(PricingError::InvalidTier);
        }
    }
    Ok(TieredPriceConfig {
        start,
        end,
        input_price: amount(currency, input)?,
        output_price: amount(currency, output)?,
    })
}

fn insert_currency<T>(map: &mut BTreeMap<Currency, T>, currency: Currency, value: T) {
    let _existing = map.insert(currency, value);
}

fn convert_currency_map<T, U, F>(
    map: BTreeMap<String, T>,
    mut convert: F,
) -> Result<BTreeMap<Currency, U>, PricingError>
where
    F: FnMut(Currency, T) -> Result<U, PricingError>,
{
    let mut out = BTreeMap::new();
    for (key, value) in map {
        let currency = Currency::from_str(&key).map_err(|_| PricingError::Currency)?;
        insert_currency(&mut out, currency, convert(currency, value)?);
    }
    Ok(out)
}

fn convert_optional_map<T, U, F>(
    map: Option<BTreeMap<String, T>>,
    convert: F,
) -> Result<Option<BTreeMap<Currency, U>>, PricingError>
where
    F: FnMut(Currency, T) -> Result<U, PricingError>,
{
    match map {
        Some(map) => Ok(Some(convert_currency_map(map, convert)?)),
        None => Ok(None),
    }
}

fn convert_currency_pricing(
    currency: Currency,
    wire: WireCurrencyPricing,
) -> Result<CurrencyPricing, PricingError> {
    Ok(CurrencyPricing {
        input_price: amount(currency, &wire.input_price)?,
        output_price: amount(currency, &wire.output_price)?,
        source: wire.source,
        image_output_price: optional_amount(currency, wire.image_output_price.as_ref())?,
        audio_output_price: optional_amount(currency, wire.audio_output_price.as_ref())?,
        music_price: optional_amount(currency, wire.music_price.as_ref())?,
    })
}

fn convert_v1(wire: WireV1) -> Result<PricingConfig, PricingError> {
    let mut models = BTreeMap::new();
    for (name, model) in wire.models {
        let pricing = convert_currency_map(model.pricing, convert_currency_pricing)?;
        let tiered_pricing = convert_optional_map(model.tiered_pricing, |currency, tiers| {
            let mut parsed = Vec::new();
            for tier in tiers {
                parsed.push(checked_tier(
                    currency,
                    tier.tier_start,
                    tier.tier_end,
                    &tier.input_price,
                    &tier.output_price,
                )?);
            }
            Ok(parsed)
        })?;
        let cache_pricing = convert_optional_map(model.cache_pricing, |currency, cache| {
            Ok(CachePricingConfig {
                cache_read_input_price: amount(currency, &cache.cache_read_input_price)?,
                cache_creation_input_price: optional_amount(
                    currency,
                    cache.cache_creation_input_price.as_ref(),
                )?,
            })
        })?;
        let batch_pricing = convert_optional_map(model.batch_pricing, |currency, batch| {
            Ok(BatchPricingConfig {
                batch_input_price: amount(currency, &batch.batch_input_price)?,
                batch_output_price: amount(currency, &batch.batch_output_price)?,
            })
        })?;
        let voices_pricing = convert_optional_map(model.voices_pricing, |currency, voices| {
            Ok(VoicesPricingConfig {
                voices: money_map(currency, voices.voices)?,
            })
        })?;
        let video_pricing = convert_optional_map(model.video_pricing, |currency, video| {
            Ok(VideoPricingConfig {
                resolutions: money_map(currency, video.resolutions)?,
            })
        })?;
        let asr_pricing = convert_optional_map(model.asr_pricing, |currency, asr| {
            Ok(AsrPricingConfig {
                per_minute: amount(currency, &asr.per_minute)?,
            })
        })?;
        let realtime_pricing =
            convert_optional_map(model.realtime_pricing, |currency, realtime| {
                Ok(RealtimePricingConfig {
                    audio_input: optional_amount(currency, realtime.audio_input.as_ref())?,
                    audio_output: optional_amount(currency, realtime.audio_output.as_ref())?,
                    image_input: optional_amount(currency, realtime.image_input.as_ref())?,
                })
            })?;
        insert_model(
            &mut models,
            name,
            ModelPricing {
                pricing,
                tiered_pricing,
                cache_pricing,
                batch_pricing,
                voices_pricing,
                video_pricing,
                asr_pricing,
                realtime_pricing,
                metadata: model.metadata,
            },
        );
    }
    Ok(PricingConfig {
        version: wire.version,
        updated_at: wire.updated_at,
        source: wire.source,
        models,
    })
}

fn money_map(
    currency: Currency,
    amounts: BTreeMap<String, DecimalAmount>,
) -> Result<BTreeMap<String, Money>, PricingError> {
    let mut out = BTreeMap::new();
    for (name, value) in amounts {
        let _existing = out.insert(name, amount(currency, &value)?);
    }
    Ok(out)
}

fn insert_model(models: &mut BTreeMap<String, ModelPricing>, name: String, model: ModelPricing) {
    let _existing = models.insert(name, model);
}

fn convert_v7(wire: WireV7) -> Result<PricingConfig, PricingError> {
    let mut models = BTreeMap::new();
    for (name, currencies) in wire.models {
        let mut pricing = BTreeMap::new();
        let mut cache_pricing = BTreeMap::new();
        let mut batch_pricing = BTreeMap::new();
        let mut tiered_pricing = BTreeMap::new();
        for (key, block) in currencies {
            let currency = Currency::from_str(&key).map_err(|_| PricingError::Currency)?;
            insert_currency(
                &mut pricing,
                currency,
                CurrencyPricing {
                    input_price: amount(currency, &block.text.input)?,
                    output_price: amount(currency, &block.text.output)?,
                    source: None,
                    image_output_price: match &block.image {
                        Some(image) => Some(amount(currency, &image.output)?),
                        None => None,
                    },
                    audio_output_price: match &block.audio {
                        Some(audio) => Some(amount(currency, &audio.output)?),
                        None => None,
                    },
                    music_price: match &block.music {
                        Some(music) => Some(amount(currency, &music.per)?),
                        None => None,
                    },
                },
            );
            if let Some(cache) = &block.cache {
                insert_currency(
                    &mut cache_pricing,
                    currency,
                    CachePricingConfig {
                        cache_read_input_price: amount(currency, &cache.read)?,
                        cache_creation_input_price: optional_amount(
                            currency,
                            cache.write.as_ref(),
                        )?,
                    },
                );
            }
            if let Some(batch) = &block.batch {
                insert_currency(
                    &mut batch_pricing,
                    currency,
                    BatchPricingConfig {
                        batch_input_price: amount(currency, &batch.input)?,
                        batch_output_price: amount(currency, &batch.output)?,
                    },
                );
            }
            if let Some(tiers) = &block.tiered {
                let mut parsed = Vec::new();
                for tier in tiers {
                    parsed.push(checked_tier(
                        currency,
                        tier.tier_start,
                        tier.tier_end,
                        &tier.input,
                        &tier.output,
                    )?);
                }
                insert_currency(&mut tiered_pricing, currency, parsed);
            }
        }
        insert_model(
            &mut models,
            name,
            ModelPricing {
                pricing,
                tiered_pricing: some_map(tiered_pricing),
                cache_pricing: some_map(cache_pricing),
                batch_pricing: some_map(batch_pricing),
                voices_pricing: None,
                video_pricing: None,
                asr_pricing: None,
                realtime_pricing: None,
                metadata: None,
            },
        );
    }
    Ok(PricingConfig {
        version: wire.version,
        updated_at: wire.updated_at,
        source: wire.source,
        models,
    })
}

fn some_map<T>(map: BTreeMap<Currency, T>) -> Option<BTreeMap<Currency, T>> {
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

fn money_json(money: Money) -> Value {
    Value::String(money.to_decimal_string())
}

fn insert_money(map: &mut Map<String, Value>, key: &str, money: Option<Money>) {
    if let Some(money) = money {
        map.insert(key.to_owned(), money_json(money));
    }
}

fn model_value(model: &ModelPricing) -> Result<Value, PricingError> {
    let mut object = Map::new();
    object.insert(
        "pricing".to_owned(),
        currency_object(&model.pricing, currency_pricing_value),
    );
    insert_optional_map(
        &mut object,
        "tiered_pricing",
        model.tiered_pricing.as_ref(),
        |tiers| tier_list_value(tiers),
    );
    insert_optional_map(
        &mut object,
        "cache_pricing",
        model.cache_pricing.as_ref(),
        cache_value,
    );
    insert_optional_map(
        &mut object,
        "batch_pricing",
        model.batch_pricing.as_ref(),
        batch_value,
    );
    insert_optional_map(
        &mut object,
        "voices_pricing",
        model.voices_pricing.as_ref(),
        voices_value,
    );
    insert_optional_map(
        &mut object,
        "video_pricing",
        model.video_pricing.as_ref(),
        video_value,
    );
    insert_optional_map(
        &mut object,
        "asr_pricing",
        model.asr_pricing.as_ref(),
        asr_value,
    );
    insert_optional_map(
        &mut object,
        "realtime_pricing",
        model.realtime_pricing.as_ref(),
        realtime_value,
    );
    if let Some(metadata) = &model.metadata {
        object.insert("metadata".to_owned(), serde_json::to_value(metadata)?);
    }
    Ok(Value::Object(object))
}

fn currency_object<T, F>(map: &BTreeMap<Currency, T>, mut value_of: F) -> Value
where
    F: FnMut(&T) -> Value,
{
    let mut object = Map::new();
    for (currency, value) in map {
        object.insert(currency.code().to_owned(), value_of(value));
    }
    Value::Object(object)
}

fn insert_optional_map<T, F>(
    object: &mut Map<String, Value>,
    key: &str,
    map: Option<&BTreeMap<Currency, T>>,
    value_of: F,
) where
    F: FnMut(&T) -> Value,
{
    if let Some(map) = map {
        object.insert(key.to_owned(), currency_object(map, value_of));
    }
}

fn currency_pricing_value(pricing: &CurrencyPricing) -> Value {
    let mut object = Map::new();
    object.insert("input_price".to_owned(), money_json(pricing.input_price));
    object.insert("output_price".to_owned(), money_json(pricing.output_price));
    if let Some(source) = pricing.source() {
        object.insert("source".to_owned(), Value::String(source.to_owned()));
    }
    insert_money(
        &mut object,
        "image_output_price",
        pricing.image_output_price,
    );
    insert_money(
        &mut object,
        "audio_output_price",
        pricing.audio_output_price,
    );
    insert_money(&mut object, "music_price", pricing.music_price);
    Value::Object(object)
}

fn tier_list_value(tiers: &[TieredPriceConfig]) -> Value {
    let mut list = Vec::new();
    for tier in tiers {
        let mut object = Map::new();
        object.insert("tier_start".to_owned(), Value::from(tier.start));
        if let Some(end) = tier.end {
            object.insert("tier_end".to_owned(), Value::from(end));
        }
        object.insert("input_price".to_owned(), money_json(tier.input_price));
        object.insert("output_price".to_owned(), money_json(tier.output_price));
        list.push(Value::Object(object));
    }
    Value::Array(list)
}

fn cache_value(cache: &CachePricingConfig) -> Value {
    let mut object = Map::new();
    object.insert(
        "cache_read_input_price".to_owned(),
        money_json(cache.cache_read_input_price),
    );
    insert_money(
        &mut object,
        "cache_creation_input_price",
        cache.cache_creation_input_price,
    );
    Value::Object(object)
}

fn batch_value(batch: &BatchPricingConfig) -> Value {
    let mut object = Map::new();
    object.insert(
        "batch_input_price".to_owned(),
        money_json(batch.batch_input_price),
    );
    object.insert(
        "batch_output_price".to_owned(),
        money_json(batch.batch_output_price),
    );
    Value::Object(object)
}

fn voices_value(voices: &VoicesPricingConfig) -> Value {
    named_money_object(&voices.voices)
}

fn video_value(video: &VideoPricingConfig) -> Value {
    named_money_object(&video.resolutions)
}

fn named_money_object(map: &BTreeMap<String, Money>) -> Value {
    let mut object = Map::new();
    for (name, money) in map {
        object.insert(name.clone(), money_json(*money));
    }
    Value::Object(object)
}

fn asr_value(asr: &AsrPricingConfig) -> Value {
    let mut object = Map::new();
    object.insert("per_minute".to_owned(), money_json(asr.per_minute));
    Value::Object(object)
}

fn realtime_value(realtime: &RealtimePricingConfig) -> Value {
    let mut object = Map::new();
    insert_money(&mut object, "audio_input", realtime.audio_input);
    insert_money(&mut object, "audio_output", realtime.audio_output);
    insert_money(&mut object, "image_input", realtime.image_input);
    Value::Object(object)
}
