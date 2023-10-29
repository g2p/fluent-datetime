//! # International datetimes in Fluent translations
//!
//! fluent-datetime uses [ICU4X], in particular [`icu_datetime`] and
//! [`icu_calendar`], to format datetimes internationally within
//! a [Fluent] translation.
//!
//! [Fluent]: https://projectfluent.org/
//! [ICU4X]: https://github.com/unicode-org/icu4x
//!
//! # Example
//!
//! This example uses [`fluent_bundle`] directly.
//!
//! You may prefer to use less verbose integrations; in which case the
//! [`bundle.add_datetime_support()`](BundleExt::add_datetime_support)
//! line is the only one you need.
//!
//! ```rust
//! use fluent::fluent_args;
//! use fluent_bundle::{FluentBundle, FluentResource};
//! use fluent_datetime::{BundleExt, FluentDateTime};
//! use icu_calendar::DateTime;
//! use icu_datetime::options::length;
//! use unic_langid::LanguageIdentifier;
//!
//! // Create a FluentBundle
//! let langid_en: LanguageIdentifier = "en-US".parse()?;
//! let mut bundle = FluentBundle::new(vec![langid_en]);
//!
//! // Register the DATETIME function
//! bundle.add_datetime_support();
//!
//! // Add a FluentResource to the bundle
//! let ftl_string = r#"
//! today-is = Today is {$date}
//! today-is-fulldate = Today is {DATETIME($date, dateStyle: "full")}
//! now-is-time = Now is {DATETIME($date, timeStyle: "medium")}
//! now-is-datetime = Now is {DATETIME($date, dateStyle: "full", timeStyle: "short")}
//! "#
//! .to_string();
//!
//! let res = FluentResource::try_new(ftl_string)
//!     .expect("Failed to parse an FTL string.");
//! bundle
//!     .add_resource(res)
//!     .expect("Failed to add FTL resources to the bundle.");
//!
//! // Create an ICU DateTime
//! let datetime = DateTime::try_new_iso_datetime(1989, 11, 9, 23, 30, 0)
//!     .expect("Failed to create ICU DateTime");
//!
//! // Convert to FluentDateTime
//! let mut datetime = FluentDateTime::from(datetime);
//!
//! // Format some messages with date arguments
//! let mut errors = vec![];
//!
//! assert_eq!(
//!     bundle.format_pattern(
//!         &bundle.get_message("today-is").unwrap().value().unwrap(),
//!         Some(&fluent_args!("date" => datetime.clone())), &mut errors),
//!     "Today is \u{2068}11/9/89\u{2069}"
//! );
//!
//! assert_eq!(
//!     bundle.format_pattern(
//!         &bundle.get_message("today-is-fulldate").unwrap().value().unwrap(),
//!         Some(&fluent_args!("date" => datetime.clone())), &mut errors),
//!     "Today is \u{2068}Thursday, November 9, 1989\u{2069}"
//! );
//!
//! assert_eq!(
//!     bundle.format_pattern(
//!         &bundle.get_message("now-is-time").unwrap().value().unwrap(),
//!         Some(&fluent_args!("date" => datetime.clone())), &mut errors),
//!     "Now is \u{2068}11:30:00\u{202f}PM\u{2069}"
//! );
//!
//! assert_eq!(
//!     bundle.format_pattern(
//!         &bundle.get_message("now-is-datetime").unwrap().value().unwrap(),
//!         Some(&fluent_args!("date" => datetime.clone())), &mut errors),
//!     "Now is \u{2068}Thursday, November 9, 1989, 11:30\u{202f}PM\u{2069}"
//! );
//!
//! // Set FluentDateTime.options in code rather than in translation data
//! // This is useful because it sets presentation options that are
//! // shared between all locales
//! datetime.options.set_date_style(Some(length::Date::Full));
//! assert_eq!(
//!     bundle.format_pattern(
//!         &bundle.get_message("today-is").unwrap().value().unwrap(),
//!         Some(&fluent_args!("date" => datetime)), &mut errors),
//!     "Today is \u{2068}Thursday, November 9, 1989\u{2069}"
//! );
//!
//! assert!(errors.is_empty());
//!
//! # // I would like to use the ? operator, but Fluent and ICU error types don't implement the std Error trait…
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![forbid(unsafe_code)]
#![warn(missing_docs)]
use std::borrow::Cow;
use std::mem::discriminant;

