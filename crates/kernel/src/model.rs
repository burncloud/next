pub const MINOR_UNITS_PER_MAJOR: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Currency {
    Usd,
    Cny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Money {
    pub currency: Currency,
    pub minor_units: u64,
}
