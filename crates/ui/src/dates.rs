//! The calendar the screens lay days on: a local day (days since 1970-01-01 on the device's
//! clock) as a date and a weekday; the kept versions' labels and a mode's calendar show them.

/// A date of the proleptic Gregorian calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i64,
    /// 1 to 12.
    pub month: u32,
    /// 1 to 31.
    pub day: u32,
}

impl Date {
    /// The date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
    #[must_use]
    pub fn of(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        Self {
            year: yoe + era * 400 + i64::from(month <= 2),
            month: u32::try_from(month).unwrap_or(1),
            day: u32::try_from(day).unwrap_or(1),
        }
    }

    /// Days since 1970-01-01 (`days_from_civil`).
    #[must_use]
    pub fn days(self) -> i64 {
        let year = self.year - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let yoe = year - era * 400;
        let mp = i64::from((self.month + 9) % 12);
        let doy = (153 * mp + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The first day of its month.
    #[must_use]
    pub const fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    /// The first day of the month `by` months on (back when negative).
    #[must_use]
    pub fn months_on(self, by: i64) -> Self {
        let index = self.year * 12 + i64::from(self.month) - 1 + by;
        Self {
            year: index.div_euclid(12),
            month: u32::try_from(index.rem_euclid(12) + 1).unwrap_or(1),
            day: 1,
        }
    }

    /// How many days its month holds.
    #[must_use]
    pub fn days_in_month(self) -> u32 {
        let next = self.months_on(1).days();
        u32::try_from(next - self.first_of_month().days()).unwrap_or(30)
    }
}

/// The weekday of local day `day`: 0 Monday to 6 Sunday (1970-01-01 was a Thursday).
#[must_use]
pub fn weekday(day: i64) -> u32 {
    u32::try_from((day + 3).rem_euclid(7)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_weekdays_read_as_the_calendar_has_them() {
        let october = Date {
            year: 2026,
            month: 10,
            day: 5,
        };
        assert_eq!(Date::of(20_731), october);
        assert_eq!(october.days(), 20_731);
        assert_eq!(weekday(20_731), 0, "a Monday");
        assert_eq!(weekday(0), 3, "1970-01-01, a Thursday");
        assert_eq!(october.days_in_month(), 31);
        let february = Date {
            year: 2024,
            month: 2,
            day: 29,
        };
        assert_eq!(Date::of(february.days()), february);
        assert_eq!(february.days_in_month(), 29);
        assert_eq!(
            october.months_on(3),
            Date {
                year: 2027,
                month: 1,
                day: 1
            }
        );
        assert_eq!(october.months_on(-10).month, 12);
        for days in [-800_000, -1, 0, 59, 365, 20_731, 900_000] {
            assert_eq!(Date::of(days).days(), days);
        }
    }
}