use fluent_bundle::bundle::FluentBundle;
use fluent_bundle::types::FluentType;
use fluent_bundle::{FluentArgs, FluentError, FluentValue};

use icu_calendar::{Gregorian, Iso};
use icu_datetime::options::length;

fn val_as_str<'a>(val: &'a FluentValue) -> Option<&'a str> {
    if let FluentValue::String(str) = val {
        Some(str)
    } else {
        None
    }
}

/// Options for formatting a DateTime
#[derive(Debug, Clone, PartialEq)]
pub struct FluentDateTimeOptions {
    // This calendar arg makes loading provider data and memoizing formatters harder
    // In particular, the AnyCalendarKind logic (in
    // AnyCalendarKind::from_data_locale_with_fallback) that defaults to
    // Gregorian for most calendars, except for the thai locale (Buddhist),
    // isn't exposed.  So we would have to build the formatter and then decide
    // if it is the correct one for the calendar we want.
    //calendar: Option<icu_calendar::AnyCalendarKind>,
    // We don't handle icu_datetime per-component settings atm, it is experimental
    // and length is expressive enough so far
    length: length::Bag,
}

impl Default for FluentDateTimeOptions {
    /// Defaults to showing a short date
    ///
    /// The intent is to emulate [Intl.DateTimeFormat] behavior:
    /// > The default value for each date-time component option is undefined,
    /// > but if all component properties are undefined, then year, month, and day default
    /// > to "numeric". If any of the date-time component options is specified, then
    /// > dateStyle and timeStyle must be undefined.
    ///
    /// In terms of the current Rust implementation:
    ///
    /// The default value for each date-time style option is None, but if both
    /// are unset, we display the date only, using the `length::Date::Short`
    /// style.
    ///
    /// [Intl.DateTimeFormat]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat/DateTimeFormat
    fn default() -> Self {
        Self {
            length: length::Bag::empty(),
        }
    }
}

impl FluentDateTimeOptions {
    /// Set a date style, from verbose to compact
    ///
    /// See [`icu_datetime::options::length::Date`].
    pub fn set_date_style(&mut self, style: Option<length::Date>) {
        self.length.date = style;
    }

    /// Set a time style, from verbose to compact
    ///
    /// See [`icu_datetime::options::length::Time`].
    pub fn set_time_style(&mut self, style: Option<length::Time>) {
        self.length.time = style;
    }

    fn make_formatter(
        &self,
        locale: &icu_provider::DataLocale,
    ) -> Result<DateTimeFormatter, icu_datetime::DateTimeError> {
        let mut length = self.length;
        if length == length::Bag::empty() {
            length = length::Bag::from_date_style(length::Date::Short);
        }
        Ok(DateTimeFormatter(icu_datetime::DateTimeFormatter::try_new(
            locale,
            length.into(),
        )?))
    }

    fn merge_args(&mut self, other: &FluentArgs) -> Result<(), ()> {
        // TODO set an err state on self to match fluent-js behaviour
        for (k, v) in other.iter() {
            match k {
                "dateStyle" => {
                    self.length.date = Some(match val_as_str(v).ok_or(())? {
                        "full" => length::Date::Full,
                        "long" => length::Date::Long,
                        "medium" => length::Date::Medium,
                        "short" => length::Date::Short,
                        _ => return Err(()),
                    });
                }
                "timeStyle" => {
                    self.length.time = Some(match val_as_str(v).ok_or(())? {
                        "full" => length::Time::Full,
                        "long" => length::Time::Long,
                        "medium" => length::Time::Medium,
                        "short" => length::Time::Short,
                        _ => return Err(()),
                    });
                }
                _ => (), // Ignore with no warning
            }
        }
        Ok(())
    }
}

