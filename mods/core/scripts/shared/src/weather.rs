use core::f32::consts::{FRAC_PI_2, PI};
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration;

use game_wasm::components::Component;
use game_wasm::encoding::{Decode, Encode};
use game_wasm::math::{Quat, Vec3};
use game_wasm::world::RecordReference;

use crate::components::SKY_LIGHT;

const NANOS_PER_SEC: u32 = 1_000_000_000;
const NANOS_PER_MIN: u128 = NANOS_PER_SEC as u128 * 60;
const NANOS_PER_HOUR: u128 = NANOS_PER_SEC as u128 * 60 * 60;
const NANOS_PER_DAY: u128 = NANOS_PER_SEC as u128 * 60 * 60 * 24;

/// A light that is used to simulate light from sky (Sun, Moon, etc.).
#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SkyLight;

impl Component for SkyLight {
    const ID: RecordReference = SKY_LIGHT;
}

/// The date and time of the world.
///
/// The ingame time ticks as follows:
/// 1. A day = 24h
/// 2. A week = 7d
/// 3. A month: number of days depending on the month:
///  1. January: `31`
///  2. February: `28`
///  3. March: `31`
///  4. April: `30`
///  5. May: `31`
///  6. June: `30`
///  7. July: `31`
///  8. August: `31`
///  9. September: `30`
///  10. October: `31`
///  11. November: `30`
///  12. December: `31`
///
/// There are not leap years or leap seconds, every year is repeated as is.
///
/// The time should be constantly advanced using the `Add<Duration>` impl.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
// FIXME: Rename to WorldTime to distinguish with frame time Time.
pub struct DateTime {
    nsecs: u128,
}

impl DateTime {
    /// Creates a new `DateTime` starting at `0`.
    #[inline]
    pub const fn new() -> Self {
        Self { nsecs: 0 }
    }

    /// Returns the `DateTime` representation as nanoseconds.
    #[inline]
    pub const fn as_nanos(&self) -> u128 {
        self.nsecs
    }

    /// Returns the number of seconds of this `DateTime`.
    #[inline]
    pub const fn as_secs(&self) -> u64 {
        (self.nsecs / NANOS_PER_SEC as u128) as u64
    }

    pub fn as_secs_f32(&self) -> f32 {
        (self.nsecs as f32) / (NANOS_PER_SEC as f32)
    }

    pub fn as_secs_f64(&self) -> f64 {
        (self.nsecs as f64) / (NANOS_PER_SEC as f64)
    }

    pub const fn from_secs(secs: u64) -> Self {
        Self {
            nsecs: NANOS_PER_SEC as u128 * secs as u128,
        }
    }

    /// Returns the current monolithic day.
    pub const fn day(&self) -> u64 {
        let days = self.as_secs() / (60 * 60 * 24);
        days + 1
    }

    pub fn set_day(&mut self, day: u64) {
        self.nsecs = (self.nsecs % NANOS_PER_DAY) + (NANOS_PER_DAY * day as u128);
    }

    /// Returns the current monolithic week.
    pub const fn week(&self) -> u64 {
        let weeks = self.as_secs() / (60 * 60 * 24 * 7);
        weeks + 1
    }

    /// Returns the [`Weekday`] of the current day.
    pub const fn weekday(&self) -> Weekday {
        // Since the first value `self.day()` returns is a `1`, that
        // value is Monday. Wrap around at 7.
        match self.day() % 7 {
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            6 => Weekday::Saturday,
            0 => Weekday::Sunday,
            _ => unreachable!(),
        }
    }

    /// Returns the current [`Month`].
    pub const fn month(&self) -> Month {
        const JAN: u16 = Month::January.days() as u16;
        const FEB: u16 = JAN + Month::February.days() as u16;
        const MAR: u16 = FEB + Month::March.days() as u16;
        const APR: u16 = MAR + Month::April.days() as u16;
        const MAY: u16 = APR + Month::May.days() as u16;
        const JUN: u16 = MAY + Month::June.days() as u16;
        const JUL: u16 = JUN + Month::July.days() as u16;
        const AUG: u16 = JUL + Month::August.days() as u16;
        const SEP: u16 = AUG + Month::September.days() as u16;
        const OCT: u16 = SEP + Month::October.days() as u16;
        const NOV: u16 = OCT + Month::November.days() as u16;

        match self.ordinal() {
            n if n <= JAN => Month::January,
            n if n <= FEB => Month::February,
            n if n <= MAR => Month::March,
            n if n <= APR => Month::April,
            n if n <= MAY => Month::May,
            n if n <= JUN => Month::June,
            n if n <= JUL => Month::July,
            n if n <= AUG => Month::August,
            n if n <= SEP => Month::September,
            n if n <= OCT => Month::October,
            n if n <= NOV => Month::November,
            _ => Month::December,
        }
    }

