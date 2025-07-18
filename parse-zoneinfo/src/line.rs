//! Parsing zoneinfo data files, line-by-line.
//!
//! This module provides functions that take a line of input from a zoneinfo
//! data file and attempts to parse it, returning the details of the line if
//! it gets parsed successfully. It classifies them as `Rule`, `Link`,
//! `Zone`, or `Continuation` lines.
//!
//! `Line` is the type that parses and holds zoneinfo line data. To try to
//! parse a string, use the `Line::from_str` constructor. (This isn’t the
//! `FromStr` trait, so you can’t use `parse` on a string. Sorry!)
//!
//! ## Examples
//!
//! Parsing a `Rule` line:
//!
//! ```
//! use parse_zoneinfo::line::*;
//!
//! let line = Line::new("Rule  EU  1977    1980    -   Apr Sun>=1   1:00u  1:00    S");
//!
//! assert_eq!(line, Ok(Line::Rule(Rule {
//!     name:         "EU",
//!     from_year:    Year::Number(1977),
//!     to_year:      Some(Year::Number(1980)),
//!     month:        Month::April,
//!     day:          DaySpec::FirstOnOrAfter(Weekday::Sunday, 1),
//!     time:         TimeSpec::HoursMinutes(1, 0).with_type(TimeType::UTC),
//!     time_to_add:  TimeSpec::HoursMinutes(1, 0),
//!     letters:      Some("S"),
//! })));
//! ```
//!
//! Parsing a `Zone` line:
//!
//! ```
//! use parse_zoneinfo::line::*;
//!
//! let line = Line::new("Zone  Australia/Adelaide  9:30  Aus  AC%sT  1971 Oct 31  2:00:00");
//!
//! assert_eq!(line, Ok(Line::Zone(Zone {
//!     name: "Australia/Adelaide",
//!     info: ZoneInfo {
//!         utc_offset:  TimeSpec::HoursMinutes(9, 30),
//!         saving:      Saving::Multiple("Aus"),
//!         format:      "AC%sT",
//!         time:        Some(ChangeTime::UntilTime(
//!                         Year::Number(1971),
//!                         Month::October,
//!                         DaySpec::Ordinal(31),
//!                         TimeSpec::HoursMinutesSeconds(2, 0, 0).with_type(TimeType::Wall))
//!                      ),
//!     },
//! })));
//! ```
//!
//! Parsing a `Link` line:
//!
//! ```
//! use parse_zoneinfo::line::*;
//!
//! let line = Line::new("Link  Europe/Istanbul  Asia/Istanbul");
//! assert_eq!(line, Ok(Line::Link(Link {
//!     existing:  "Europe/Istanbul",
//!     new:       "Asia/Istanbul",
//! })));
//! ```

use std::fmt;
use std::str::FromStr;

#[derive(PartialEq, Debug, Clone)]
pub enum Error {
    FailedYearParse(String),
    FailedMonthParse(String),
    FailedWeekdayParse(String),
    InvalidLineType(String),
    TypeColumnContainedNonHyphen(String),
    CouldNotParseSaving(String),
    InvalidDaySpec(String),
    InvalidTimeSpecAndType(String),
    NonWallClockInTimeSpec(String),
    NotParsedAsRuleLine,
    NotParsedAsZoneLine,
    NotParsedAsLinkLine,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::FailedYearParse(s) => write!(f, "failed to parse as a year value: \"{}\"", s),
            Error::FailedMonthParse(s) => write!(f, "failed to parse as a month value: \"{}\"", s),
            Error::FailedWeekdayParse(s) => {
                write!(f, "failed to parse as a weekday value: \"{}\"", s)
            }
            Error::InvalidLineType(s) => write!(f, "line with invalid format: \"{}\"", s),
            Error::TypeColumnContainedNonHyphen(s) => {
                write!(
                    f,
                    "'type' column is not a hyphen but has the value: \"{}\"",
                    s
                )
            }
            Error::CouldNotParseSaving(s) => write!(f, "failed to parse RULES column: \"{}\"", s),
            Error::InvalidDaySpec(s) => write!(f, "invalid day specification ('ON'): \"{}\"", s),
            Error::InvalidTimeSpecAndType(s) => write!(f, "invalid time: \"{}\"", s),
            Error::NonWallClockInTimeSpec(s) => {
                write!(f, "time value not given as wall time: \"{}\"", s)
            }
            Error::NotParsedAsRuleLine => write!(f, "failed to parse line as a rule"),
            Error::NotParsedAsZoneLine => write!(f, "failed to parse line as a zone"),
            Error::NotParsedAsLinkLine => write!(f, "failed to parse line as a link"),
        }
    }
}

impl std::error::Error for Error {}

/// A **year** definition field.
///
/// A year has one of the following representations in a file:
///
/// - `min` or `minimum`, the minimum year possible, for when a rule needs to
///   apply up until the first rule with a specific year;
/// - `max` or `maximum`, the maximum year possible, for when a rule needs to
///   apply after the last rule with a specific year;
/// - a year number, referring to a specific year.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Year {
    /// The minimum year possible: `min` or `minimum`.
    Minimum,
    /// The maximum year possible: `max` or `maximum`.
    Maximum,
    /// A specific year number.
    Number(i64),
}

impl FromStr for Year {
    type Err = Error;

    fn from_str(input: &str) -> Result<Year, Self::Err> {
        Ok(match &*input.to_ascii_lowercase() {
            "min" | "minimum" => Year::Minimum,
            "max" | "maximum" => Year::Maximum,
            year => match year.parse() {
                Ok(year) => Year::Number(year),
                Err(_) => return Err(Error::FailedYearParse(input.to_string())),
            },
        })
    }
}

