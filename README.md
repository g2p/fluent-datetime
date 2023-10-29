# fluent-datetime

## International datetimes in Fluent translations

fluent-datetime uses [ICU4X], in particular [`icu_datetime`] and
[`icu_calendar`], to format datetimes internationally within
a [Fluent] translation.

[Fluent]: https://projectfluent.org/
[ICU4X]: https://github.com/unicode-org/icu4x

## Example

This example uses [`fluent_bundle`] directly.

You may prefer to use less verbose integrations; in which case the
[`bundle.add_datetime_support()`](BundleExt::add_datetime_support)
line is the only one you need.

```rust
use fluent::fluent_args;
use fluent_bundle::{FluentBundle, FluentResource};
use fluent_datetime::{BundleExt, FluentDateTime};
use icu_calendar::DateTime;
use icu_datetime::options::length;
use unic_langid::LanguageIdentifier;

// Create a FluentBundle
let langid_en: LanguageIdentifier = "en-US".parse()?;
let mut bundle = FluentBundle::new(vec![langid_en]);

// Register the DATETIME function
bundle.add_datetime_support();

// Add a FluentResource to the bundle
let ftl_string = r#"
today-is = Today is {$date}
today-is-fulldate = Today is {DATETIME($date, dateStyle: "full")}
now-is-time = Now is {DATETIME($date, timeStyle: "medium")}
now-is-datetime = Now is {DATETIME($date, dateStyle: "full", timeStyle: "short")}
"#
.to_string();

let res = FluentResource::try_new(ftl_string)
    .expect("Failed to parse an FTL string.");
bundle
    .add_resource(res)
    .expect("Failed to add FTL resources to the bundle.");

// Create an ICU DateTime
let datetime = DateTime::try_new_iso_datetime(1989, 11, 9, 23, 30, 0)
    .expect("Failed to create ICU DateTime");

// Convert to FluentDateTime
let mut datetime = FluentDateTime::from(datetime);

// Format some messages with date arguments
let mut errors = vec![];

assert_eq!(
    bundle.format_pattern(
        &bundle.get_message("today-is").unwrap().value().unwrap(),
        Some(&fluent_args!("date" => datetime.clone())), &mut errors),
    "Today is \u{2068}11/9/89\u{2069}"
);

assert_eq!(
    bundle.format_pattern(
        &bundle.get_message("today-is-fulldate").unwrap().value().unwrap(),
        Some(&fluent_args!("date" => datetime.clone())), &mut errors),
    "Today is \u{2068}Thursday, November 9, 1989\u{2069}"
);

assert_eq!(
    bundle.format_pattern(
        &bundle.get_message("now-is-time").unwrap().value().unwrap(),
        Some(&fluent_args!("date" => datetime.clone())), &mut errors),
    "Now is \u{2068}11:30:00\u{202f}PM\u{2069}"
);

assert_eq!(
    bundle.format_pattern(
        &bundle.get_message("now-is-datetime").unwrap().value().unwrap(),
        Some(&fluent_args!("date" => datetime.clone())), &mut errors),
    "Now is \u{2068}Thursday, November 9, 1989, 11:30\u{202f}PM\u{2069}"
);

// Set FluentDateTime.options in code rather than in translation data
// This is useful because it sets presentation options that are
// shared between all locales
datetime.options.set_date_style(Some(length::Date::Full));
assert_eq!(
    bundle.format_pattern(
        &bundle.get_message("today-is").unwrap().value().unwrap(),
        Some(&fluent_args!("date" => datetime)), &mut errors),
    "Today is \u{2068}Thursday, November 9, 1989\u{2069}"
);

assert!(errors.is_empty());

```
