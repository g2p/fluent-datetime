//! Length is a model of encoding information on how to format date and time by
//! specifying the preferred length ! of date and time fields.
//!
//! It is intended to represent dateStyle and timeStyle from
//! ECMA 402 DateTimeFormat constructor options.
//!
//! If either of the fields is omitted, the value will be formatted according to
//! the pattern associated with the ! preferred length of the present field in a
//! given locale.
//!
//! If both fields are present, both parts of the value will be formatted and an
//! additional connector pattern ! will be used to construct a full result.
//! The type of the connector is determined by the length of the [`Date`] field.

// Restored from icu_datetime 1
// https://docs.rs/icu_datetime/1.5.1/src/icu_datetime/options/length.rs.html

use icu_datetime::fieldsets::builder::{DateFields, FieldSetBuilder, ZoneStyle};
use icu_datetime::options::{Length, TimePrecision};

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
    // https://rust-lang.github.io/api-guidelines/naming.html#ad-hoc-conversions-follow-as_-to_-into_-conventions-c-conv
    pub(super) fn to_fieldset_builder(self) -> FieldSetBuilder {
        let (date, time) = if self == Self::empty() {
            (Some(Date::Short), None)
        } else {
            (self.date, self.time)
        };
        let mut builder = FieldSetBuilder::new();
        if let Some(date) = date {
            builder.date_fields = Some(if date == Date::Full {
                DateFields::YMDE
            } else {
                DateFields::YMD
            });
            builder.length = Some(match date {
                Date::Full | Date::Long => Length::Long,
                Date::Medium => Length::Medium,
                Date::Short => Length::Short,
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
        builder
    }
}