/// A **month** field, which is actually just a wrapper around
/// `datetime::Month`.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl Month {
    fn length(self, is_leap: bool) -> i8 {
        match self {
            Month::January => 31,
            Month::February if is_leap => 29,
            Month::February => 28,
            Month::March => 31,
            Month::April => 30,
            Month::May => 31,
            Month::June => 30,
            Month::July => 31,
            Month::August => 31,
            Month::September => 30,
            Month::October => 31,
            Month::November => 30,
            Month::December => 31,
        }
    }

    /// Get the next calendar month, with an error going from Dec->Jan
    fn next_in_year(self) -> Result<Month, &'static str> {
        Ok(match self {
            Month::January => Month::February,
            Month::February => Month::March,
            Month::March => Month::April,
            Month::April => Month::May,
            Month::May => Month::June,
            Month::June => Month::July,
            Month::July => Month::August,
            Month::August => Month::September,
            Month::September => Month::October,
            Month::October => Month::November,
            Month::November => Month::December,
            Month::December => Err("Cannot wrap year from dec->jan")?,
        })
    }

    /// Get the previous calendar month, with an error going from Jan->Dec
    fn prev_in_year(self) -> Result<Month, &'static str> {
        Ok(match self {
            Month::January => Err("Cannot wrap years from jan->dec")?,
            Month::February => Month::January,
            Month::March => Month::February,
            Month::April => Month::March,
            Month::May => Month::April,
            Month::June => Month::May,
            Month::July => Month::June,
            Month::August => Month::July,
            Month::September => Month::August,
            Month::October => Month::September,
            Month::November => Month::October,
            Month::December => Month::November,
        })
    }
}

impl FromStr for Month {
    type Err = Error;

    /// Attempts to parse the given string into a value of this type.
    fn from_str(input: &str) -> Result<Month, Self::Err> {
        Ok(match &*input.to_ascii_lowercase() {
            "jan" | "january" => Month::January,
            "feb" | "february" => Month::February,
            "mar" | "march" => Month::March,
            "apr" | "april" => Month::April,
            "may" => Month::May,
            "jun" | "june" => Month::June,
            "jul" | "july" => Month::July,
            "aug" | "august" => Month::August,
            "sep" | "september" => Month::September,
            "oct" | "october" => Month::October,
            "nov" | "november" => Month::November,
            "dec" | "december" => Month::December,
            other => return Err(Error::FailedMonthParse(other.to_string())),
        })
    }
}

/// A **weekday** field, which is actually just a wrapper around
/// `datetime::Weekday`.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    fn calculate(year: i64, month: Month, day: i8) -> Weekday {
        let m = month as i64;
        let y = if m < 3 { year - 1 } else { year };
        let d = day as i64;
        const T: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        match (y + y / 4 - y / 100 + y / 400 + T[m as usize - 1] + d) % 7 {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            6 => Weekday::Saturday,
            _ => panic!("why is negative modulus designed so?"),
        }
    }
}

impl FromStr for Weekday {
    type Err = Error;

    fn from_str(input: &str) -> Result<Weekday, Self::Err> {
        Ok(match &*input.to_ascii_lowercase() {
            "mon" | "monday" => Weekday::Monday,
            "tue" | "tuesday" => Weekday::Tuesday,
            "wed" | "wednesday" => Weekday::Wednesday,
            "thu" | "thursday" => Weekday::Thursday,
            "fri" | "friday" => Weekday::Friday,
            "sat" | "saturday" => Weekday::Saturday,
            "sun" | "sunday" => Weekday::Sunday,
            other => return Err(Error::FailedWeekdayParse(other.to_string())),
        })
    }
}

/// A **day** definition field.
///
/// This can be given in either absolute terms (such as “the fifth day of the
/// month”), or relative terms (such as “the last Sunday of the month”, or
/// “the last Friday before or including the 13th”).
///
/// Note that in the last example, it’s allowed for that particular Friday to
/// *be* the 13th in question.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum DaySpec {
    /// A specific day of the month, given by its number.
    Ordinal(i8),
    /// The last day of the month with a specific weekday.
    Last(Weekday),
    /// The **last** day with the given weekday **before** (or including) a
    /// day with a specific number.
    LastOnOrBefore(Weekday, i8),
    /// The **first** day with the given weekday **after** (or including) a
    /// day with a specific number.
    FirstOnOrAfter(Weekday, i8),
}

impl DaySpec {
    /// Converts this day specification to a concrete date, given the year and
    /// month it should occur in.
    pub fn to_concrete_day(&self, year: i64, month: Month) -> (Month, i8) {
        let leap = is_leap(year);
        let length = month.length(leap);
        // we will never hit the 0 because we unwrap prev_in_year below
        let prev_length = month.prev_in_year().map(|m| m.length(leap)).unwrap_or(0);

        match *self {
            DaySpec::Ordinal(day) => (month, day),
            DaySpec::Last(weekday) => (
                month,
                (1..length + 1)
                    .rev()
                    .find(|&day| Weekday::calculate(year, month, day) == weekday)
                    .unwrap(),
            ),
            DaySpec::LastOnOrBefore(weekday, day) => (-7..day + 1)
                .rev()
                .flat_map(|inner_day| {
                    if inner_day >= 1 && Weekday::calculate(year, month, inner_day) == weekday {
                        Some((month, inner_day))
                    } else if inner_day < 1
                        && Weekday::calculate(
                            year,
                            month.prev_in_year().unwrap(),
                            prev_length + inner_day,
                        ) == weekday
                    {
                        // inner_day is negative, so this is subtraction
                        Some((month.prev_in_year().unwrap(), prev_length + inner_day))
                    } else {
                        None
                    }
                })
                .next()
                .unwrap(),
            DaySpec::FirstOnOrAfter(weekday, day) => (day..day + 8)
                .flat_map(|inner_day| {
                    if inner_day <= length && Weekday::calculate(year, month, inner_day) == weekday
                    {
                        Some((month, inner_day))
                    } else if inner_day > length
                        && Weekday::calculate(
                            year,
                            month.next_in_year().unwrap(),
                            inner_day - length,
                        ) == weekday
                    {
                        Some((month.next_in_year().unwrap(), inner_day - length))
                    } else {
                        None
                    }
                })
                .next()
                .unwrap(),
        }
    }
}

