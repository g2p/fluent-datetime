use std::borrow::Cow;
use std::mem::discriminant;

use fluent_bundle::bundle::FluentBundle;
use fluent_bundle::types::FluentType;
use fluent_bundle::{FluentArgs, FluentError, FluentValue};

use icu_calendar::Gregorian;
use icu_datetime::options::length;

fn val_as_str<'a>(val: &'a FluentValue) -> Option<&'a str> {
    if let FluentValue::String(str) = val {
        Some(str)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FluentDateTimeOptions {
    // This calendar arg makes loading provider data and memoizing formatters harder
    // In particular, the AnyCalendarKind logic (in
    // AnyCalendarKind::from_data_locale_with_fallback) that defaults to
    // Gregorian for most calendars, except for the thai locale (Buddhist),
    // isn't exposed.  So we would have to build the formatter and then decide
    // if it is the correct one for the calendar we want.
    //calendar: Option<icu_calendar::AnyCalendarKind>,
    // We don't handle icu_datetime per-component settings atm, it is experimental
    // and length is expressive enough
    pub length: length::Bag,
}

impl Default for FluentDateTimeOptions {
    /// Defaults to showing a short date
    ///
    /// The intent is to emulate the Intl.DateTimeFormat default:
    /// The default value for each date-time component option is undefined, but
    /// if all component properties are undefined, then year, month, and day default
    /// to "numeric". If any of the date-time component options is specified, then
    /// dateStyle and timeStyle must be undefined.
    ///
    /// From <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat/DateTimeFormat>
    fn default() -> Self {
        Self {
            length: length::Bag::from_date_style(length::Date::Short),
        }
    }
}

impl FluentDateTimeOptions {
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
                    })
                }
                "timeStyle" => {
                    self.length.time = Some(match val_as_str(v).ok_or(())? {
                        "full" => length::Time::Full,
                        "long" => length::Time::Long,
                        "medium" => length::Time::Medium,
                        "short" => length::Time::Short,
                        _ => return Err(()),
                    })
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

#[derive(Debug, Clone, PartialEq)]
pub struct FluentDateTime {
    value: icu_calendar::DateTime<Gregorian>,
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
                    .unwrap_or("".to_string())
                    .into()
            })
            .unwrap_or("".into())
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
        let locale = icu_provider::DataLocale::from(langid);
        let Ok(dtf) = icu_datetime::DateTimeFormatter::try_new(&locale, self.options.length.into())
        else {
            return "".into();
        };
        dtf.format_to_string(&self.value.to_any())
            .unwrap_or("".to_string())
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
        let locale = icu_provider::DataLocale::from(langid);
        let inner = icu_datetime::DateTimeFormatter::try_new(&locale, args.length.into())
            .map_err(|_| ())?;
        Ok(Self(inner))
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

/// A FluentFunction for formatted datetimes
///
/// Documented on [BundleExt::add_datetime_support] as the function isn't public
pub(crate) fn datetime_func<'a>(
    positional: &[FluentValue<'a>],
    named: &FluentArgs,
) -> FluentValue<'a> {
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
        _ => FluentValue::Error,
    }
}

pub trait BundleExt {
    /// Registers the `DATETIME` function
    ///
    /// Call this on a [`FluentBundle`]
    ///
    /// See [`DATETIME` in the Fluent guide](https://projectfluent.org/fluent/guide/functions.html#datetime)
    /// and [the `Intl.DateTimeFormat` constructor](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat/DateTimeFormat) from [ECMA 402](https://tc39.es/ecma402/#sec-createdatetimeformat).
    ///
    /// We currently implement only a subset of the formatting options:
    /// * dateStyle
    /// * timeStyle
    /// Unknown options and extra positional arguments are ignored, unknown values of
    /// known options cause the date to be returned as-is.
    fn add_datetime_support(&mut self) -> Result<(), FluentError>;
}

impl<R, M> BundleExt for FluentBundle<R, M> {
    fn add_datetime_support(&mut self) -> Result<(), FluentError> {
        self.add_function("DATETIME", datetime_func)?;
        //self.set_formatter(Some(datetime_formatter));
        Ok(())
    }
}