impl std::hash::Hash for FluentDateTimeOptions {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // We could also use serde… or send a simple PR to have derive(Hash) upstream
        //self.calendar.hash(state);
        self.length.date.map(|e| discriminant(&e)).hash(state);
        self.length.time.map(|e| discriminant(&e)).hash(state);
    }
}

impl Eq for FluentDateTimeOptions {}

/// An ICU [`DateTime`](icu_calendar::DateTime) with attached formatting options
///
/// Construct from an [`icu_calendar::DateTime`] using From / Into.
///
/// Convert to a [`FluentValue`] with From / Into.
///
/// See [`FluentDateTimeOptions`] and [`FluentDateTimeOptions::default`].
///
///```
/// use icu_calendar::DateTime;
/// use fluent_datetime::FluentDateTime;
///
/// let datetime = DateTime::try_new_iso_datetime(1989, 11, 9, 23, 30, 0)
///     .expect("Failed to create ICU DateTime");
///
/// let datetime = FluentDateTime::from(datetime);
// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FluentDateTime {
    // Iso seemed like a natural default, but [AnyCalendarKind::from_data_locale_with_fallback]
    // loads Gregorian in almost all cases.  Differences have to do with eras:
    // proleptic Gregorian has BCE / CE and no year zero, iso has just the one era and a year zero
    value: icu_calendar::DateTime<Gregorian>,
    /// Options for rendering
    pub options: FluentDateTimeOptions,
}

impl FluentType for FluentDateTime {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        // Basically Clone
        Box::new(self.clone())
    }

    fn as_string(&self, intls: &intl_memoizer::IntlLangMemoizer) -> Cow<'static, str> {
        intls
            .with_try_get::<DateTimeFormatter, _, _>(self.options.clone(), |dtf| {
                dtf.0
                    .format_to_string(&self.value.to_any())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
            .into()
    }

    fn as_string_threadsafe(
        &self,
        intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> Cow<'static, str> {
        // Maybe don't try to cache formatters in this case, the traits don't work out
        let lang = intls
            .with_try_get::<GimmeTheLocale, _, _>((), |gimme| gimme.0.clone())
            .expect("Infallible");
        let Some(langid): Option<icu_locid::LanguageIdentifier> = lang.to_string().parse().ok()
        else {
            return "".into();
        };
        let Ok(dtf) = self.options.make_formatter(&langid.into()) else {
            return "".into();
        };
        dtf.0
            .format_to_string(&self.value.to_any())
            .unwrap_or_default()
            .into()
    }
}

impl From<icu_calendar::DateTime<Gregorian>> for FluentDateTime {
    fn from(value: icu_calendar::DateTime<Gregorian>) -> Self {
        Self {
            value,
            options: Default::default(),
        }
    }
}

impl From<icu_calendar::DateTime<Iso>> for FluentDateTime {
    fn from(value: icu_calendar::DateTime<Iso>) -> Self {
        Self {
            value: value.to_calendar(Gregorian),
            options: Default::default(),
        }
    }
}

impl From<FluentDateTime> for FluentValue<'static> {
    fn from(value: FluentDateTime) -> Self {
        Self::Custom(Box::new(value))
    }
}

struct DateTimeFormatter(icu_datetime::DateTimeFormatter);

impl intl_memoizer::Memoizable for DateTimeFormatter {
    type Args = FluentDateTimeOptions;

    type Error = ();