impl FromStr for DaySpec {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        // Parse the field as a number if it vaguely resembles one.
        if input.chars().all(|c| c.is_ascii_digit()) {
            return Ok(DaySpec::Ordinal(input.parse().unwrap()));
        }
        // Check if it starts with ‘last’, and trim off the first four bytes if it does
        else if let Some(remainder) = input.strip_prefix("last") {
            let weekday = remainder.parse()?;
            return Ok(DaySpec::Last(weekday));
        }

        let weekday = match input.get(..3) {
            Some(wd) => Weekday::from_str(wd)?,
            None => return Err(Error::InvalidDaySpec(input.to_string())),
        };

        let dir = match input.get(3..5) {
            Some(">=") => true,
            Some("<=") => false,
            _ => return Err(Error::InvalidDaySpec(input.to_string())),
        };

        let day = match input.get(5..) {
            Some(day) => u8::from_str(day).map_err(|_| Error::InvalidDaySpec(input.to_string()))?,
            None => return Err(Error::InvalidDaySpec(input.to_string())),
        } as i8;

        Ok(match dir {
            true => DaySpec::FirstOnOrAfter(weekday, day),
            false => DaySpec::LastOnOrBefore(weekday, day),
        })
    }
}

fn is_leap(year: i64) -> bool {
    // Leap year rules: years which are factors of 4, except those divisible
    // by 100, unless they are divisible by 400.
    //
    // We test most common cases first: 4th year, 100th year, then 400th year.
    //
    // We factor out 4 from 100 since it was already tested, leaving us checking
    // if it's divisible by 25. Afterwards, we do the same, factoring 25 from
    // 400, leaving us with 16.
    //
    // Factors of 4 and 16 can quickly be found with bitwise AND.
    year & 3 == 0 && (year % 25 != 0 || year & 15 == 0)
}

/// A **time** definition field.
///
/// A time must have an hours component, with optional minutes and seconds
/// components. It can also be negative with a starting ‘-’.
///
/// Hour 0 is midnight at the start of the day, and Hour 24 is midnight at the
/// end of the day.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum TimeSpec {
    /// A number of hours.
    Hours(i8),
    /// A number of hours and minutes.
    HoursMinutes(i8, i8),
    /// A number of hours, minutes, and seconds.
    HoursMinutesSeconds(i8, i8, i8),
    /// Zero, or midnight at the start of the day.
    Zero,
}

impl TimeSpec {
    /// Returns the number of seconds past midnight that this time spec
    /// represents.
    pub fn as_seconds(self) -> i64 {
        match self {
            TimeSpec::Hours(h) => h as i64 * 60 * 60,
            TimeSpec::HoursMinutes(h, m) => h as i64 * 60 * 60 + m as i64 * 60,
            TimeSpec::HoursMinutesSeconds(h, m, s) => h as i64 * 60 * 60 + m as i64 * 60 + s as i64,
            TimeSpec::Zero => 0,
        }
    }

    pub fn with_type(self, timetype: TimeType) -> TimeSpecAndType {
        TimeSpecAndType(self, timetype)
    }
}

impl FromStr for TimeSpec {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input == "-" {
            return Ok(TimeSpec::Zero);
        }

        let neg = if input.starts_with('-') { -1 } else { 1 };
        let mut state = TimeSpec::Zero;
        for part in input.split(':') {
            state = match (state, part) {
                (TimeSpec::Zero, hour) => TimeSpec::Hours(
                    i8::from_str(hour)
                        .map_err(|_| Error::InvalidTimeSpecAndType(input.to_string()))?,
                ),
                (TimeSpec::Hours(hours), minutes) if minutes.len() == 2 => TimeSpec::HoursMinutes(
                    hours,
                    i8::from_str(minutes)
                        .map_err(|_| Error::InvalidTimeSpecAndType(input.to_string()))?
                        * neg,
                ),
                (TimeSpec::HoursMinutes(hours, minutes), seconds) if seconds.len() == 2 => {
                    TimeSpec::HoursMinutesSeconds(
                        hours,
                        minutes,
                        i8::from_str(seconds)
                            .map_err(|_| Error::InvalidTimeSpecAndType(input.to_string()))?
                            * neg,
                    )
                }
                _ => return Err(Error::InvalidTimeSpecAndType(input.to_string())),
            };
        }

