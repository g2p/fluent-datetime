use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use icu_calendar::{AnyCalendar, DateTime, Gregorian};
use icu_datetime::options::length;
use icu_datetime::{DateTimeFormatter, TypedDateTimeFormatter};
use icu_provider::DataLocale;

fn make_formatter(locale: &DataLocale) -> DateTimeFormatter {
    // Using Time::Short otherwise try_new returns an Err(UnsupportedField(TimeZone(LowerZ)))
    let length = length::Bag::from_date_time_style(length::Date::Full, length::Time::Short);
    icu_datetime::DateTimeFormatter::try_new(locale, length.into()).unwrap()
}

fn format(formatter: &DateTimeFormatter, dt: &DateTime<AnyCalendar>) -> String {
    formatter.format_to_string(dt).unwrap()
}

fn make_typed_formatter(locale: &DataLocale) -> TypedDateTimeFormatter<Gregorian> {
    let length = length::Bag::from_date_time_style(length::Date::Full, length::Time::Short);
    // Using Gregorian, not Iso, otherwise I get:
    // the trait `CldrCalendar` is not implemented for `icu_calendar::Iso`
    icu_datetime::TypedDateTimeFormatter::try_new(locale, length.into()).unwrap()
}

fn format_typed(formatter: &TypedDateTimeFormatter<Gregorian>, dt: &DateTime<Gregorian>) -> String {
    formatter.format_to_string(dt)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let locale = icu_locid::langid!("fr-FR");
    let locale = &locale.into();
    let datetime = DateTime::try_new_gregorian_datetime(1989, 11, 9, 23, 30, 0).unwrap();
    let dt_any = datetime.clone().to_any();

    c.bench_function("instanciate every time", |b| {
        b.iter(|| format(&make_formatter(black_box(locale)), black_box(&dt_any)))
    });
    let formatter = make_formatter(locale);
    c.bench_function("instanciate once", |b| {
        b.iter(|| format(black_box(&formatter), black_box(&dt_any)))
    });

    c.bench_function("instanciate every time (typed)", |b| {
        b.iter(|| {
            format_typed(
                &make_typed_formatter(black_box(locale)),
                black_box(&datetime),
            )
        })
    });
    let formatter = make_typed_formatter(locale);
    c.bench_function("instanciate once (typed)", |b| {
        b.iter(|| format_typed(black_box(&formatter), black_box(&datetime)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