    /// Retursn the current day of the year in the range of `1..=366`.
    pub const fn ordinal(&self) -> u16 {
        (self.day() / 365) as u16
    }

    pub const fn year(&self) -> u64 {
        let years = self.as_secs() / (60 * 60 * 24 * 365);
        years + 1
    }

    pub fn hour(&self) -> u32 {
        (self.nsecs % NANOS_PER_DAY / NANOS_PER_HOUR) as u32
    }

    pub fn minute(&self) -> u32 {
        (self.nsecs % NANOS_PER_HOUR / NANOS_PER_MIN) as u32
    }
}

impl Add<Duration> for DateTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        let nsecs = self.nsecs.saturating_add(rhs.as_nanos());
        Self { nsecs }
    }
}

impl AddAssign<Duration> for DateTime {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for DateTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        let nsecs = self.nsecs.saturating_sub(rhs.as_nanos());
        Self { nsecs }
    }
}

impl SubAssign<Duration> for DateTime {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Returns the next following `Weekday`.
    pub const fn next(self) -> Self {
        match self {
            Self::Monday => Self::Tuesday,
            Self::Tuesday => Self::Wednesday,
            Self::Wednesday => Self::Thursday,
            Self::Thursday => Self::Friday,
            Self::Friday => Self::Saturday,
            Self::Saturday => Self::Sunday,
            Self::Sunday => Self::Monday,
        }
    }

    /// Returns the previous `Weekday`.
    pub const fn prev(self) -> Self {
        match self {
            Self::Monday => Self::Sunday,
            Self::Tuesday => Self::Monday,
            Self::Wednesday => Self::Tuesday,
            Self::Thursday => Self::Wednesday,
            Self::Friday => Self::Thursday,
            Self::Saturday => Self::Friday,
            Self::Sunday => Self::Saturday,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl Month {
    pub const fn next(self) -> Self {
        match self {
            Self::January => Self::February,
            Self::February => Self::March,
            Self::March => Self::April,
            Self::April => Self::May,
            Self::May => Self::June,
            Self::June => Self::July,
            Self::July => Self::August,
            Self::August => Self::September,
            Self::September => Self::October,
            Self::October => Self::November,
            Self::November => Self::December,
            Self::December => Self::January,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::January => Self::December,
            Self::February => Self::January,
            Self::March => Self::February,
            Self::April => Self::March,
            Self::May => Self::April,
            Self::June => Self::May,
            Self::July => Self::June,
            Self::August => Self::July,
            Self::September => Self::August,
            Self::October => Self::September,
            Self::November => Self::October,
            Self::December => Self::November,
        }
    }

    pub const fn days(self) -> u8 {
        match self {
            Self::January => 31,
            Self::February => 28,
            Self::March => 31,
            Self::April => 30,
            Self::May => 31,
            Self::June => 30,
            Self::July => 31,
            Self::August => 31,
            Self::September => 30,
            Self::October => 31,
            Self::November => 30,
            Self::December => 31,
        }
    }
}

/// How fast time elapses relative to real time.
///
/// The default scale is 5x, i.e. 5 ingame seconds take 1 real second.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TimeScale(pub f32);

impl TimeScale {
    /// Creates a new `TimeScale` with a factor of `n`. In other words, how many ingame seconds
    /// elapse for every real second.
    #[inline]
    pub fn new(n: f32) -> Self {
        Self(n)
    }
}

impl Default for TimeScale {
    #[inline]
    fn default() -> Self {
        Self::new(5.0)
    }
}

pub fn sun_rotation(date: DateTime) -> Quat {
    let day_time = date.hour() as f32;
    let angle = (day_time - 6.0) / 12.0 * -PI;
    Quat::from_axis_angle(Vec3::X, angle)
}

#[cfg(test)]
mod tests {

    use core::f32::consts::PI;

    use game_wasm::math::{Quat, RotationExt, Vec3};

    use super::{sun_rotation, DateTime};

    #[test]
    fn test_datetime_hour() {
        let date = DateTime::from_secs(60 * 60);
        assert_eq!(date.hour(), 1);

        let date = DateTime::from_secs(26 * 60 * 60);
        assert_eq!(date.hour(), 2);
    }

    #[test]
    fn test_sun_rotation() {
        // Midnight
        let date = DateTime::new();
        assert_eq!(sun_rotation(date), Quat::UP);

        // Sunrise
        let mut date = DateTime::from_secs(6 * 60 * 60);
        assert_eq!(sun_rotation(date), Quat::FORWARD);

        // Noon
        let date = DateTime::from_secs(12 * 60 * 60);
        assert_eq!(sun_rotation(date), Quat::DOWN);

        let date = DateTime::from_secs(18 * 60 * 60);
        // Equivalent to `Quat::BACKWARD` but differently around
        // different axes, therefore the Quaternion is not the same.
        assert_eq!(sun_rotation(date), Quat::from_axis_angle(Vec3::X, -PI));
    }
}
