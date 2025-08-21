#![allow(dead_code)]

mod clock;
pub use clock::*;

#[derive(Default)]
pub struct TokenStream {
    decay_rate_per_second: f64,
    total_deposited: u128, // Cumulative
    total_claimed: u128,   // Cumulative

    last_update_principal: u128,
    last_update_timestamp: u64,
}

impl TokenStream {
    //const SECONDS_PER_DAY: f64 = 86_400.0;

    fn decay_rate_from_half_life(days: f64) -> f64 {
        std::f64::consts::LN_2 / (days) // To use seconds: (days * Self::SECONDS_PER_DAY)
    }

    /// Construct with half-life (in days) and automatically compute the rate.
    pub fn new_from_half_life(days: f64) -> Self {
        Self::new(Self::decay_rate_from_half_life(days))
    }

    pub fn new(decay_rate_per_second: f64) -> Self {
        Self {
            decay_rate_per_second,
            ..Default::default()
        }
    }

    /// Change half-life. Settles first to preserve continuity.
    pub fn set_half_life(&mut self, days: f64) {
        self.settle();
        self.decay_rate_per_second = Self::decay_rate_from_half_life(days);
    }

    /// Total vested since inception, regardless of whether it was claimed.
    pub fn total_vested(&self) -> u128 {
        self.total_deposited
            .saturating_sub(self.balance_still_vesting())
    }

    /// Total amount claimed since inception.
    pub fn total_claimed(&self) -> u128 {
        self.total_claimed
    }

    /// Amount you could claim *right now*.
    pub fn balance_claimable(&self) -> u128 {
        let rel = self.total_vested();
        rel.saturating_sub(self.total_claimed)
    }

    /// The amount that has yet to fully vest (rounds down). Continuously decays.
    pub fn balance_still_vesting(&self) -> u128 {
        if self.last_update_principal == 0 {
            return 0;
        }
        let dt = now().saturating_sub(self.last_update_timestamp) as f64;
        let factor = (-self.decay_rate_per_second * dt).exp();
        ((self.last_update_principal as f64) * factor).floor() as u128
    }

    /// Snapshot current remaining and reset timestamp. Returns the current amount still vesting.
    pub fn settle(&mut self) -> u128 {
        let p_now = self.balance_still_vesting();
        self.last_update_principal = p_now;
        self.last_update_timestamp = now();
        p_now
    }

    /// Deposit `amount` into the bucket
    pub fn deposit(&mut self, amount: u128) {
        let p_now = self.settle();
        self.last_update_principal = p_now.saturating_add(amount);
        self.total_deposited = self.total_deposited.saturating_add(amount);
    }

    /// Claim everything currently claimable; returns the claimed amount.
    pub fn claim(&mut self) -> u128 {
        let p_now = self.settle();
        let amt = self
            .total_deposited
            .saturating_sub(self.total_claimed)
            .saturating_sub(p_now); // = (vested - already claimed)
        self.total_claimed = self.total_claimed.saturating_add(amt);
        amt
    }

    /// Total still unclaimed.
    pub fn unclaimed_total(&self) -> u128 {
        self.total_deposited.saturating_sub(self.total_claimed)
    }
}
