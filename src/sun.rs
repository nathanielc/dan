//! This module can compute the sunrise and sunset times given a date and lattitide and longititue location
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use spa::{sunrise_and_set, StdFloatOps, SunriseAndSet};

pub struct Location {
    latitude: f64,
    longitude: f64,
}

pub fn sunrise(location: &Location, date: DateTime<Utc>) -> Result<DateTime<Utc>, anyhow::Error> {
    let sunrise_set = sunrise_and_set::<StdFloatOps>(date, location.latitude, location.longitude)
        .context("Failed to calculate sunrise and sunset")?;
    match sunrise_set {
        SunriseAndSet::PolarNight => Err(anyhow!("Polar Night")),
        SunriseAndSet::PolarDay => Err(anyhow!("Polar Day")),
        SunriseAndSet::Daylight(sunrise, _sunset) => Ok(sunrise),
    }
}

pub fn sunset(location: &Location, date: DateTime<Utc>) -> Result<DateTime<Utc>, anyhow::Error> {
    let sunrise_set = sunrise_and_set::<StdFloatOps>(date, location.latitude, location.longitude)
        .context("Failed to calculate sunrise and sunset")?;
    match sunrise_set {
        SunriseAndSet::PolarNight => Err(anyhow!("Polar Night")),
        SunriseAndSet::PolarDay => Err(anyhow!("Polar Day")),
        SunriseAndSet::Daylight(_sunrise, sunset) => Ok(sunset),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};
    use expect_test::expect;

    use super::*;

    #[test]
    fn test_sunrise_time() {
        let location = Location {
            latitude: 40.131_066,
            longitude: -111.567_286,
        };
        let mountain = FixedOffset::west_opt(7 * 60 * 60).unwrap();
        let date = mountain.with_ymd_and_hms(2025, 3, 3, 12, 0, 0).unwrap();
        expect!["2025-03-03 12:00:00 -07:00"].assert_eq(&date.to_string());
        expect!["2025-03-03 06:54:42.155469056 -07:00"].assert_eq(
            &sunrise(&location, date.with_timezone(&Utc))
                .unwrap()
                .with_timezone(&mountain)
                .to_string(),
        );
    }

    #[test]
    fn test_sunset_time() {
        let location = Location {
            latitude: 40.131_066,
            longitude: -111.567_286,
        };
        let mountain = FixedOffset::west_opt(7 * 60 * 60).unwrap();
        let date = mountain.with_ymd_and_hms(2025, 3, 3, 12, 0, 0).unwrap();
        expect!["2025-03-03 12:00:00 -07:00"].assert_eq(&date.to_string());
        expect!["2025-03-03 18:19:24.722616832 -07:00"].assert_eq(
            &sunset(&location, date.with_timezone(&Utc))
                .unwrap()
                .with_timezone(&mountain)
                .to_string(),
        );
    }
}
