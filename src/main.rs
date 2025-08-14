#![allow(dead_code)]

use std::cell::RefCell;

#[derive(Default, Clone, Copy)]
struct TestClock {
    t: u64,
}
impl TestClock {
    fn new() -> Self {
        Self { t: 0 }
    }
    fn now(&self) -> u64 {
        self.t
    }
    fn wait(&mut self, secs: u64) {
        self.t = self.t.saturating_add(secs);
    }
}

thread_local! {
    static TEST_CLOCK: RefCell<TestClock> = RefCell::new(TestClock::new());
}

fn now() -> u64 {
    TEST_CLOCK.with(|c| c.borrow().now())
}
pub fn wait(secs: u64) {
    TEST_CLOCK.with(|c| c.borrow_mut().wait(secs));
}
pub fn clock_reset(ts: u64) {
    TEST_CLOCK.with(|c| c.borrow_mut().t = ts);
}

#[derive(Default)]
struct RewardBucket {
    decay_rate_per_second: f64,
    total_deposited: u128, // Cumulative
    total_claimed: u128,   // Cumulative

    last_update_principal: u128,
    last_update_timestamp: u64,
}

impl RewardBucket {
    const SECONDS_PER_DAY: f64 = 86_400.0;

    fn decay_rate_from_half_life(days: f64) -> f64 {
        std::f64::consts::LN_2 / (days * Self::SECONDS_PER_DAY)
    }

    /// Construct with half-life (in days) and automatically compute the rate.
    fn new_from_half_life(days: f64) -> Self {
        Self::new(Self::decay_rate_from_half_life(days))
    }

    fn new(decay_rate_per_second: f64) -> Self {
        Self {
            decay_rate_per_second,
            ..Default::default()
        }
    }

    /// Change half-life. Settles first to preserve continuity.
    fn set_half_life(&mut self, days: f64) {
        self.settle();
        self.decay_rate_per_second = Self::decay_rate_from_half_life(days);
    }

    /// Total vested since inception, regardless of whether it was claimed.
    fn total_vested(&self) -> u128 {
        self.total_deposited
            .saturating_sub(self.balance_still_vesting())
    }

    /// Total amount claimed since inception.
    fn total_claimed(&self) -> u128 {
        self.total_claimed
    }

    /// Amount you could claim *right now*.
    fn balance_claimable(&self) -> u128 {
        let rel = self.total_vested();
        rel.saturating_sub(self.total_claimed)
    }

    /// The amount that has yet to fully vest (rounds down). Continuously decays.
    fn balance_still_vesting(&self) -> u128 {
        if self.last_update_principal == 0 {
            return 0;
        }
        let dt = now().saturating_sub(self.last_update_timestamp) as f64;
        let factor = (-self.decay_rate_per_second * dt).exp();
        ((self.last_update_principal as f64) * factor).floor() as u128
    }

    /// Snapshot current remaining and reset timestamp. Returns the current amount still vesting.
    fn settle(&mut self) -> u128 {
        let p_now = self.balance_still_vesting();
        self.last_update_principal = p_now;
        self.last_update_timestamp = now();
        p_now
    }

    /// Deposit `amount` into the bucket
    fn deposit(&mut self, amount: u128) {
        let p_now = self.settle();
        self.last_update_principal = p_now.saturating_add(amount);
        self.total_deposited = self.total_deposited.saturating_add(amount);
    }

    /// Claim everything currently claimable; returns the claimed amount.
    fn claim(&mut self) -> u128 {
        let p_now = self.settle();
        let amt = self
            .total_deposited
            .saturating_sub(self.total_claimed)
            .saturating_sub(p_now); // = (vested - already claimed)
        self.total_claimed = self.total_claimed.saturating_add(amt);
        amt
    }

    /// Total still unclaimed.
    fn unclaimed_total(&self) -> u128 {
        self.total_deposited.saturating_sub(self.total_claimed)
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal_after(p: u128, secs: u64, lambda: f64) -> u128 {
        ((p as f64) * (-lambda * secs as f64).exp()).floor() as u128
    }
    fn vested_after(p: u128, secs: u64, lambda: f64) -> u128 {
        p - principal_after(p, secs, lambda)
    }

    #[test]
    fn continuous_basic() {
        clock_reset(0);
        let rate = 0.01; // 1% per second
        let mut b = RewardBucket::new(rate);

        b.deposit(100);
        wait(10);

        let expected = vested_after(100, 10, rate);

        let claimable = b.balance_claimable();
        assert_eq!(claimable, expected); // floor(100 - 100*e^-0.1) = 10
        let still_vesting = b.balance_still_vesting();
        println!(
            "Deposited 100, wait 10s, claimable = {}, still vesting = {}",
            claimable, still_vesting
        );

        assert_eq!(still_vesting, principal_after(100, 10, rate) as u128);
    }

    #[test]
    fn deposit_preserves_withdrawable() {
        clock_reset(0);
        let rate = 0.01;
        let mut b = RewardBucket::new(rate);

        b.deposit(100);
        wait(10);
        let w0 = b.balance_claimable(); // should be 10
        assert_eq!(w0, 10);
        println!("Deposited 100, waited 10s, claimable=10");

        b.deposit(100); // rebase principal; claimable unchanged
        assert_eq!(b.balance_claimable(), w0);
        println!("Deposited another 100, claimable is still 10");

        wait(10);
        let expected_extra = vested_after(190, 10, rate); // 19
        assert_eq!(expected_extra, 19);

        assert_eq!(b.balance_claimable(), (10 + 19) as u128);
        println!("Waited another 10s, claimable=29")
    }

    #[test]
    fn withdraw_side_effects_and_conservation() {
        clock_reset(0);
        let rate = 0.01;
        let mut b = RewardBucket::new(rate);

        b.deposit(200);
        wait(20);
        let claim = b.balance_claimable();
        let claimed = b.claim();
        assert_eq!(claimed, claim); // withdraw pays exactly claimable
        assert_eq!(b.balance_claimable(), 0); // now nothing claimable
        assert_eq!(b.balance_still_vesting(), 200 - claimed);
        assert_eq!(b.total_vested(), claimed);
        assert_eq!(b.total_claimed(), claimed);
    }

    #[test]
    fn time_split_equivalence() {
        clock_reset(0);
        let rate = 0.01;
        let mut b = RewardBucket::new(rate);

        b.deposit(100);

        wait(5);
        let claimed5_1 = b.claim();
        wait(5);
        let claimed5_2 = b.claim();

        let expected10 = vested_after(100, 10, rate);
        assert_eq!(claimed5_1 + claimed5_2, expected10);
        println!("(Wait 5 sec, claim, then wait another 5 sec, claim) == (wait 10 sec, claim)");
    }

    #[test]
    fn dust_eventually_rounds_to_zero() {
        clock_reset(0);
        let rate = 0.01;
        let mut b = RewardBucket::new(rate);

        b.deposit(100);

        // Wait long enough so floor(100 * e^{-λt}) == 0
        let secs = (((100.0f64 + 1.0).ln() / rate).ceil()) as u64;
        println!("Waiting {} seconds", secs);
        wait(secs);

        assert_eq!(b.balance_still_vesting(), 0);
        let claimed = b.claim();
        assert_eq!(claimed, 100);
        assert_eq!(b.balance_claimable(), 0);
    }
}