        Ok(state)
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum TimeType {
    Wall,
    Standard,
    UTC,
}

impl TimeType {
    fn from_char(c: char) -> Option<Self> {
        Some(match c {
            'w' => Self::Wall,
            's' => Self::Standard,
            'u' | 'g' | 'z' => Self::UTC,
            _ => return None,
        })
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct TimeSpecAndType(pub TimeSpec, pub TimeType);

impl FromStr for TimeSpecAndType {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input == "-" {
            return Ok(TimeSpecAndType(TimeSpec::Zero, TimeType::Wall));
        } else if input.chars().all(|c| c == '-' || c.is_ascii_digit()) {
            return Ok(TimeSpecAndType(TimeSpec::from_str(input)?, TimeType::Wall));
        }

        let (input, ty) = match input.chars().last().and_then(TimeType::from_char) {
            Some(ty) => (&input[..input.len() - 1], Some(ty)),
            None => (input, None),
        };

        let spec = TimeSpec::from_str(input)?;
        Ok(TimeSpecAndType(spec, ty.unwrap_or(TimeType::Wall)))
    }
}

/// The time at which the rules change for a location.
///
/// This is described with as few units as possible: a change that occurs at
/// the beginning of the year lists only the year, a change that occurs on a
/// particular day has to list the year, month, and day, and one that occurs
/// at a particular second has to list everything.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ChangeTime {
    /// The earliest point in a particular **year**.
    UntilYear(Year),
    /// The earliest point in a particular **month**.
    UntilMonth(Year, Month),
    /// The earliest point in a particular **day**.
    UntilDay(Year, Month, DaySpec),
    /// The earliest point in a particular **hour, minute, or second**.
    UntilTime(Year, Month, DaySpec, TimeSpecAndType),
}

impl ChangeTime {
    /// Convert this change time to an absolute timestamp, as the number of
    /// seconds since the Unix epoch that the change occurs at.
    pub fn to_timestamp(&self, utc_offset: i64, dst_offset: i64) -> i64 {
        fn seconds_in_year(year: i64) -> i64 {
            if is_leap(year) {
                366 * 24 * 60 * 60
            } else {
                365 * 24 * 60 * 60
            }
        }

        fn seconds_until_start_of_year(year: i64) -> i64 {
            if year >= 1970 {
                (1970..year).map(seconds_in_year).sum()
            } else {
                -(year..1970).map(seconds_in_year).sum::<i64>()
            }
        }

        fn time_to_timestamp(
            year: i64,
            month: i8,
            day: i8,
            hour: i8,
            minute: i8,
            second: i8,
        ) -> i64 {
            const MONTHS_NON_LEAP: [i64; 12] = [
                0,
                31,
                31 + 28,
                31 + 28 + 31,
                31 + 28 + 31 + 30,
                31 + 28 + 31 + 30 + 31,
                31 + 28 + 31 + 30 + 31 + 30,
                31 + 28 + 31 + 30 + 31 + 30 + 31,
                31 + 28 + 31 + 30 + 31 + 30 + 31 + 31,
                31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30,
                31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31,
                31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30,
            ];
            const MONTHS_LEAP: [i64; 12] = [
                0,
                31,
                31 + 29,
                31 + 29 + 31,
                31 + 29 + 31 + 30,
                31 + 29 + 31 + 30 + 31,
                31 + 29 + 31 + 30 + 31 + 30,
                31 + 29 + 31 + 30 + 31 + 30 + 31,
                31 + 29 + 31 + 30 + 31 + 30 + 31 + 31,
                31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30,
                31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31,
                31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30,
            ];
            seconds_until_start_of_year(year)
                + 60 * 60
                    * 24
                    * if is_leap(year) {
                        MONTHS_LEAP[month as usize - 1]
                    } else {
                        MONTHS_NON_LEAP[month as usize - 1]
                    }
                + 60 * 60 * 24 * (day as i64 - 1)
                + 60 * 60 * hour as i64
                + 60 * minute as i64
                + second as i64
        }

        match *self {
            ChangeTime::UntilYear(Year::Number(y)) => {
                time_to_timestamp(y, 1, 1, 0, 0, 0) - (utc_offset + dst_offset)
            }
            ChangeTime::UntilMonth(Year::Number(y), m) => {
                time_to_timestamp(y, m as i8, 1, 0, 0, 0) - (utc_offset + dst_offset)
            }
            ChangeTime::UntilDay(Year::Number(y), m, d) => {
                let (m, wd) = d.to_concrete_day(y, m);
                time_to_timestamp(y, m as i8, wd, 0, 0, 0) - (utc_offset + dst_offset)
            }
            ChangeTime::UntilTime(Year::Number(y), m, d, time) => {
                (match time.0 {
                    TimeSpec::Zero => {
                        let (m, wd) = d.to_concrete_day(y, m);
                        time_to_timestamp(y, m as i8, wd, 0, 0, 0)
                    }
                    TimeSpec::Hours(h) => {
                        let (m, wd) = d.to_concrete_day(y, m);
                        time_to_timestamp(y, m as i8, wd, h, 0, 0)
                    }
                    TimeSpec::HoursMinutes(h, min) => {
                        let (m, wd) = d.to_concrete_day(y, m);
                        time_to_timestamp(y, m as i8, wd, h, min, 0)
                    }
                    TimeSpec::HoursMinutesSeconds(h, min, s) => {
                        let (m, wd) = d.to_concrete_day(y, m);
                        time_to_timestamp(y, m as i8, wd, h, min, s)
                    }
                }) - match time.1 {
                    TimeType::UTC => 0,
                    TimeType::Standard => utc_offset,
                    TimeType::Wall => utc_offset + dst_offset,
                }
            }

            _ => unreachable!(),
        }
    }

    pub fn year(&self) -> i64 {
        match *self {
            ChangeTime::UntilYear(Year::Number(y)) => y,
            ChangeTime::UntilMonth(Year::Number(y), ..) => y,
            ChangeTime::UntilDay(Year::Number(y), ..) => y,
            ChangeTime::UntilTime(Year::Number(y), ..) => y,
            _ => unreachable!(),
        }
    }
}

/// The information contained in both zone lines *and* zone continuation lines.
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct ZoneInfo<'a> {
    /// The amount of time that needs to be added to UTC to get the standard
    /// time in this zone.
    pub utc_offset: TimeSpec,
    /// The name of all the rules that should apply in the time zone, or the
    /// amount of time to add.
    pub saving: Saving<'a>,
    /// The format for time zone abbreviations, with `%s` as the string marker.
    pub format: &'a str,
    /// The time at which the rules change for this location, or `None` if
    /// these rules are in effect until the end of time (!).
    pub time: Option<ChangeTime>,
}

