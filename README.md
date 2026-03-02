# ws2812-esp32s3-ulp-rs

This is a working 'bit-banged' driver for WS2812B leds, using the RISCV ULP core of an ESP32-S3.
I recommend compiling with the 's' optimisation level, for space.

After uploading this, I realised that most people won't have a 'DelayNs'-compatible 'TIMER' object to pass to this Ws2812 driver constructor.
Here's the general vibe of how to make one of those, yourself. You will need to include these two files as modules, in your `main.rs` file.
Most of 'time.rs' is ripped from `embassy` IIRC. Or maybe esp-hal. I've forgotten.

```rust
// delay.rs
use embedded_hal::delay::DelayNs;
use time::{Duration,Instant,Rate};

// For esp32s3 ULP RISCV
pub const CPU_CLOCK: u32 = 17_500_000;

// this is important, not doing this may cause stackoverflow
#[inline(always)]
pub fn cycles() -> u32 {
    let mut cycles: u32;
    unsafe {
        core::arch::asm!(
            "rdcycle {cycles}",
            cycles = out(reg) cycles,
        )
    }
    cycles
}

/// WARN: This is limited to u32 precision and will not work for large delay values.
#[inline(always)] // this is important, not doing this may cause stackoverflow
pub fn delay_cycles_u32(n: u32) {
    let t0 = cycles();
    while cycles().wrapping_sub(t0) <= n {}
}

#[inline(always)] // this is important, not doing this may cause stackoverflow
pub fn delay_cycles(n: u64) {
    if n < u32::MAX as u64 {
        delay_cycles_u32(n as u32);
    } else {
        let mut remain = n;
        while remain > 0 {
            let take = remain.min(u32::MAX as u64);
            delay_cycles_u32(take as u32);
            remain -= take;
        }
    }
}

// A blocking delay utility.
pub struct Timer {}

impl Timer {
    pub fn new() -> Self {
        Timer {}
    }
}

impl DelayNs for Timer {
    fn delay_ns(&mut self, ns: u32) {
        Timer::after(Duration::from_nanos(ns as u64))
    }

    fn delay_us(&mut self, us: u32) {
        Timer::after(Duration::from_micros(us as u64))
    }

    fn delay_ms(&mut self, ms: u32) {
        Timer::after(Duration::from_millis(ms as u64))
    }
}
```


```rust
/// time.rs
use delay::CPU_CLOCK;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};

type InnerRate = fugit::Rate<u32, 1, 1>;
type InnerInstant = fugit::Instant<u64, 1, CPU_CLOCK>;
type InnerDuration = fugit::Duration<u64, 1, CPU_CLOCK>;

/// Represents a rate or frequency of events.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rate(InnerRate);

impl core::hash::Hash for Rate {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_hz().hash(state);
    }
}

impl Display for Rate {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} Hz", self.as_hz())
    }
}

impl Debug for Rate {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Rate({} Hz)", self.as_hz())
    }
}

impl Rate {
    /// Shorthand for creating a rate which represents hertz.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Rate;
    /// let rate = Rate::from_hz(1000);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_hz(val: u32) -> Self {
        Self(InnerRate::Hz(val))
    }

    /// Shorthand for creating a rate which represents kilohertz.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Rate;
    /// let rate = Rate::from_khz(1000);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_khz(val: u32) -> Self {
        Self(InnerRate::kHz(val))
    }

    /// Shorthand for creating a rate which represents megahertz.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Rate;
    /// let rate = Rate::from_mhz(1000);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_mhz(val: u32) -> Self {
        Self(InnerRate::MHz(val))
    }

    /// Convert the `Rate` to an interger number of Hz.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Rate;
    /// let rate = Rate::from_hz(1000);
    /// let hz = rate.as_hz();
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn as_hz(&self) -> u32 {
        self.0.to_Hz()
    }

    /// Convert the `Rate` to an interger number of kHz.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Rate;
    /// let rate = Rate::from_khz(1000);
    /// let khz = rate.as_khz();
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn as_khz(&self) -> u32 {
        self.0.to_kHz()
    }

    /// Convert the `Rate` to an interger number of MHz.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Rate;
    /// let rate = Rate::from_mhz(1000);
    /// let mhz = rate.as_mhz();
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn as_mhz(&self) -> u32 {
        self.0.to_MHz()
    }

    /// Convert the `Rate` to a `Duration`.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Rate;
    /// let rate = Rate::from_hz(1000);
    /// let duration = rate.as_duration();
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn as_duration(&self) -> Duration {
        // Duration::from_micros(1_000_000 / self.as_hz() as u64)
        Duration::from_nanos(1_000_000_000 / self.as_hz() as u64)
    }
}

impl core::ops::Div for Rate {
    type Output = u32;

    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl core::ops::Mul<u32> for Rate {
    type Output = Rate;

    #[inline]
    fn mul(self, rhs: u32) -> Self::Output {
        Rate(self.0 * rhs)
    }
}

impl core::ops::Div<u32> for Rate {
    type Output = Rate;

    #[inline]
    fn div(self, rhs: u32) -> Self::Output {
        Rate(self.0 / rhs)
    }
}

/// Represents an instant in time.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(InnerInstant);

impl Debug for Instant {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Instant({} µs since epoch)",
            self.duration_since_epoch().as_micros()
        )
    }
}

impl Display for Instant {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{} µs since epoch",
            self.duration_since_epoch().as_micros()
        )
    }
}

impl core::hash::Hash for Instant {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.duration_since_epoch().hash(state);
    }
}

impl Instant {
    /// Represents the moment the system booted.
    pub const EPOCH: Instant = Instant(InnerInstant::from_ticks(0));

    /// Returns the current instant.
    ///
    /// The counter won’t measure time in sleep-mode.
    ///
    /// The timer has a 1 microsecond resolution and will wrap after
    /// # {wrap_after}
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Instant;
    /// let now = Instant::now();
    /// # {after_snippet}
    /// ```
    #[inline]
    pub fn now() -> Self {
        now()
    }

    #[inline]
    pub(crate) fn from_ticks(ticks: u64) -> Self {
        Instant(InnerInstant::from_ticks(ticks))
    }

    /// Returns the elapsed `Duration` since boot.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Instant;
    /// let now = Instant::now();
    /// let duration = now.duration_since_epoch();
    /// # {after_snippet}
    /// ```
    #[inline]
    pub fn duration_since_epoch(&self) -> Duration {
        *self - Self::EPOCH
    }

    /// Returns the elapsed `Duration` since this `Instant` was created.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Instant;
    /// let now = Instant::now();
    /// let duration = now.elapsed();
    /// # {after_snippet}
    /// ```
    #[inline]
    pub fn elapsed(&self) -> Duration {
        Self::now() - *self
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        Instant(self.0 + rhs.0)
    }
}

