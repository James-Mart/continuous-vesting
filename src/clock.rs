use std::cell::RefCell;

#[derive(Default, Clone, Copy)]
pub struct TestClock {
    t: u64,
}
impl TestClock {
    pub fn new() -> Self {
        Self { t: 0 }
    }
    pub fn now(&self) -> u64 {
        self.t
    }
    pub fn wait(&mut self, secs: u64) {
        self.t = self.t.saturating_add(secs);
    }
}

thread_local! {
    static TEST_CLOCK: RefCell<TestClock> = RefCell::new(TestClock::new());
}

pub fn now() -> u64 {
    TEST_CLOCK.with(|c| c.borrow().now())
}
pub fn wait(secs: u64) {
    TEST_CLOCK.with(|c| c.borrow_mut().wait(secs));
}
pub fn clock_reset(ts: u64) {
    TEST_CLOCK.with(|c| c.borrow_mut().t = ts);
}