impl<'a> ZoneInfo<'a> {
    fn from_iter(iter: impl Iterator<Item = &'a str>) -> Result<Self, Error> {
        let mut state = ZoneInfoState::Start;
        for part in iter {
            state = match (state, part) {
                // In theory a comment is allowed to come after a field without preceding
                // whitespace, but this doesn't seem to be used in practice.
                (st, _) if part.starts_with('#') => {
                    state = st;
                    break;
                }
                (ZoneInfoState::Start, offset) => ZoneInfoState::Save {
                    offset: TimeSpec::from_str(offset)?,
                },
                (ZoneInfoState::Save { offset }, saving) => ZoneInfoState::Format {
                    offset,
                    saving: Saving::from_str(saving)?,
                },
                (ZoneInfoState::Format { offset, saving }, format) => ZoneInfoState::Year {
                    offset,
                    saving,
                    format,
                },
                (
                    ZoneInfoState::Year {
                        offset,
                        saving,
                        format,
                    },
                    year,
                ) => ZoneInfoState::Month {
                    offset,
                    saving,
                    format,
                    year: Year::from_str(year)?,
                },
                (
                    ZoneInfoState::Month {
                        offset,
                        saving,
                        format,
                        year,
                    },
                    month,
                ) => ZoneInfoState::Day {
                    offset,
                    saving,
                    format,
                    year,
                    month: Month::from_str(month)?,
                },
                (
                    ZoneInfoState::Day {
                        offset,
                        saving,
                        format,
                        year,
                        month,
                    },
                    day,
                ) => ZoneInfoState::Time {
                    offset,
                    saving,
                    format,
                    year,
                    month,
                    day: DaySpec::from_str(day)?,
                },
                (
                    ZoneInfoState::Time {
                        offset,
                        saving,
                        format,
                        year,
                        month,
                        day,
                    },
                    time,
                ) => {
                    return Ok(Self {
                        utc_offset: offset,
                        saving,
                        format,
                        time: Some(ChangeTime::UntilTime(
                            year,
                            month,
                            day,
                            TimeSpecAndType::from_str(time)?,
                        )),
                    })
                }
            };
        }

        match state {
            ZoneInfoState::Start | ZoneInfoState::Save { .. } | ZoneInfoState::Format { .. } => {
                Err(Error::NotParsedAsZoneLine)
            }
            ZoneInfoState::Year {
                offset,
                saving,
                format,
            } => Ok(Self {
                utc_offset: offset,
                saving,
                format,
                time: None,
            }),
            ZoneInfoState::Month {
                offset,
                saving,
                format,
                year,
            } => Ok(Self {
                utc_offset: offset,
                saving,
                format,
                time: Some(ChangeTime::UntilYear(year)),
            }),
            ZoneInfoState::Day {
                offset,
                saving,
                format,
                year,
                month,
            } => Ok(Self {
                utc_offset: offset,
                saving,
                format,
                time: Some(ChangeTime::UntilMonth(year, month)),
            }),
            ZoneInfoState::Time {
                offset,
                saving,
                format,
                year,
                month,
                day,
            } => Ok(Self {
                utc_offset: offset,
                saving,
                format,
                time: Some(ChangeTime::UntilDay(year, month, day)),
            }),
        }
    }
}

enum ZoneInfoState<'a> {
    Start,
    Save {
        offset: TimeSpec,
    },
    Format {
        offset: TimeSpec,
        saving: Saving<'a>,
    },
    Year {
        offset: TimeSpec,
        saving: Saving<'a>,
        format: &'a str,
    },
    Month {
        offset: TimeSpec,
        saving: Saving<'a>,
        format: &'a str,
        year: Year,
    },
    Day {
        offset: TimeSpec,
        saving: Saving<'a>,
        format: &'a str,
        year: Year,
        month: Month,
    },
    Time {
        offset: TimeSpec,
        saving: Saving<'a>,
        format: &'a str,
        year: Year,
        month: Month,
        day: DaySpec,
    },
}

/// The amount of daylight saving time (DST) to apply to this timespan. This
/// is a special type for a certain field in a zone line, which can hold
/// different types of value.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Saving<'a> {
    /// Just stick to the base offset.
    NoSaving,
    /// This amount of time should be saved while this timespan is in effect.
    /// (This is the equivalent to there being a single one-off rule with the
    /// given amount of time to save).
    OneOff(TimeSpec),
    /// All rules with the given name should apply while this timespan is in
    /// effect.
    Multiple(&'a str),
}

impl<'a> Saving<'a> {
    fn from_str(input: &'a str) -> Result<Self, Error> {
        if input == "-" {
            Ok(Self::NoSaving)
        } else if input
            .chars()
            .all(|c| c == '-' || c == '_' || c.is_alphabetic())
        {
            Ok(Self::Multiple(input))
        } else if let Ok(time) = TimeSpec::from_str(input) {
            Ok(Self::OneOff(time))
        } else {
            Err(Error::CouldNotParseSaving(input.to_string()))
        }
    }
}

/// A **rule** definition line.
///
/// According to the `zic(8)` man page, a rule line has this form, along with
/// an example:
///
/// ```text
///     Rule  NAME  FROM  TO    TYPE  IN   ON       AT    SAVE  LETTER/S
///     Rule  US    1967  1973  ‐     Apr  lastSun  2:00  1:00  D
/// ```
///
/// Apart from the opening `Rule` to specify which kind of line this is, and
/// the `type` column, every column in the line has a field in this struct.
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Rule<'a> {
    /// The name of the set of rules that this rule is part of.
    pub name: &'a str,
    /// The first year in which the rule applies.
    pub from_year: Year,
    /// The final year, or `None` if’s ‘only’.
    pub to_year: Option<Year>,
    /// The month in which the rule takes effect.
    pub month: Month,
    /// The day on which the rule takes effect.
    pub day: DaySpec,
    /// The time of day at which the rule takes effect.
    pub time: TimeSpecAndType,
    /// The amount of time to be added when the rule is in effect.
    pub time_to_add: TimeSpec,
    /// The variable part of time zone abbreviations to be used when this rule
    /// is in effect, if any.
    pub letters: Option<&'a str>,
}

