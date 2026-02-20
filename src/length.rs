// Restored from icu_datetime 1
// https://docs.rs/icu_datetime/1.5.1/src/icu_datetime/options/length.rs.html

use icu_datetime::fieldsets::builder::{FieldSetBuilder, ZoneStyle};
use icu_datetime::fieldsets::enums::CompositeDateTimeFieldSet;
use icu_datetime::options::TimePrecision;

/// Represents different lengths a date part can be formatted into.
/// Each length has associated best pattern for it for a given locale.
///
/// The available lengths correspond to [`UTS #35: Unicode LDML 4. Dates`], section 2.4 [`Element dateFormats`].
///
/// [`UTS #35: Unicode LDML 4. Dates`]: https://unicode.org/reports/tr35/tr35-dates.html
/// [`Element dateFormats`]: https://unicode.org/reports/tr35/tr35-dates.html#dateFormats
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Date {
    /// Full length, usually with weekday name.
    ///
    /// # Examples
    ///
    /// * Tuesday, January 21, 2020 (`en-US`)
    /// * wtorek, 21 stycznia, 2020 (`pl`)
    /// * الثلاثاء، ٢١ يناير ٢٠٢٠ (`ar`)
    /// * вторник, 21 января 2020 г. (`ru`)
    /// * 2020年1月21日火曜日 (`ja`)
    Full,
    /// Long length, with wide month name.
    ///
    /// # Examples
    ///
    /// * September 10, 2020 (`en-US`)
    /// * 10 września 2020 (`pl`)
    /// * ١٠ سبتمبر ٢٠٢٠ (`ar`)
    /// * 10 сентября 2020 г. (`ru`)
    /// * 2020年9月10日 (`ja`)
    Long,
    /// Medium length.
    ///
    /// # Examples
    ///
    /// * Feb 20, 2020 (`en-US`)
    /// * 20 lut 2020 (`pl`)
    /// * ٢٠‏/٠٢‏/٢٠٢٠ (`ar`)
    /// * 20 февр. 2020 г. (`ru`)
    /// * 2020/02/20 (`ja`)
    Medium,
    /// Short length, usually with numeric month.
    ///
    /// # Examples
    ///
    /// * 1/30/20 (`en-US`)
    /// * 30.01.2020 (`pl`)
    /// * ٣٠‏/١‏/٢٠٢٠ (`ar`)
    /// * 30.01.2020 (`ru`)
    /// * 2020/01/30 (`ja`)
    Short,
}

/// Represents different length lengths a time part can be formatted into.
/// Each length has associated best pattern for it for a given locale.
///
/// The available lengths correspond to [`UTS #35: Unicode LDML 4. Dates`], section 2.4 [`Element timeFormats`].
///
/// [`UTS #35: Unicode LDML 4. Dates`]: https://unicode.org/reports/tr35/tr35-dates.html
/// [`Element dateFormats`]: https://unicode.org/reports/tr35/tr35-dates.html#timeFormats
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Time {
    /// Full length, with spelled out time zone name.
    ///
    /// # Examples
    ///
    /// * 8:25:07 AM Pacific Standard Time (`en-US`)
    /// * 08:25:07 czas pacyficzny standardowy (`pl`)
    /// * ٨:٢٥:٠٧ ص توقيت المحيط الهادي الرسمي (`ar`)
    /// * 08:25:07 Тихоокеанское стандартное время (`ru`)
    /// * 8時25分07秒 アメリカ太平洋標準時 (`ja`)
    Full,
    /// Full length, usually with short time-zone code.
    ///
    /// # Examples
    ///
    /// * 8:25:07 AM PST (`en-US`)
    /// * 08:25:07 GMT-8 (`pl`)
    /// * ٨:٢٥:٠٧ ص غرينتش-٨ (`ar`)
    /// * 08:25:07 GMT-8 (`ru`)
    /// * 8:25:07 GMT-8 (`ja`)
    Long,
    /// Full length, usually with seconds.
    ///
    /// # Examples
    ///
    /// * 8:25:07 AM (`en-US`)
    /// * 08:25:07 (`pl`)
    /// * ٨:٢٥:٠٧ ص (`ar`)
    /// * 08:25:07 (`ru`)
    /// * 8:25:07 (`ja`)
    Medium,
    /// Full length, usually without seconds.
    ///
    /// # Examples
    ///
    /// * 8:25 AM (`en-US`)
    /// * 08:25 (`pl`)
    /// * ٨:٢٥ ص (`ar`)
    /// * 08:25 (`ru`)
    /// * 8:25 (`ja`)
    Short,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(super) struct Bag {
    /// Configure the date part of the datetime.
    pub date: Option<Date>,
    /// Configure the time part of the datetime.
    pub time: Option<Time>,
}

impl Bag {
    /// Constructs a `Bag` with all fields set to `None`.
    ///
    /// Note that the [`Default`] implementation returns medium date and time options
    pub fn empty() -> Self {
        Self {
            date: None,
            time: None,
        }
    }

    // For a Copy type, is it as_ or to_?
    pub fn as_fieldset(self) -> CompositeDateTimeFieldSet {
        use icu_datetime::options;
        let (date, time) = if self == Self::empty() {
            (Some(Date::Short), None)
        } else {
            (self.date, self.time)
        };
        let mut builder = FieldSetBuilder::new();
        if let Some(date) = date {
            builder.date_fields = Some(if date == Date::Full {
                icu_datetime::fieldsets::builder::DateFields::YMDE
            } else {
                icu_datetime::fieldsets::builder::DateFields::YMD
            });
            builder.length = Some(match date {
                Date::Full | Date::Long => options::Length::Long,
                Date::Medium => options::Length::Medium,
                Date::Short => options::Length::Short,
            });
        }
        if let Some(time) = time {
            builder.time_precision = Some(if time == Time::Short {
                TimePrecision::Minute
            } else {
                TimePrecision::Second
            });
            if time == Time::Full {
                builder.zone_style = Some(ZoneStyle::SpecificLong);
            } else if time == Time::Long {
                builder.zone_style = Some(ZoneStyle::SpecificShort)
            }
        }
        // If we set any incompatible options, it's a bug
        builder.build_composite_datetime().unwrap()
    }
}