impl core::ops::AddAssign<Duration> for Instant {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs.0;
    }
}

impl core::ops::Sub for Instant {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        // Avoid "Sub failed! Other > self" panics
        Duration::from_ticks(self.0.ticks().wrapping_sub(rhs.0.ticks()))
    }
}

impl core::ops::Sub<Duration> for Instant {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Duration) -> Self::Output {
        Instant(self.0 - rhs.0)
    }
}

impl core::ops::SubAssign<Duration> for Instant {
    #[inline]
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 -= rhs.0;
    }
}

/// Represents a duration of time.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(InnerDuration);

impl Debug for Duration {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Duration({} µs)", self.as_micros())
    }
}

impl Display for Duration {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} µs", self.as_micros())
    }
}

impl core::hash::Hash for Duration {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_micros().hash(state);
    }
}

impl Duration {
    /// A duration of zero time.
    pub const ZERO: Self = Self(InnerDuration::from_ticks(0));

    /// A duration representing the maximum possible time.
    pub const MAX: Self = Self(InnerDuration::from_ticks(u64::MAX));

    /// Creates a duration which represents ticks.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_ticks(1000);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_ticks(val: u64) -> Self {
        Self(InnerDuration::from_ticks(val))
    }

    /// Creates a duration which represents nanoseconds.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_nanos(1000);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_nanos(val: u64) -> Self {
        Self(InnerDuration::nanos(val))
    }

    /// Creates a duration which represents microseconds.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_micros(1000);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_micros(val: u64) -> Self {
        Self(InnerDuration::micros(val))
    }

    /// Creates a duration which represents milliseconds.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_millis(100);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_millis(val: u64) -> Self {
        Self(InnerDuration::millis(val))
    }

    /// Creates a duration which represents seconds.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_secs(1);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_secs(val: u64) -> Self {
        Self(InnerDuration::secs(val))
    }

    /// Creates a duration which represents minutes.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_minutes(1);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_minutes(val: u64) -> Self {
        Self(InnerDuration::minutes(val))
    }

    /// Creates a duration which represents hours.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_hours(1);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn from_hours(val: u64) -> Self {
        Self(InnerDuration::hours(val))
    }

    delegate::delegate! {
        #[inline]
        to self.0 {
            /// Convert the `Duration` to an interger number of ticks.
            ///
            /// ## Example
            ///
            /// ```rust, no_run
            /// # {before_snippet}
            /// use crate::time::Duration;
            /// let duration = Duration::from_micros(1000);
            /// let micros = duration.ticks();
            /// # {after_snippet}
            /// ```
            #[call(ticks)]
            pub const fn ticks(&self) -> u64;

            /// Convert the `Duration` to an interger number of microseconds.
            ///
            /// ## Example
            ///
            /// ```rust, no_run
            /// # {before_snippet}
            /// use crate::time::Duration;
            /// let duration = Duration::from_nanos(1000);
            /// let micros = duration.as_nanos();
            /// # {after_snippet}
            /// ```
            #[call(to_nanos)]
            pub const fn as_nanos(&self) -> u64;

            /// Convert the `Duration` to an interger number of microseconds.
            ///
            /// ## Example
            ///
            /// ```rust, no_run
            /// # {before_snippet}
            /// use crate::time::Duration;
            /// let duration = Duration::from_micros(1000);
            /// let micros = duration.as_micros();
            /// # {after_snippet}
            /// ```
            #[call(to_micros)]
            pub const fn as_micros(&self) -> u64;

            /// Convert the `Duration` to an interger number of milliseconds.
            ///
            /// ## Example
            ///
            /// ```rust, no_run
            /// # {before_snippet}
            /// use crate::time::Duration;
            /// let duration = Duration::from_millis(100);
            /// let millis = duration.as_millis();
            /// # {after_snippet}
            /// ```
            #[call(to_millis)]
            pub const fn as_millis(&self) -> u64;

            /// Convert the `Duration` to an interger number of seconds.
            ///
            /// ## Example
            ///
            /// ```rust, no_run
            /// # {before_snippet}
            /// use crate::time::Duration;
            /// let duration = Duration::from_secs(1);
            /// let secs = duration.as_secs();
            /// # {after_snippet}
            /// ```
            #[call(to_secs)]
            pub const fn as_secs(&self) -> u64;

            /// Convert the `Duration` to an interger number of minutes.
            ///
            /// ## Example
            ///
            /// ```rust, no_run
            /// # {before_snippet}
            /// use crate::time::Duration;
            /// let duration = Duration::from_minutes(1);
            /// let minutes = duration.as_minutes();
            /// # {after_snippet}
            /// ```
            #[call(to_minutes)]
            pub const fn as_minutes(&self) -> u64;

            /// Convert the `Duration` to an interger number of hours.
            ///
            /// ## Example
            ///
            /// ```rust, no_run
            /// # {before_snippet}
            /// use crate::time::Duration;
            /// let duration = Duration::from_hours(1);
            /// let hours = duration.as_hours();
            /// # {after_snippet}
            /// ```
            #[call(to_hours)]
            pub const fn as_hours(&self) -> u64;
        }
    }

    /// Add two durations while checking for overflow.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_secs(1);
    /// let duration2 = Duration::from_secs(2);
    ///
    /// if let Some(sum) = duration.checked_add(duration2) {
    ///     println!("Sum: {}", sum);
    /// } else {
    ///     println!("Overflow occurred");
    /// }
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        if let Some(val) = self.0.checked_add(rhs.0) {
            Some(Duration(val))
        } else {
            None
        }
    }

    /// Subtract two durations while checking for overflow.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_secs(3);
    /// let duration2 = Duration::from_secs(1);
    ///
    /// if let Some(diff) = duration.checked_sub(duration2) {
    ///     println!("Difference: {}", diff);
    /// } else {
    ///     println!("Underflow occurred");
    /// }
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        if let Some(val) = self.0.checked_sub(rhs.0) {
            Some(Duration(val))
        } else {
            None
        }
    }

    /// Add two durations, returning the maximum value if overflow occurred.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_secs(1);
    /// let duration2 = Duration::from_secs(2);
    ///
    /// let sum = duration.saturating_add(duration2);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        if let Some(val) = self.checked_add(rhs) {
            val
        } else {
            Self::MAX
        }
    }

    /// Subtract two durations, returning the minimum value if the result would
    /// be negative.
    ///
    /// ## Example
    ///
    /// ```rust, no_run
    /// # {before_snippet}
    /// use crate::time::Duration;
    /// let duration = Duration::from_secs(3);
    /// let duration2 = Duration::from_secs(1);
    ///
    /// let diff = duration.saturating_sub(duration2);
    /// # {after_snippet}
    /// ```
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        if let Some(val) = self.checked_sub(rhs) {
            val
        } else {
            Self::ZERO
        }
    }
}

impl core::ops::Add for Duration {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Duration(self.0 + rhs.0)
    }
}

impl core::ops::AddAssign for Duration {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl core::ops::Sub for Duration {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl core::ops::SubAssign for Duration {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl core::ops::Mul<u32> for Duration {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: u32) -> Self::Output {
        Duration(self.0 * rhs)
    }
}

impl core::ops::Div<u32> for Duration {
    type Output = Self;

    #[inline]
    fn div(self, rhs: u32) -> Self::Output {
        Duration(self.0 / rhs)
    }
}

impl core::ops::Div<Duration> for Duration {
    type Output = u64;

    #[inline]
    fn div(self, rhs: Duration) -> Self::Output {
        self.0 / rhs.0
    }
}

#[inline]
fn now() -> Instant {
    let cyc_lo = crate::cycle_delay::cycles();
    let cyc_hi = crate::cycle_delay::cycles_hi();
    if cyc_hi == 0 {
        Instant::from_ticks((cyc_hi as u64) << 32 | cyc_lo as u64)
    } else {
        Instant::from_ticks(cyc_lo as u64)
    }
}
```
