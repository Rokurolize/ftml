use super::{RelativeTimeUnit, localization_error, relative_time_fmt_options};
use icu_calendar::Gregorian;
use icu_datetime::DateTimeFormatter;
use icu_datetime::DateTimeFormatterPreferences;
use icu_datetime::fieldsets::enums::TimeFieldSet;
use icu_datetime::fieldsets::{E, M, T, YMD, YMDT};
use icu_datetime::options::{TimePrecision, YearStyle};
use icu_datetime::pattern::{DayPeriodNameLength, FixedCalendarDateTimeNames};
use icu_datetime::preferences::HourCycle;
use icu_experimental::relativetime::options::Numeric;
use icu_experimental::relativetime::{
    RelativeTimeFormatter, RelativeTimeFormatterPreferences,
};
use icu_locale::Locale;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::rc::Rc;

const MAX_ICU_CACHE_ENTRIES: usize = 64;

type Cached<T> = Rc<Result<T, String>>;

thread_local! {
    static LOCALE_CACHE: RefCell<HashMap<String, Locale>> =
        RefCell::new(HashMap::new());
    static WEEKDAY_FORMATTERS: RefCell<HashMap<LengthKey, Cached<DateTimeFormatter<E>>>> =
        RefCell::new(HashMap::new());
    static MONTH_FORMATTERS: RefCell<HashMap<LengthKey, Cached<DateTimeFormatter<M>>>> =
        RefCell::new(HashMap::new());
    static DATE_FORMATTERS: RefCell<HashMap<LocaleKey, Cached<DateTimeFormatter<YMD>>>> =
        RefCell::new(HashMap::new());
    static DATETIME_FORMATTERS: RefCell<HashMap<LocaleKey, Cached<DateTimeFormatter<YMDT>>>> =
        RefCell::new(HashMap::new());
    static TIME_FORMATTERS: RefCell<HashMap<TimeKey, Cached<DateTimeFormatter<T>>>> =
        RefCell::new(HashMap::new());
    static DAY_PERIOD_NAMES: RefCell<HashMap<LocaleKey, Cached<FixedCalendarDateTimeNames<Gregorian, TimeFieldSet>>>> =
        RefCell::new(HashMap::new());
    static RELATIVE_TIME_FORMATTERS: RefCell<HashMap<RelativeTimeKey, Cached<RelativeTimeFormatter>>> =
        RefCell::new(HashMap::new());
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct LocaleKey {
    locale: String,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct LengthKey {
    locale: String,
    length: FormatLength,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct TimeKey {
    locale: String,
    hour_cycle: TimeHourCycle,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct RelativeTimeKey {
    locale: String,
    unit: RelativeTimeUnit,
    numeric: RelativeTimeNumeric,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub(super) enum FormatLength {
    Short,
    Medium,
    Long,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub(super) enum TimeHourCycle {
    LocaleDefault,
    H12,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub(super) enum RelativeTimeNumeric {
    Always,
    Auto,
}

pub(super) fn locale_from_language(language: &str) -> Locale {
    LOCALE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(locale) = cache.get(language) {
            return locale.clone();
        }

        let locale = match Locale::try_from_str(language) {
            Ok(locale) => locale,
            Err(error) => {
                debug!(
                    "Invalid date render locale '{language}', falling back to English: {error}"
                );
                return Locale::try_from_str("en")
                    .expect("English locale should always parse");
            }
        };

        insert_bounded(&mut cache, language.to_owned(), locale.clone());
        locale
    })
}

pub(super) fn with_weekday_formatter<R>(
    locale: &Locale,
    length: FormatLength,
    format: impl FnOnce(&DateTimeFormatter<E>) -> io::Result<R>,
) -> io::Result<R> {
    let key = LengthKey::new(locale, length);
    WEEKDAY_FORMATTERS.with(|cache| {
        with_cached(
            cache,
            key,
            || {
                let prefs = formatter_preferences(locale);
                match length {
                    FormatLength::Short => DateTimeFormatter::try_new(prefs, E::short()),
                    FormatLength::Long => DateTimeFormatter::try_new(prefs, E::long()),
                    FormatLength::Medium => {
                        unreachable!("weekday format has no medium length")
                    }
                }
            },
            format,
        )
    })
}

pub(super) fn with_month_formatter<R>(
    locale: &Locale,
    length: FormatLength,
    format: impl FnOnce(&DateTimeFormatter<M>) -> io::Result<R>,
) -> io::Result<R> {
    let key = LengthKey::new(locale, length);
    MONTH_FORMATTERS.with(|cache| {
        with_cached(
            cache,
            key,
            || {
                let prefs = formatter_preferences(locale);
                match length {
                    FormatLength::Medium => {
                        DateTimeFormatter::try_new(prefs, M::medium())
                    }
                    FormatLength::Long => DateTimeFormatter::try_new(prefs, M::long()),
                    FormatLength::Short => {
                        unreachable!("month format has no short length")
                    }
                }
            },
            format,
        )
    })
}

pub(super) fn with_date_formatter<R>(
    locale: &Locale,
    format: impl FnOnce(&DateTimeFormatter<YMD>) -> io::Result<R>,
) -> io::Result<R> {
    let key = LocaleKey::new(locale);
    DATE_FORMATTERS.with(|cache| {
        with_cached(
            cache,
            key,
            || DateTimeFormatter::try_new(formatter_preferences(locale), YMD::short()),
            format,
        )
    })
}

pub(super) fn with_datetime_formatter<R>(
    locale: &Locale,
    format: impl FnOnce(&DateTimeFormatter<YMDT>) -> io::Result<R>,
) -> io::Result<R> {
    let key = LocaleKey::new(locale);
    DATETIME_FORMATTERS.with(|cache| {
        with_cached(
            cache,
            key,
            || {
                DateTimeFormatter::try_new(
                    formatter_preferences(locale),
                    YMDT::short()
                        .with_year_style(YearStyle::Full)
                        .with_time_precision(TimePrecision::Second),
                )
            },
            format,
        )
    })
}

pub(super) fn with_time_formatter<R>(
    locale: &Locale,
    hour_cycle: TimeHourCycle,
    format: impl FnOnce(&DateTimeFormatter<T>) -> io::Result<R>,
) -> io::Result<R> {
    let key = TimeKey::new(locale, hour_cycle);
    TIME_FORMATTERS.with(|cache| {
        with_cached(
            cache,
            key,
            || {
                let mut prefs = formatter_preferences(locale);
                if hour_cycle == TimeHourCycle::H12 {
                    prefs.hour_cycle = Some(HourCycle::H12);
                }
                DateTimeFormatter::try_new(prefs, T::medium())
            },
            format,
        )
    })
}

pub(super) fn with_day_period_names<R>(
    locale: &Locale,
    format: impl FnOnce(&FixedCalendarDateTimeNames<Gregorian, TimeFieldSet>) -> io::Result<R>,
) -> io::Result<R> {
    let key = LocaleKey::new(locale);
    DAY_PERIOD_NAMES.with(|cache| {
        with_cached(
            cache,
            key,
            || {
                let mut names: FixedCalendarDateTimeNames<Gregorian, TimeFieldSet> =
                    FixedCalendarDateTimeNames::try_new(formatter_preferences(locale))
                        .map_err(|error| error.to_string())?;
                names
                    .include_day_period_names(DayPeriodNameLength::Abbreviated)
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>(names)
            },
            format,
        )
    })
}

pub(super) fn with_relative_time_formatter<R>(
    locale: &Locale,
    unit: RelativeTimeUnit,
    numeric: RelativeTimeNumeric,
    format: impl FnOnce(&RelativeTimeFormatter) -> io::Result<R>,
) -> io::Result<R> {
    let key = RelativeTimeKey::new(locale, unit, numeric);
    RELATIVE_TIME_FORMATTERS.with(|cache| {
        with_cached(
            cache,
            key,
            || {
                let prefs: RelativeTimeFormatterPreferences = locale.clone().into();
                match unit {
                    RelativeTimeUnit::Second => {
                        RelativeTimeFormatter::try_new_long_second(
                            prefs,
                            relative_time_fmt_options(numeric),
                        )
                    }
                    RelativeTimeUnit::Minute => {
                        RelativeTimeFormatter::try_new_long_minute(
                            prefs,
                            relative_time_fmt_options(numeric),
                        )
                    }
                    RelativeTimeUnit::Hour => RelativeTimeFormatter::try_new_long_hour(
                        prefs,
                        relative_time_fmt_options(numeric),
                    ),
                    RelativeTimeUnit::Day => RelativeTimeFormatter::try_new_long_day(
                        prefs,
                        relative_time_fmt_options(numeric),
                    ),
                }
            },
            format,
        )
    })
}

fn with_cached<K, T, E, R>(
    cache: &RefCell<HashMap<K, Cached<T>>>,
    key: K,
    construct: impl FnOnce() -> Result<T, E>,
    format: impl FnOnce(&T) -> io::Result<R>,
) -> io::Result<R>
where
    K: Clone + Eq + Hash,
    E: ToString,
{
    let cached = {
        let mut cache = cache.borrow_mut();
        if !cache.contains_key(&key) {
            if cache.len() >= MAX_ICU_CACHE_ENTRIES {
                cache.clear();
            }
            cache.insert(
                key.clone(),
                Rc::new(construct().map_err(|error| error.to_string())),
            );
        }
        Rc::clone(cache.get(&key).expect("cached formatter was just inserted"))
    };

    match cached.as_ref() {
        Ok(formatter) => format(formatter),
        Err(error) => Err(localization_error(error.clone())),
    }
}

fn insert_bounded<K, V>(cache: &mut HashMap<K, V>, key: K, value: V)
where
    K: Eq + Hash,
{
    if cache.len() >= MAX_ICU_CACHE_ENTRIES {
        cache.clear();
    }
    cache.insert(key, value);
}

impl LocaleKey {
    fn new(locale: &Locale) -> Self {
        Self {
            locale: locale.to_string(),
        }
    }
}

impl LengthKey {
    fn new(locale: &Locale, length: FormatLength) -> Self {
        Self {
            locale: locale.to_string(),
            length,
        }
    }
}

impl TimeKey {
    fn new(locale: &Locale, hour_cycle: TimeHourCycle) -> Self {
        Self {
            locale: locale.to_string(),
            hour_cycle,
        }
    }
}

impl RelativeTimeKey {
    fn new(
        locale: &Locale,
        unit: RelativeTimeUnit,
        numeric: RelativeTimeNumeric,
    ) -> Self {
        Self {
            locale: locale.to_string(),
            unit,
            numeric,
        }
    }
}

impl RelativeTimeNumeric {
    pub(super) fn as_icu_numeric(self) -> Numeric {
        match self {
            Self::Always => Numeric::Always,
            Self::Auto => Numeric::Auto,
        }
    }
}

fn formatter_preferences(locale: &Locale) -> DateTimeFormatterPreferences {
    locale.clone().into()
}