    fn construct(
        lang: unic_langid::LanguageIdentifier,
        args: Self::Args,
    ) -> Result<Self, Self::Error>
    where
        Self: std::marker::Sized,
    {
        // Convert LanguageIdentifier from unic_langid to icu_locid
        let langid: icu_locid::LanguageIdentifier = lang.to_string().parse().map_err(|_| ())?;
        args.make_formatter(&langid.into()).map_err(|_| ())
    }
}

/// Working around that intl_memoizer API, because IntlLangMemoizer doesn't
/// expose the language it is caching
///
/// This would be a trivial addition but it isn't maintained these days.
struct GimmeTheLocale(unic_langid::LanguageIdentifier);

impl intl_memoizer::Memoizable for GimmeTheLocale {
    type Args = ();
    type Error = std::convert::Infallible;

    fn construct(lang: unic_langid::LanguageIdentifier, _args: ()) -> Result<Self, Self::Error>
    where
        Self: std::marker::Sized,
    {
        Ok(Self(lang))
    }
}

/// A Fluent function for formatted datetimes
///
/// Normally you would register this using
/// [`BundleExt::add_datetime_support`]; you would not use it directly.
///
/// However, some frameworks like [l10n](https://lib.rs/crates/l10n)
/// require functions to be set up like this:
///
/// ```ignore
/// l10n::init!({
///     functions: { "DATETIME": fluent_datetime::DATETIME }
/// });
/// ```
///
/// # Usage
///
/// ```fluent
/// today-is = Today is {$date}
/// today-is-fulldate = Today is {DATETIME($date, dateStyle: "full")}
/// now-is-time = Now is {DATETIME($date, timeStyle: "medium")}
/// now-is-datetime = Now is {DATETIME($date, dateStyle: "full", timeStyle: "short")}
/// ````
///
/// See [`DATETIME` in the Fluent guide][datetime-fluent]
/// and [the `Intl.DateTimeFormat` constructor][Intl.DateTimeFormat]
/// from [ECMA 402] for how to use this inside a Fluent document.
///
/// We currently implement only a subset of the formatting options:
/// * `dateStyle`
/// * `timeStyle`
///
/// Unknown options and extra positional arguments are ignored, unknown values
/// of known options cause the date to be returned as-is.
///
/// [datetime-fluent]: https://projectfluent.org/fluent/guide/functions.html#datetime
/// [Intl.DateTimeFormat]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat/DateTimeFormat
/// [ECMA 402]: https://tc39.es/ecma402/#sec-createdatetimeformat
#[allow(non_snake_case)]
pub fn DATETIME<'a>(positional: &[FluentValue<'a>], named: &FluentArgs) -> FluentValue<'a> {
    match positional.get(0) {
        Some(FluentValue::Custom(cus)) => {
            if let Some(dt) = cus.as_any().downcast_ref::<FluentDateTime>() {
                let mut dt = dt.clone();
                let Ok(()) = dt.options.merge_args(named) else {
                    return FluentValue::Error;
                };
                FluentValue::Custom(Box::new(dt))
            } else {
                FluentValue::Error
            }
        }
        // https://github.com/projectfluent/fluent/wiki/Error-Handling
        // argues for graceful recovery (think lingering trauma from XUL DTD
        // errors)
        _ => FluentValue::Error,
    }
}

/// Extension trait to register DateTime support on [`FluentBundle`]
///
/// [`FluentDateTime`] values are rendered automatically, but you need to call
/// [`BundleExt::add_datetime_support`] at bundle creation time when using
/// the [`DATETIME`] function inside FTL resources.
pub trait BundleExt {
    /// Registers the [`DATETIME`] function
    ///
    /// Call this on a [`FluentBundle`].
    ///
    fn add_datetime_support(&mut self) -> Result<(), FluentError>;
}

impl<R, M> BundleExt for FluentBundle<R, M> {
    fn add_datetime_support(&mut self) -> Result<(), FluentError> {
        self.add_function("DATETIME", DATETIME)?;
        //self.set_formatter(Some(datetime_formatter));
        Ok(())
    }
}
