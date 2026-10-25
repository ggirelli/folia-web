//! Dates: the site's clock and time zone, dated file names, and the date forms content uses.

use std::sync::LazyLock;

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use regex::Regex;

/// Why a date cannot be read.
#[derive(Debug, thiserror::Error)]
pub enum DateError {
    /// The site's time zone is unknown.
    #[error("unknown time zone {0:?}: {1}")]
    TimeZone(String, jiff::Error),
    /// A date does not exist, or is in a form Folia does not read.
    #[error(
        "{0:?} is not a date in a form Folia reads (YYYY-MM-DD, optionally with HH:MM or HH:MM:SS)"
    )]
    Form(String),
}

/// The site's time zone and the moment of the build, against which future-dated content is
/// held back.
#[derive(Clone, Debug)]
pub struct SiteClock {
    zone: TimeZone,
    now: Timestamp,
}

impl SiteClock {
    /// A clock for the time zone named `zone` (such as `Europe/Stockholm`), at `now`.
    ///
    /// # Errors
    ///
    /// Fails when the time zone is unknown.
    pub fn new(zone: &str, now: Timestamp) -> Result<Self, DateError> {
        let tz =
            TimeZone::get(zone).map_err(|error| DateError::TimeZone(zone.to_owned(), error))?;
        Ok(Self { zone: tz, now })
    }

    /// Whether a moment is after the build, comparing whole seconds as Jekyll does.
    #[must_use]
    pub fn is_future(&self, moment: Timestamp) -> bool {
        moment.as_second() > self.now.as_second()
    }

    /// The year of the build, in the site's time zone.
    #[must_use]
    pub fn year(&self) -> i16 {
        self.now.to_zoned(self.zone.clone()).year()
    }

    /// A moment in the site's time zone, as Jekyll's `date_to_xmlschema` writes it
    /// (`2026-01-04T13:36:24+01:00`).
    #[must_use]
    pub fn xmlschema(&self, moment: Timestamp) -> String {
        moment
            .to_zoned(self.zone.clone())
            .strftime("%Y-%m-%dT%H:%M:%S%:z")
            .to_string()
    }

    /// Midnight at the start of a date, in the site's time zone.
    ///
    /// # Errors
    ///
    /// Fails when the date has no midnight in the time zone.
    pub fn midnight(&self, date: Date) -> Result<Timestamp, DateError> {
        date.to_zoned(self.zone.clone())
            .map(|zoned| zoned.timestamp())
            .map_err(|_| DateError::Form(date.to_string()))
    }

    /// A front matter date: `YYYY-MM-DD`, optionally followed by `HH:MM` or `HH:MM:SS`, in the
    /// site's time zone.
    ///
    /// # Errors
    ///
    /// Fails for any other form, or a date that does not exist.
    pub fn parse(&self, text: &str) -> Result<Timestamp, DateError> {
        let form = |format: &str| DateTime::strptime(format, text.trim()).ok();
        let local = form("%Y-%m-%d %H:%M:%S")
            .or_else(|| form("%Y-%m-%d %H:%M"))
            .or_else(|| {
                Date::strptime("%Y-%m-%d", text.trim())
                    .ok()
                    .map(|date| date.at(0, 0, 0, 0))
            })
            .ok_or_else(|| DateError::Form(text.to_owned()))?;
        local
            .to_zoned(self.zone.clone())
            .map(|zoned| zoned.timestamp())
            .map_err(|_| DateError::Form(text.to_owned()))
    }
}

static DATED_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4})-(\d{1,2})-(\d{1,2})-([^/]*)(\.[^./]+)$").unwrap());

/// The date and slug in a dated file name such as `2021-08-08-webmentions.md`, as Jekyll reads
/// posts and dated collection documents. Returns `None` when the name is not dated, and an error
/// when it names a date that does not exist.
///
/// Jekyll also accepts two- and three-digit years; Folia requires four.
///
/// # Errors
///
/// Fails when the name is dated but the date does not exist.
pub fn dated_file_name(file_name: &str) -> Result<Option<(Date, String)>, DateError> {
    let Some(captures) = DATED_NAME.captures(file_name) else {
        return Ok(None);
    };
    let number = |i: usize| {
        captures[i]
            .parse::<i16>()
            .map_err(|_| DateError::Form(file_name.to_owned()))
    };
    let year = number(1)?;
    let month = i8::try_from(number(2)?).map_err(|_| DateError::Form(file_name.to_owned()))?;
    let day = i8::try_from(number(3)?).map_err(|_| DateError::Form(file_name.to_owned()))?;
    let date = Date::new(year, month, day).map_err(|_| DateError::Form(file_name.to_owned()))?;
    let slug = captures[4].trim_end_matches('.').to_owned();
    Ok(Some((date, slug)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> SiteClock {
        SiteClock::new("Europe/Stockholm", "2026-10-25T09:00:00Z".parse().unwrap()).unwrap()
    }

    #[test]
    fn a_dated_file_name_gives_its_date_and_slug() {
        let (date, slug) = dated_file_name("2023-05-13-May-train-trip-pt1.md")
            .unwrap()
            .unwrap();
        assert_eq!(date, Date::new(2023, 5, 13).unwrap());
        assert_eq!(slug, "May-train-trip-pt1");
        assert_eq!(
            dated_file_name("2021-8-8-x.md").unwrap().unwrap().0,
            Date::new(2021, 8, 8).unwrap()
        );
    }

    #[test]
    fn an_undated_name_is_not_an_error() {
        assert!(dated_file_name("chat_control.md").unwrap().is_none());
        assert!(
            dated_file_name("2021-08-08-no-extension")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn an_impossible_date_is_an_error() {
        assert!(dated_file_name("2021-02-30-x.md").is_err());
    }

    #[test]
    fn front_matter_dates_are_local_to_the_site() {
        let clock = clock();
        assert_eq!(
            clock.parse("2026-04-08 18:25").unwrap(),
            "2026-04-08T16:25:00Z".parse().unwrap()
        );
        assert_eq!(
            clock.parse("2026-01-31").unwrap(),
            "2026-01-30T23:00:00Z".parse().unwrap()
        );
        assert!(clock.parse("April 8").is_err());
    }

    #[test]
    fn moments_are_written_in_the_site_time_zone() {
        let clock = clock();
        assert_eq!(
            clock.xmlschema("2026-01-04T12:36:24Z".parse().unwrap()),
            "2026-01-04T13:36:24+01:00"
        );
        assert_eq!(
            clock.xmlschema("2026-07-01T00:00:00Z".parse().unwrap()),
            "2026-07-01T02:00:00+02:00"
        );
    }

    #[test]
    fn the_build_year_is_local_to_the_site() {
        let new_year = SiteClock::new("Europe/Stockholm", "2026-12-31T23:30:00Z".parse().unwrap());
        assert_eq!(new_year.unwrap().year(), 2027);
        assert_eq!(clock().year(), 2026);
    }

    #[test]
    fn only_moments_after_the_build_are_future() {
        let clock = clock();
        assert!(!clock.is_future(clock.midnight(Date::new(2026, 10, 25).unwrap()).unwrap()));
        assert!(clock.is_future(clock.midnight(Date::new(2026, 10, 26).unwrap()).unwrap()));
    }
}