impl<'a> Rule<'a> {
    fn from_str(input: &'a str) -> Result<Self, Error> {
        let mut state = RuleState::Start;
        // Not handled: quoted strings, parts of which are allowed to contain whitespace.
        // Extra complexity does not seem worth it while they don't seem to be used in practice.
        for part in input.split_ascii_whitespace() {
            if part.starts_with('#') {
                continue;
            }

            state = match (state, part) {
                (RuleState::Start, "Rule") => RuleState::Name,
                (RuleState::Name, name) => RuleState::FromYear { name },
                (RuleState::FromYear { name }, year) => RuleState::ToYear {
                    name,
                    from_year: Year::from_str(year)?,
                },
                (RuleState::ToYear { name, from_year }, year) => RuleState::Type {
                    name,
                    from_year,
                    // The end year can be ‘only’ to indicate that this rule only
                    // takes place on that year.
                    to_year: match year {
                        "only" => None,
                        _ => Some(Year::from_str(year)?),
                    },
                },
                // According to the spec, the only value inside the ‘type’ column
                // should be “-”, so throw an error if it isn’t. (It only exists
                // for compatibility with old versions that used to contain year
                // types.) Sometimes “‐”, a Unicode hyphen, is used as well.
                (
                    RuleState::Type {
                        name,
                        from_year,
                        to_year,
                    },
                    "-" | "\u{2010}",
                ) => RuleState::Month {
                    name,
                    from_year,
                    to_year,
                },
                (RuleState::Type { .. }, _) => {
                    return Err(Error::TypeColumnContainedNonHyphen(part.to_string()))
                }
                (
                    RuleState::Month {
                        name,
                        from_year,
                        to_year,
                    },
                    month,
                ) => RuleState::Day {
                    name,
                    from_year,
                    to_year,
                    month: Month::from_str(month)?,
                },
                (
                    RuleState::Day {
                        name,
                        from_year,
                        to_year,
                        month,
                    },
                    day,
                ) => RuleState::Time {
                    name,
                    from_year,
                    to_year,
                    month,
                    day: DaySpec::from_str(day)?,
                },
                (
                    RuleState::Time {
                        name,
                        from_year,
                        to_year,
                        month,
                        day,
                    },
                    time,
                ) => RuleState::TimeToAdd {
                    name,
                    from_year,
                    to_year,
                    month,
                    day,
                    time: TimeSpecAndType::from_str(time)?,
                },
                (
                    RuleState::TimeToAdd {
                        name,
                        from_year,
                        to_year,
                        month,
                        day,
                        time,
                    },
                    time_to_add,
                ) => RuleState::Letters {
                    name,
                    from_year,
                    to_year,
                    month,
                    day,
                    time,
                    time_to_add: TimeSpec::from_str(time_to_add)?,
                },
                (
                    RuleState::Letters {
                        name,
                        from_year,
                        to_year,
                        month,
                        day,
                        time,
                        time_to_add,
                    },
                    letters,
                ) => {
                    return Ok(Self {
                        name,
                        from_year,
                        to_year,
                        month,
                        day,
                        time,
                        time_to_add,
                        letters: match letters {
                            "-" => None,
                            _ => Some(letters),
                        },
                    })
                }
                _ => return Err(Error::NotParsedAsRuleLine),
            };
        }

        Err(Error::NotParsedAsRuleLine)
    }
}

enum RuleState<'a> {
    Start,
    Name,
    FromYear {
        name: &'a str,
    },
    ToYear {
        name: &'a str,
        from_year: Year,
    },
    Type {
        name: &'a str,
        from_year: Year,
        to_year: Option<Year>,
    },
    Month {
        name: &'a str,
        from_year: Year,
        to_year: Option<Year>,
    },
    Day {
        name: &'a str,
        from_year: Year,
        to_year: Option<Year>,
        month: Month,
    },
    Time {
        name: &'a str,
        from_year: Year,
        to_year: Option<Year>,
        month: Month,
        day: DaySpec,
    },
    TimeToAdd {
        name: &'a str,
        from_year: Year,
        to_year: Option<Year>,
        month: Month,
        day: DaySpec,
        time: TimeSpecAndType,
    },
    Letters {
        name: &'a str,
        from_year: Year,
        to_year: Option<Year>,
        month: Month,
        day: DaySpec,
        time: TimeSpecAndType,
        time_to_add: TimeSpec,
    },
}

/// A **zone** definition line.
///
/// According to the `zic(8)` man page, a zone line has this form, along with
/// an example:
///
/// ```text
///     Zone  NAME                GMTOFF  RULES/SAVE  FORMAT  [UNTILYEAR [MONTH [DAY [TIME]]]]
///     Zone  Australia/Adelaide  9:30    Aus         AC%sT   1971       Oct    31   2:00
/// ```
///
/// The opening `Zone` identifier is ignored, and the last four columns are
/// all optional, with their variants consolidated into a `ChangeTime`.
///
/// The `Rules/Save` column, if it contains a value, *either* contains the
/// name of the rules to use for this zone, *or* contains a one-off period of
/// time to save.
///
/// A continuation rule line contains all the same fields apart from the
/// `Name` column and the opening `Zone` identifier.
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Zone<'a> {
    /// The name of the time zone.
    pub name: &'a str,
    /// All the other fields of info.
    pub info: ZoneInfo<'a>,
}

