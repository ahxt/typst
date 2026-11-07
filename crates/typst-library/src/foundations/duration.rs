use std::fmt::{self, Debug, Formatter};
use std::ops::Div;

use ecow::{EcoString, eco_format};
use time::ext::NumericalDuration;

use crate::diag::StrResult;
use crate::foundations::{Repr, func, repr, scope, ty};

/// Represents a positive or negative span of time.
#[ty(scope, cast, since = "0.8.0")]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Duration(time::Duration);

impl Duration {
    /// Whether the duration is empty / zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Decomposes the time into whole weeks, days, hours, minutes, and seconds.
    pub fn decompose(&self) -> [i64; 5] {
        let mut tmp = self.0;
        let weeks = tmp.whole_weeks();
        tmp -= weeks.weeks();
        let days = tmp.whole_days();
        tmp -= days.days();
        let hours = tmp.whole_hours();
        tmp -= hours.hours();
        let minutes = tmp.whole_minutes();
        tmp -= minutes.minutes();
        let seconds = tmp.whole_seconds();
        [weeks, days, hours, minutes, seconds]
    }
}

#[scope]
impl Duration {
    /// Creates a new duration.
    ///
    /// You can specify the @duration[duration] using weeks, days, hours,
    /// minutes and seconds. You can also get a duration by subtracting two
    /// @datetime[datetimes].
    ///
    /// ```example
    /// #duration(
    ///   days: 3,
    ///   hours: 12,
    /// ).hours()
    /// ```
    #[func(constructor, since = "0.8.0")]
    pub fn construct(
        /// The number of seconds.
        #[named]
        #[default(0)]
        seconds: i64,
        /// The number of minutes.
        #[named]
        #[default(0)]
        minutes: i64,
        /// The number of hours.
        #[named]
        #[default(0)]
        hours: i64,
        /// The number of days.
        #[named]
        #[default(0)]
        days: i64,
        /// The number of weeks.
        #[named]
        #[default(0)]
        weeks: i64,
    ) -> StrResult<Duration> {
        let overflow = || "duration is too large";
        let total = seconds
            .checked_add(minutes.checked_mul(60).ok_or_else(overflow)?)
            .and_then(|s| s.checked_add(hours.checked_mul(3_600)?))
            .and_then(|s| s.checked_add(days.checked_mul(86_400)?))
            .and_then(|s| s.checked_add(weeks.checked_mul(604_800)?))
            .ok_or_else(overflow)?;
        Ok(Duration::from(time::Duration::seconds(total)))
    }

    /// The duration expressed in seconds.
    ///
    /// This function returns the total duration represented in seconds as a
    /// floating-point number, rather than the seconds component of the
    /// duration.
    #[func(since = "0.8.0")]
    pub fn seconds(&self) -> f64 {
        self.0.as_seconds_f64()
    }

    /// The duration expressed in minutes.
    ///
    /// This function returns the total duration represented in minutes as a
    /// floating-point number, rather than the minutes component of the
    /// duration.
    #[func(since = "0.8.0")]
    pub fn minutes(&self) -> f64 {
        self.seconds() / 60.0
    }

    /// The duration expressed in hours.
    ///
    /// This function returns the total duration represented in hours as a
    /// floating-point number, rather than the hours component of the duration.
    #[func(since = "0.8.0")]
    pub fn hours(&self) -> f64 {
        self.seconds() / 3_600.0
    }

    /// The duration expressed in days.
    ///
    /// This function returns the total duration represented in days as a
    /// floating-point number, rather than the days component of the duration.
    #[func(since = "0.8.0")]
    pub fn days(&self) -> f64 {
        self.seconds() / 86_400.0
    }

    /// The duration expressed in weeks.
    ///
    /// This function returns the total duration represented in weeks as a
    /// floating-point number, rather than the weeks component of the duration.
    #[func(since = "0.8.0")]
    pub fn weeks(&self) -> f64 {
        self.seconds() / 604_800.0
    }
}

impl Debug for Duration {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Repr for Duration {
    fn repr(&self) -> EcoString {
        let [weeks, days, hours, minutes, seconds] = self.decompose();
        let mut vec = Vec::with_capacity(5);

        if weeks != 0 {
            vec.push(eco_format!("weeks: {}", weeks.repr()));
        }

        if days != 0 {
            vec.push(eco_format!("days: {}", days.repr()));
        }

        if hours != 0 {
            vec.push(eco_format!("hours: {}", hours.repr()));
        }

        if minutes != 0 {
            vec.push(eco_format!("minutes: {}", minutes.repr()));
        }

        if seconds != 0 {
            vec.push(eco_format!("seconds: {}", seconds.repr()));
        }

        eco_format!("duration{}", &repr::pretty_array_like(&vec, false))
    }
}

impl From<time::Duration> for Duration {
    fn from(value: time::Duration) -> Self {
        Self(value)
    }
}

impl From<Duration> for time::Duration {
    fn from(value: Duration) -> Self {
        value.0
    }
}

impl Duration {
    /// Add two durations, failing on overflow.
    pub fn checked_add(self, rhs: Self) -> StrResult<Self> {
        self.0.checked_add(rhs.0).map(Self).ok_or_else(too_large)
    }

    /// Subtract two durations, failing on overflow.
    pub fn checked_sub(self, rhs: Self) -> StrResult<Self> {
        self.0.checked_sub(rhs.0).map(Self).ok_or_else(too_large)
    }

    /// Negate a duration, failing on overflow.
    pub fn checked_neg(self) -> StrResult<Self> {
        Self(time::Duration::ZERO).checked_sub(self)
    }

    /// Multiply by a float, failing if the result is not representable.
    pub fn checked_mul(self, rhs: f64) -> StrResult<Self> {
        Self::from_seconds_f64(self.0.as_seconds_f64() * rhs)
    }

    /// Divide by a float, failing if the result is not representable.
    pub fn checked_div(self, rhs: f64) -> StrResult<Self> {
        Self::from_seconds_f64(self.0.as_seconds_f64() / rhs)
    }

    /// Create a duration from seconds, failing if it is not finite or out of
    /// the representable range.
    fn from_seconds_f64(secs: f64) -> StrResult<Self> {
        // `i64::MAX as f64` is 2^63, so both bounds are exact powers of two.
        if !secs.is_finite() || secs < i64::MIN as f64 || secs >= i64::MAX as f64 {
            return Err(too_large());
        }
        Ok(Self(time::Duration::seconds_f64(secs)))
    }
}

fn too_large() -> EcoString {
    "duration is too large".into()
}

impl Div for Duration {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}