impl<'a> Zone<'a> {
    fn from_str(input: &'a str) -> Result<Self, Error> {
        let mut iter = input.split_ascii_whitespace();
        if iter.next() != Some("Zone") {
            return Err(Error::NotParsedAsZoneLine);
        }

        let name = match iter.next() {
            Some(name) => name,
            None => return Err(Error::NotParsedAsZoneLine),
        };

        Ok(Self {
            name,
            info: ZoneInfo::from_iter(iter)?,
        })
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Link<'a> {
    pub existing: &'a str,
    pub new: &'a str,
}

impl<'a> Link<'a> {
    fn from_str(input: &'a str) -> Result<Self, Error> {
        let mut iter = input.split_ascii_whitespace();
        if iter.next() != Some("Link") {
            return Err(Error::NotParsedAsLinkLine);
        }

        Ok(Link {
            existing: iter.next().ok_or(Error::NotParsedAsLinkLine)?,
            new: iter.next().ok_or(Error::NotParsedAsLinkLine)?,
        })
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Line<'a> {
    /// This line is empty.
    Space,
    /// This line contains a **zone** definition.
    Zone(Zone<'a>),
    /// This line contains a **continuation** of a zone definition.
    Continuation(ZoneInfo<'a>),
    /// This line contains a **rule** definition.
    Rule(Rule<'a>),
    /// This line contains a **link** definition.
    Link(Link<'a>),
}

impl<'a> Line<'a> {
    /// Attempt to parse this line, returning a `Line` depending on what
    /// type of line it was, or an `Error` if it couldn't be parsed.
    pub fn new(input: &'a str) -> Result<Line<'a>, Error> {
        let input = match input.split_once('#') {
            Some((input, _)) => input,
            None => input,
        };

        if input.trim().is_empty() {
            return Ok(Line::Space);
        }

        if input.starts_with("Zone") {
            return Ok(Line::Zone(Zone::from_str(input)?));
        }

        if input.starts_with(&[' ', '\t'][..]) {
            return Ok(Line::Continuation(ZoneInfo::from_iter(
                input.split_ascii_whitespace(),
            )?));
        }

        if input.starts_with("Rule") {
            return Ok(Line::Rule(Rule::from_str(input)?));
        }

        if input.starts_with("Link") {
            return Ok(Line::Link(Link::from_str(input)?));
        }

        Err(Error::InvalidLineType(input.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays() {
        assert_eq!(
            Weekday::calculate(1970, Month::January, 1),
            Weekday::Thursday
        );
        assert_eq!(
            Weekday::calculate(2017, Month::February, 11),
            Weekday::Saturday
        );
        assert_eq!(Weekday::calculate(1890, Month::March, 2), Weekday::Sunday);
        assert_eq!(Weekday::calculate(2100, Month::April, 20), Weekday::Tuesday);
        assert_eq!(Weekday::calculate(2009, Month::May, 31), Weekday::Sunday);
        assert_eq!(Weekday::calculate(2001, Month::June, 9), Weekday::Saturday);
        assert_eq!(Weekday::calculate(1995, Month::July, 21), Weekday::Friday);
        assert_eq!(Weekday::calculate(1982, Month::August, 8), Weekday::Sunday);
        assert_eq!(
            Weekday::calculate(1962, Month::September, 6),
            Weekday::Thursday
        );
        assert_eq!(
            Weekday::calculate(1899, Month::October, 14),
            Weekday::Saturday
        );
        assert_eq!(
            Weekday::calculate(2016, Month::November, 18),
            Weekday::Friday
        );
        assert_eq!(
            Weekday::calculate(2010, Month::December, 19),
            Weekday::Sunday
        );
        assert_eq!(
            Weekday::calculate(2016, Month::February, 29),
            Weekday::Monday
        );
    }

    #[test]
    fn last_monday() {
        let dayspec = DaySpec::Last(Weekday::Monday);
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::January),
            (Month::January, 25)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::February),
            (Month::February, 29)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::March),
            (Month::March, 28)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::April),
            (Month::April, 25)
        );
        assert_eq!(dayspec.to_concrete_day(2016, Month::May), (Month::May, 30));
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::June),
            (Month::June, 27)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::July),
            (Month::July, 25)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::August),
            (Month::August, 29)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::September),
            (Month::September, 26)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::October),
            (Month::October, 31)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::November),
            (Month::November, 28)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::December),
            (Month::December, 26)
        );
    }

    #[test]
    fn first_monday_on_or_after() {
        let dayspec = DaySpec::FirstOnOrAfter(Weekday::Monday, 20);
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::January),
            (Month::January, 25)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::February),
            (Month::February, 22)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::March),
            (Month::March, 21)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::April),
            (Month::April, 25)
        );
        assert_eq!(dayspec.to_concrete_day(2016, Month::May), (Month::May, 23));
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::June),
            (Month::June, 20)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::July),
            (Month::July, 25)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::August),
            (Month::August, 22)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::September),
            (Month::September, 26)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::October),
            (Month::October, 24)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::November),
            (Month::November, 21)
        );
        assert_eq!(
            dayspec.to_concrete_day(2016, Month::December),
            (Month::December, 26)
        );
    }

    // A couple of specific timezone transitions that we care about
    #[test]
    fn first_sunday_in_toronto() {
        let dayspec = DaySpec::FirstOnOrAfter(Weekday::Sunday, 25);
        assert_eq!(dayspec.to_concrete_day(1932, Month::April), (Month::May, 1));
        // asia/zion
        let dayspec = DaySpec::LastOnOrBefore(Weekday::Friday, 1);
        assert_eq!(
            dayspec.to_concrete_day(2012, Month::April),
            (Month::March, 30)
        );
    }

    #[test]
    fn to_timestamp() {
        let time = ChangeTime::UntilYear(Year::Number(1970));
        assert_eq!(time.to_timestamp(0, 0), 0);
        let time = ChangeTime::UntilYear(Year::Number(2016));
        assert_eq!(time.to_timestamp(0, 0), 1451606400);
        let time = ChangeTime::UntilYear(Year::Number(1900));
        assert_eq!(time.to_timestamp(0, 0), -2208988800);
        let time = ChangeTime::UntilTime(
            Year::Number(2000),
            Month::February,
            DaySpec::Last(Weekday::Sunday),
            TimeSpecAndType(TimeSpec::Hours(9), TimeType::Wall),
        );
        assert_eq!(time.to_timestamp(3600, 3600), 951642000 - 2 * 3600);
    }

    macro_rules! test {
        ($name:ident: $input:expr => $result:expr) => {
            #[test]
            fn $name() {
                assert_eq!(Line::new($input), $result);
            }
        };
    }

    test!(empty:    ""          => Ok(Line::Space));
    test!(spaces:   "        "  => Ok(Line::Space));

    test!(rule_1: "Rule  US    1967  1973  ‐     Apr  lastSun  2:00  1:00  D" => Ok(Line::Rule(Rule {
        name:         "US",
        from_year:    Year::Number(1967),
        to_year:      Some(Year::Number(1973)),
        month:        Month::April,
        day:          DaySpec::Last(Weekday::Sunday),
        time:         TimeSpec::HoursMinutes(2, 0).with_type(TimeType::Wall),
        time_to_add:  TimeSpec::HoursMinutes(1, 0),
        letters:      Some("D"),
    })));

    test!(rule_2: "Rule	Greece	1976	only	-	Oct	10	2:00s	0	-" => Ok(Line::Rule(Rule {
        name:         "Greece",
        from_year:    Year::Number(1976),
        to_year:      None,
        month:        Month::October,
        day:          DaySpec::Ordinal(10),
        time:         TimeSpec::HoursMinutes(2, 0).with_type(TimeType::Standard),
        time_to_add:  TimeSpec::Hours(0),
        letters:      None,
    })));

    test!(rule_3: "Rule	EU	1977	1980	-	Apr	Sun>=1	 1:00u	1:00	S" => Ok(Line::Rule(Rule {
        name:         "EU",
        from_year:    Year::Number(1977),
        to_year:      Some(Year::Number(1980)),
        month:        Month::April,
        day:          DaySpec::FirstOnOrAfter(Weekday::Sunday, 1),
        time:         TimeSpec::HoursMinutes(1, 0).with_type(TimeType::UTC),
        time_to_add:  TimeSpec::HoursMinutes(1, 0),
        letters:      Some("S"),
    })));

    test!(no_hyphen: "Rule	EU	1977	1980	HEY	Apr	Sun>=1	 1:00u	1:00	S"         => Err(Error::TypeColumnContainedNonHyphen("HEY".to_string())));
    test!(bad_month: "Rule	EU	1977	1980	-	Febtober	Sun>=1	 1:00u	1:00	S" => Err(Error::FailedMonthParse("febtober".to_string())));

    test!(zone: "Zone  Australia/Adelaide  9:30    Aus         AC%sT   1971 Oct 31  2:00:00" => Ok(Line::Zone(Zone {
        name: "Australia/Adelaide",
        info: ZoneInfo {
            utc_offset:  TimeSpec::HoursMinutes(9, 30),
            saving:      Saving::Multiple("Aus"),
            format:      "AC%sT",
            time:        Some(ChangeTime::UntilTime(Year::Number(1971), Month::October, DaySpec::Ordinal(31), TimeSpec::HoursMinutesSeconds(2, 0, 0).with_type(TimeType::Wall))),
        },
    })));

    test!(continuation_1: "                          9:30    Aus         AC%sT   1971 Oct 31  2:00:00" => Ok(Line::Continuation(ZoneInfo {
        utc_offset:  TimeSpec::HoursMinutes(9, 30),
        saving:      Saving::Multiple("Aus"),
        format:      "AC%sT",
        time:        Some(ChangeTime::UntilTime(Year::Number(1971), Month::October, DaySpec::Ordinal(31), TimeSpec::HoursMinutesSeconds(2, 0, 0).with_type(TimeType::Wall))),
    })));

    test!(continuation_2: "			1:00	C-Eur	CE%sT	1943 Oct 25" => Ok(Line::Continuation(ZoneInfo {
        utc_offset:  TimeSpec::HoursMinutes(1, 00),
        saving:      Saving::Multiple("C-Eur"),
        format:      "CE%sT",
        time:        Some(ChangeTime::UntilDay(Year::Number(1943), Month::October, DaySpec::Ordinal(25))),
    })));

    test!(zone_hyphen: "Zone Asia/Ust-Nera\t 9:32:54 -\tLMT\t1919" => Ok(Line::Zone(Zone {
        name: "Asia/Ust-Nera",
        info: ZoneInfo {
            utc_offset:  TimeSpec::HoursMinutesSeconds(9, 32, 54),
            saving:      Saving::NoSaving,
            format:      "LMT",
            time:        Some(ChangeTime::UntilYear(Year::Number(1919))),
        },
    })));

    #[test]
    fn negative_offsets() {
        static LINE: &str = "Zone    Europe/London   -0:01:15 -  LMT 1847 Dec  1  0:00s";
        let zone = Zone::from_str(LINE).unwrap();
        assert_eq!(
            zone.info.utc_offset,
            TimeSpec::HoursMinutesSeconds(0, -1, -15)
        );
    }

    #[test]
    fn negative_offsets_2() {
        static LINE: &str =
            "Zone        Europe/Madrid   -0:14:44 -      LMT     1901 Jan  1  0:00s";
        let zone = Zone::from_str(LINE).unwrap();
        assert_eq!(
            zone.info.utc_offset,
            TimeSpec::HoursMinutesSeconds(0, -14, -44)
        );
    }

    #[test]
    fn negative_offsets_3() {
        static LINE: &str = "Zone America/Danmarkshavn -1:14:40 -    LMT 1916 Jul 28";
        let zone = Zone::from_str(LINE).unwrap();
        assert_eq!(
            zone.info.utc_offset,
            TimeSpec::HoursMinutesSeconds(-1, -14, -40)
        );
    }

    test!(link: "Link  Europe/Istanbul  Asia/Istanbul" => Ok(Line::Link(Link {
        existing:  "Europe/Istanbul",
        new:       "Asia/Istanbul",
    })));

    #[test]
    fn month() {
        assert_eq!(Month::from_str("Aug"), Ok(Month::August));
        assert_eq!(Month::from_str("December"), Ok(Month::December));
    }

    test!(golb: "GOLB" => Err(Error::InvalidLineType("GOLB".to_string())));

    test!(comment: "# this is a comment" => Ok(Line::Space));
    test!(another_comment: "     # so is this" => Ok(Line::Space));
    test!(multiple_hash: "     # so is this ## " => Ok(Line::Space));
    test!(non_comment: " this is not a # comment" => Err(Error::InvalidTimeSpecAndType("this".to_string())));

    test!(comment_after: "Link  Europe/Istanbul  Asia/Istanbul #with a comment after" => Ok(Line::Link(Link {
        existing:  "Europe/Istanbul",
        new:       "Asia/Istanbul",
    })));

    test!(two_comments_after: "Link  Europe/Istanbul  Asia/Istanbul   # comment ## comment" => Ok(Line::Link(Link {
        existing:  "Europe/Istanbul",
        new:       "Asia/Istanbul",
    })));

    #[test]
    fn leap_years() {
        assert!(!is_leap(1900));
        assert!(is_leap(1904));
        assert!(is_leap(1964));
        assert!(is_leap(1996));
        assert!(!is_leap(1997));
        assert!(!is_leap(1997));
        assert!(!is_leap(1999));
        assert!(is_leap(2000));
        assert!(is_leap(2016));
        assert!(!is_leap(2100));
    }
}
