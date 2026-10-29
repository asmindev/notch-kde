use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

pub const PRAYER_COUNT: usize = 5;
pub const MAX_REMINDER_MINUTES: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prayer {
    Fajr,
    Dhuhr,
    Asr,
    Maghrib,
    Isha,
}

impl Prayer {
    pub const ALL: [Self; PRAYER_COUNT] = [
        Self::Fajr,
        Self::Dhuhr,
        Self::Asr,
        Self::Maghrib,
        Self::Isha,
    ];

    pub fn name_id(self) -> &'static str {
        match self {
            Self::Fajr => "Subuh",
            Self::Dhuhr => "Dzuhur",
            Self::Asr => "Ashar",
            Self::Maghrib => "Maghrib",
            Self::Isha => "Isya",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrayerAlertKind {
    Reminder,
    Adhan,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct City {
    pub name: &'static str,
    pub region: &'static str,
    pub latitude: f64,
    pub longitude: f64,
    pub utc_offset_minutes: i32,
}

impl City {
    pub fn offset(self) -> FixedOffset {
        FixedOffset::east_opt(self.utc_offset_minutes * 60)
            .expect("city UTC offset is within the supported range")
    }
}

pub const CITIES: [City; 14] = [
    City {
        name: "Jakarta",
        region: "WIB",
        latitude: -6.2088,
        longitude: 106.8456,
        utc_offset_minutes: 420,
    },
    City {
        name: "Bandung",
        region: "WIB",
        latitude: -6.9175,
        longitude: 107.6191,
        utc_offset_minutes: 420,
    },
    City {
        name: "Surabaya",
        region: "WIB",
        latitude: -7.2575,
        longitude: 112.7521,
        utc_offset_minutes: 420,
    },
    City {
        name: "Yogyakarta",
        region: "WIB",
        latitude: -7.7956,
        longitude: 110.3695,
        utc_offset_minutes: 420,
    },
    City {
        name: "Semarang",
        region: "WIB",
        latitude: -6.9667,
        longitude: 110.4167,
        utc_offset_minutes: 420,
    },
    City {
        name: "Medan",
        region: "WIB",
        latitude: 3.5952,
        longitude: 98.6722,
        utc_offset_minutes: 420,
    },
    City {
        name: "Padang",
        region: "WIB",
        latitude: -0.9471,
        longitude: 100.4172,
        utc_offset_minutes: 420,
    },
    City {
        name: "Palembang",
        region: "WIB",
        latitude: -2.9761,
        longitude: 104.7754,
        utc_offset_minutes: 420,
    },
    City {
        name: "Makassar",
        region: "WITA",
        latitude: -5.1477,
        longitude: 119.4327,
        utc_offset_minutes: 480,
    },
    City {
        name: "Denpasar",
        region: "WITA",
        latitude: -8.6705,
        longitude: 115.2126,
        utc_offset_minutes: 480,
    },
    City {
        name: "Balikpapan",
        region: "WITA",
        latitude: -1.2379,
        longitude: 116.8529,
        utc_offset_minutes: 480,
    },
    City {
        name: "Kendari",
        region: "WITA",
        latitude: -3.9450,
        longitude: 122.4989,
        utc_offset_minutes: 480,
    },
    City {
        name: "Baubau",
        region: "WITA",
        latitude: -5.4667,
        longitude: 122.6333,
        utc_offset_minutes: 480,
    },
    City {
        name: "Jayapura",
        region: "WIT",
        latitude: -2.5916,
        longitude: 140.6690,
        utc_offset_minutes: 540,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrayerSettings {
    pub enabled: bool,
    pub city: u32,
    pub reminder_minutes: u32,
}

impl Default for PrayerSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            city: 0,
            reminder_minutes: 10,
        }
    }
}

impl PrayerSettings {
    pub fn normalized(self) -> Self {
        Self {
            enabled: self.enabled,
            city: self.city % CITIES.len() as u32,
            reminder_minutes: self.reminder_minutes.min(MAX_REMINDER_MINUTES),
        }
    }

    pub fn city(self) -> &'static City {
        &CITIES[(self.city as usize) % CITIES.len()]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrayerAlert {
    pub prayer: Prayer,
    pub kind: PrayerAlertKind,
    pub at: DateTime<Utc>,
    pub title: String,
    pub time_label: String,
    pub city_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrayerTimes {
    pub date: NaiveDate,
    pub times: [DateTime<FixedOffset>; PRAYER_COUNT],
    pub sunrise: DateTime<FixedOffset>,
    pub sunset: DateTime<FixedOffset>,
    pub solar_noon: DateTime<FixedOffset>,
    sun_latitude_rad: f64,
    sun_declination_rad: f64,
    sun_max_altitude_deg: f64,
}

impl PrayerTimes {
    pub fn iter(&self) -> impl Iterator<Item = (Prayer, DateTime<FixedOffset>)> + '_ {
        Prayer::ALL.into_iter().zip(self.times.iter().copied())
    }

    /// Signed solar altitude in degrees. Positive above the horizon, negative below.
    /// This is the raw geometric altitude (not normalized), so it is suitable for
    /// driving an honest sky: civil/nautical/astronomical twilight all read as
    /// distinct negative values instead of collapsing to 0.
    pub fn sun_altitude_deg(&self, at: DateTime<FixedOffset>) -> f64 {
        let seconds_from_noon = (at - self.solar_noon).num_seconds() as f64;
        let hour_angle = (seconds_from_noon / 240.0).to_radians();
        let sin_altitude = self.sun_latitude_rad.sin() * self.sun_declination_rad.sin()
            + self.sun_latitude_rad.cos() * self.sun_declination_rad.cos() * hour_angle.cos();
        sin_altitude.clamp(-1.0, 1.0).asin().to_degrees()
    }

    pub fn sun_height(&self, at: DateTime<FixedOffset>) -> f32 {
        let altitude = self.sun_altitude_deg(at);
        if altitude <= 0.0 || self.sun_max_altitude_deg <= 0.1 {
            return 0.0;
        }
        (altitude / self.sun_max_altitude_deg).clamp(0.0, 1.0) as f32
    }
}

/// A rendered sky derived from true solar altitude. Colors are sRGB in `0.0..=1.0`.
/// Kept as plain data so Slint stays dumb and the "honest sky" logic is testable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyState {
    /// Zenith (top of the dome) color.
    pub top: [f32; 3],
    /// Horizon (near the diameter line) color.
    pub horizon: [f32; 3],
    /// Ambient glow around the sun/moon.
    pub glow: [f32; 3],
    /// Star field opacity, `0.0` (full day) ..= `1.0` (deep night).
    pub star_opacity: f32,
}

const SKY_KEYFRAMES: [(f64, [f32; 3], [f32; 3], [f32; 3], f32); 8] = [
    // (altitude°, top, horizon, glow, star_opacity)
    (
        -18.0,
        [0.02, 0.03, 0.08],
        [0.03, 0.05, 0.12],
        [0.06, 0.08, 0.14],
        0.95,
    ),
    (
        -12.0,
        [0.03, 0.04, 0.11],
        [0.05, 0.07, 0.16],
        [0.10, 0.10, 0.20],
        0.70,
    ),
    (
        -6.0,
        [0.06, 0.06, 0.16],
        [0.14, 0.10, 0.20],
        [0.55, 0.30, 0.35],
        0.35,
    ),
    (
        0.0,
        [0.10, 0.10, 0.24],
        [0.55, 0.28, 0.22],
        [0.95, 0.50, 0.25],
        0.12,
    ),
    (
        4.0,
        [0.12, 0.16, 0.32],
        [0.70, 0.45, 0.30],
        [0.96, 0.62, 0.28],
        0.04,
    ),
    (
        12.0,
        [0.10, 0.20, 0.40],
        [0.35, 0.45, 0.55],
        [0.94, 0.71, 0.16],
        0.00,
    ),
    (
        35.0,
        [0.05, 0.12, 0.28],
        [0.18, 0.35, 0.52],
        [0.94, 0.71, 0.16],
        0.00,
    ),
    (
        60.0,
        [0.04, 0.10, 0.26],
        [0.15, 0.32, 0.50],
        [0.96, 0.77, 0.32],
        0.00,
    ),
];

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

/// Map a signed solar altitude to a full sky palette by interpolating the
/// keyframe table. Clamped at both ends (deep night / high noon).
pub fn sky_from_altitude(altitude_deg: f64) -> SkyState {
    let first = SKY_KEYFRAMES[0];
    if altitude_deg <= first.0 {
        return SkyState {
            top: first.1,
            horizon: first.2,
            glow: first.3,
            star_opacity: first.4,
        };
    }
    let last = *SKY_KEYFRAMES.last().expect("sky keyframes non-empty");
    if altitude_deg >= last.0 {
        return SkyState {
            top: last.1,
            horizon: last.2,
            glow: last.3,
            star_opacity: last.4,
        };
    }

    for pair in SKY_KEYFRAMES.windows(2) {
        let (lo, hi) = (pair[0], pair[1]);
        if altitude_deg >= lo.0 && altitude_deg <= hi.0 {
            let span = (hi.0 - lo.0) as f32;
            let t = if span <= f32::EPSILON {
                0.0
            } else {
                ((altitude_deg - lo.0) as f32 / span).clamp(0.0, 1.0)
            };
            return SkyState {
                top: lerp3(lo.1, hi.1, t),
                horizon: lerp3(lo.2, hi.2, t),
                glow: lerp3(lo.3, hi.3, t),
                star_opacity: lerp(lo.4, hi.4, t),
            };
        }
    }

    // Unreachable: the clamp branches above cover everything outside the table.
    SkyState {
        top: last.1,
        horizon: last.2,
        glow: last.3,
        star_opacity: last.4,
    }
}

/// Convenience: full sky for a city at an instant, from true solar altitude.
pub fn sky_for(city: &City, now: DateTime<FixedOffset>) -> Option<SkyState> {
    let times = times_for(city, now.date_naive())?;
    Some(sky_from_altitude(times.sun_altitude_deg(now)))
}

pub fn times_for(city: &City, date: NaiveDate) -> Option<PrayerTimes> {
    let day_of_year = date.ordinal() as f64;
    let gamma = TAU / 365.0 * (day_of_year - 1.0 + 0.5);
    let equation_of_time = 229.18
        * (0.000075 + 0.001868 * gamma.cos()
            - 0.032077 * gamma.sin()
            - 0.014615 * (2.0 * gamma).cos()
            - 0.040849 * (2.0 * gamma).sin());
    let declination = 0.006918 - 0.399912 * gamma.cos() + 0.070257 * gamma.sin()
        - 0.006758 * (2.0 * gamma).cos()
        + 0.000907 * (2.0 * gamma).sin()
        - 0.002697 * (3.0 * gamma).cos()
        + 0.001480 * (3.0 * gamma).sin();

    let latitude = city.latitude.to_radians();
    let solar_noon_utc_minutes = 720.0 - 4.0 * city.longitude - equation_of_time;
    let utc_midnight = date.and_hms_opt(0, 0, 0)?.and_utc();
    let offset = city.offset();

    let hour_angle = |altitude_deg: f64| -> Option<f64> {
        let altitude = altitude_deg.to_radians();
        let cosine = (altitude.sin() - latitude.sin() * declination.sin())
            / (latitude.cos() * declination.cos());
        if !(-1.0..=1.0).contains(&cosine) {
            return None;
        }
        Some(cosine.acos().to_degrees())
    };

    let asr_altitude = (1.0 / (1.0 + (latitude - declination).abs().tan()))
        .atan()
        .to_degrees();
    let ihtiyati_minutes = 2.0;

    let sunrise = solar_noon_utc_minutes - 4.0 * hour_angle(-0.833)?;
    let sunset = solar_noon_utc_minutes + 4.0 * hour_angle(-0.833)?;
    let fajr = solar_noon_utc_minutes - 4.0 * hour_angle(-20.0)? + ihtiyati_minutes;
    let dhuhr = solar_noon_utc_minutes + ihtiyati_minutes;
    let asr = solar_noon_utc_minutes + 4.0 * hour_angle(asr_altitude)? + ihtiyati_minutes;
    let maghrib = sunset + ihtiyati_minutes;
    let isha = solar_noon_utc_minutes + 4.0 * hour_angle(-18.0)? + ihtiyati_minutes;

    let to_local = |minutes: f64| -> DateTime<FixedOffset> {
        (utc_midnight + Duration::minutes(minutes.round() as i64)).with_timezone(&offset)
    };

    Some(PrayerTimes {
        date,
        times: [
            to_local(fajr),
            to_local(dhuhr),
            to_local(asr),
            to_local(maghrib),
            to_local(isha),
        ],
        sunrise: to_local(sunrise),
        sunset: to_local(sunset),
        solar_noon: to_local(solar_noon_utc_minutes),
        sun_latitude_rad: latitude,
        sun_declination_rad: declination,
        sun_max_altitude_deg: 90.0 - (city.latitude - declination.to_degrees()).abs(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrayerPhase {
    Day,
    Night,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrayerMoment {
    pub phase: PrayerPhase,
    pub next_prayer: Prayer,
    pub next_at: DateTime<FixedOffset>,
    pub orbit_progress: f32,
    pub orb_height: f32,
    pub arc_start: DateTime<FixedOffset>,
    pub arc_end: DateTime<FixedOffset>,
}

pub fn moment_for(city: &City, now: DateTime<FixedOffset>) -> Option<PrayerMoment> {
    let date = now.date_naive();
    let today = times_for(city, date)?;
    let tomorrow = times_for(city, date.succ_opt()?)?;

    let (phase, arc_start, arc_end, orbit_progress, orb_height) =
        if now >= today.sunrise && now <= today.sunset {
            (
                PrayerPhase::Day,
                today.sunrise,
                today.sunset,
                fraction_between(now, today.sunrise, today.sunset),
                today.sun_height(now),
            )
        } else {
            let (arc_start, arc_end) = if now < today.sunrise {
                let yesterday = times_for(city, date.pred_opt()?)?;
                (yesterday.sunset, today.sunrise)
            } else {
                (today.sunset, tomorrow.sunrise)
            };
            let progress = fraction_between(now, arc_start, arc_end);
            let orb_height = (std::f32::consts::PI * progress).sin();
            (PrayerPhase::Night, arc_start, arc_end, progress, orb_height)
        };

    let (next_prayer, next_at) = today
        .iter()
        .find(|(_, time)| *time > now)
        .unwrap_or((Prayer::Fajr, tomorrow.times[Prayer::Fajr as usize]));

    Some(PrayerMoment {
        phase,
        next_prayer,
        next_at,
        orbit_progress: orbit_progress.clamp(0.0, 1.0),
        orb_height: orb_height.clamp(0.0, 1.0),
        arc_start,
        arc_end,
    })
}

fn fraction_between(
    now: DateTime<FixedOffset>,
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
) -> f32 {
    let total = (end - start).num_seconds().max(1) as f32;
    ((now - start).num_seconds() as f32 / total).clamp(0.0, 1.0)
}

pub fn alerts_for_date(settings: &PrayerSettings, date: NaiveDate) -> Vec<PrayerAlert> {
    let settings = settings.normalized();
    if !settings.enabled {
        return Vec::new();
    }
    let city = settings.city();
    let Some(times) = times_for(city, date) else {
        return Vec::new();
    };

    let mut alerts = Vec::with_capacity(PRAYER_COUNT * 2);
    for (prayer, time) in times.iter() {
        let time_label = time.format("%H:%M").to_string();
        if settings.reminder_minutes > 0 {
            let at = time - Duration::minutes(settings.reminder_minutes as i64);
            alerts.push(PrayerAlert {
                prayer,
                kind: PrayerAlertKind::Reminder,
                at: at.with_timezone(&Utc),
                title: format!(
                    "{} {} menit lagi",
                    prayer.name_id(),
                    settings.reminder_minutes
                ),
                time_label: time_label.clone(),
                city_label: city.name.to_string(),
            });
        }
        alerts.push(PrayerAlert {
            prayer,
            kind: PrayerAlertKind::Adhan,
            at: time.with_timezone(&Utc),
            title: format!("Waktu {}", prayer.name_id()),
            time_label,
            city_label: city.name.to_string(),
        });
    }
    alerts.sort_by_key(|alert| alert.at);
    alerts
}

pub fn alerts_around(settings: &PrayerSettings, now: DateTime<Utc>) -> Vec<PrayerAlert> {
    let settings = settings.normalized();
    let local_now = now.with_timezone(&settings.city().offset());
    let today = local_now.date_naive();
    let Some(tomorrow) = today.succ_opt() else {
        return Vec::new();
    };

    let mut alerts = alerts_for_date(&settings, today);
    alerts.extend(alerts_for_date(&settings, tomorrow));
    alerts.sort_by_key(|alert| alert.at);
    alerts
}

pub fn next_alert(settings: &PrayerSettings, now: DateTime<Utc>) -> Option<PrayerAlert> {
    alerts_around(settings, now)
        .into_iter()
        .find(|alert| alert.at > now)
}

pub fn latest_due_alert(
    settings: &PrayerSettings,
    now: DateTime<Utc>,
    grace_minutes: i64,
) -> Option<PrayerAlert> {
    let grace = Duration::minutes(grace_minutes);
    alerts_around(settings, now)
        .into_iter()
        .filter(|alert| alert.at <= now && alert.at + grace > now)
        .max_by_key(|alert| alert.at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid test date")
    }

    fn labels(times: &PrayerTimes) -> Vec<String> {
        times
            .iter()
            .map(|(_, time)| time.format("%H:%M").to_string())
            .collect()
    }

    #[test]
    fn jakarta_kemenag_times_match_reference() {
        let times = times_for(&CITIES[0], date(2026, 10, 28)).expect("Indonesian latitude");

        assert_eq!(
            labels(&times),
            vec!["04:09", "11:38", "14:52", "17:48", "18:59"]
        );
    }

    #[test]
    fn alerts_include_reminder_and_adhan() {
        let settings = PrayerSettings {
            enabled: true,
            city: 0,
            reminder_minutes: 10,
        };
        let alerts = alerts_for_date(&settings, date(2026, 10, 28));

        assert_eq!(alerts.len(), 10);
        assert_eq!(alerts[0].kind, PrayerAlertKind::Reminder);
        assert_eq!(alerts[0].title, "Subuh 10 menit lagi");
        assert_eq!(alerts[0].time_label, "04:09");
        assert_eq!(alerts[1].kind, PrayerAlertKind::Adhan);
        assert_eq!(alerts[1].title, "Waktu Subuh");
        assert_eq!(alerts[1].city_label, "Jakarta");
    }

    #[test]
    fn reminder_can_be_disabled() {
        let settings = PrayerSettings {
            enabled: true,
            city: 0,
            reminder_minutes: 0,
        };
        let alerts = alerts_for_date(&settings, date(2026, 10, 28));

        assert_eq!(alerts.len(), PRAYER_COUNT);
        assert!(
            alerts
                .iter()
                .all(|alert| alert.kind == PrayerAlertKind::Adhan)
        );
    }

    #[test]
    fn disabled_settings_produce_no_alerts() {
        let settings = PrayerSettings {
            enabled: false,
            city: 0,
            reminder_minutes: 10,
        };

        assert!(alerts_for_date(&settings, date(2026, 10, 28)).is_empty());
        assert!(alerts_around(&settings, Utc::now()).is_empty());
    }

    #[test]
    fn latest_due_alert_finds_adhan_inside_grace_window() {
        let settings = PrayerSettings::default();
        let times = times_for(settings.city(), date(2026, 10, 28)).expect("valid times");
        let fajr = times.times[0];
        let now = fajr.with_timezone(&Utc) + Duration::minutes(1);

        let alert = latest_due_alert(&settings, now, 2).expect("adhan is due");

        assert_eq!(alert.prayer, Prayer::Fajr);
        assert_eq!(alert.kind, PrayerAlertKind::Adhan);
    }

    #[test]
    fn moment_tracks_day_segment_progress() {
        let city = &CITIES[0];
        let now = city
            .offset()
            .with_ymd_and_hms(2026, 10, 28, 8, 0, 0)
            .unwrap();
        let moment = moment_for(city, now).expect("valid moment");

        assert_eq!(moment.phase, PrayerPhase::Day);
        assert_eq!(moment.next_prayer, Prayer::Dhuhr);
        assert_eq!(moment.next_at.format("%H:%M").to_string(), "11:38");
        assert!(moment.orbit_progress > 0.0 && moment.orbit_progress < 0.25);
        assert!(moment.orb_height > 0.0);
        assert!(moment.arc_start < now && moment.arc_end > now);
    }

    #[test]
    fn moment_switches_to_night_after_sunset() {
        let city = &CITIES[0];
        let now = city
            .offset()
            .with_ymd_and_hms(2026, 10, 28, 21, 0, 0)
            .unwrap();
        let moment = moment_for(city, now).expect("valid moment");

        assert_eq!(moment.phase, PrayerPhase::Night);
        assert_eq!(moment.next_prayer, Prayer::Fajr);
        assert_eq!(moment.next_at.date_naive(), date(2026, 10, 29));
        assert!(moment.orbit_progress > 0.0 && moment.orbit_progress < 1.0);
        assert_eq!(moment.arc_start.format("%H:%M").to_string(), "17:46");
        assert_eq!(moment.arc_end.date_naive(), date(2026, 10, 29));
    }

    #[test]
    fn night_progress_runs_from_sunset_to_sunrise() {
        let city = &CITIES[0];
        let evening = city
            .offset()
            .with_ymd_and_hms(2026, 10, 28, 19, 0, 0)
            .unwrap();
        let early = city
            .offset()
            .with_ymd_and_hms(2026, 10, 29, 4, 0, 0)
            .unwrap();

        let evening = moment_for(city, evening).expect("valid evening moment");
        let early = moment_for(city, early).expect("valid early moment");

        assert!(evening.orbit_progress < 0.2);
        assert!(early.orbit_progress > 0.8);
        assert!(evening.orb_height < early.orb_height);
    }

    #[test]
    fn moment_before_fajr_targets_today_fajr() {
        let city = &CITIES[0];
        let now = city
            .offset()
            .with_ymd_and_hms(2026, 10, 28, 3, 0, 0)
            .unwrap();
        let moment = moment_for(city, now).expect("valid moment");

        assert_eq!(moment.phase, PrayerPhase::Night);
        assert_eq!(moment.next_prayer, Prayer::Fajr);
        assert_eq!(moment.next_at.date_naive(), date(2026, 10, 28));
        assert_eq!(moment.arc_end.date_naive(), date(2026, 10, 28));
        assert!(moment.orb_height > 0.0);
    }

    #[test]
    fn solar_arc_tracks_sunrise_and_sunset() {
        let times = times_for(&CITIES[0], date(2026, 10, 28)).expect("Indonesian latitude");

        assert_eq!(times.sunrise.format("%H:%M").to_string(), "05:27");
        assert_eq!(times.sunset.format("%H:%M").to_string(), "17:46");
        assert!(times.sun_height(times.sunrise) < 0.02);
        assert!(times.sun_height(times.solar_noon) > 0.99);
        assert!(times.sun_height(times.sunset) < 0.02);
    }

    #[test]
    fn sky_is_deep_night_with_stars_at_astral_night() {
        let sky = sky_from_altitude(-30.0);
        assert!(sky.star_opacity > 0.9);
        // Deep night zenith is near-black and cool.
        assert!(sky.top[0] < 0.05 && sky.top[1] < 0.05 && sky.top[2] < 0.12);
    }

    #[test]
    fn sky_has_no_stars_at_full_day() {
        let sky = sky_from_altitude(50.0);
        assert_eq!(sky.star_opacity, 0.0);
        // Day zenith is blue-dominant.
        assert!(sky.top[2] > sky.top[0]);
    }

    #[test]
    fn sky_warms_at_the_horizon() {
        let sunset = sky_from_altitude(-0.5);
        let noon = sky_from_altitude(60.0);
        // At the horizon the horizon channel reddens relative to noon.
        assert!(sunset.horizon[0] > noon.horizon[0]);
        // Twilight keeps some stars but far fewer than deep night.
        assert!(sunset.star_opacity > 0.0 && sunset.star_opacity < 0.3);
    }

    #[test]
    fn sky_interpolates_monotonically_toward_night() {
        // Descending altitude = the sun sinking. Star opacity must never decrease.
        let mut prev = 0.0;
        for alt in [30.0f64, 5.0, 0.0, -1.0, -6.0, -12.0, -20.0] {
            let sky = sky_from_altitude(alt);
            if alt <= 0.0 {
                assert!(sky.star_opacity >= prev, "star opacity dropped at {alt}");
            }
            prev = sky.star_opacity;
        }
    }

    #[test]
    fn sky_for_tracks_time_of_day() {
        let city = &CITIES[0];
        let noon = city
            .offset()
            .with_ymd_and_hms(2026, 10, 28, 12, 30, 0)
            .unwrap();
        let midnight = city
            .offset()
            .with_ymd_and_hms(2026, 10, 28, 0, 30, 0)
            .unwrap();

        let day_sky = sky_for(city, noon).expect("valid day sky");
        let night_sky = sky_for(city, midnight).expect("valid night sky");

        assert_eq!(day_sky.star_opacity, 0.0);
        assert!(night_sky.star_opacity > 0.8);
    }

    #[test]
    fn latest_due_alert_ignores_old_events() {
        let settings = PrayerSettings::default();
        let times = times_for(settings.city(), date(2026, 10, 28)).expect("valid times");
        let fajr = times.times[0];
        let now = fajr.with_timezone(&Utc) + Duration::minutes(3);

        assert!(latest_due_alert(&settings, now, 2).is_none());
    }

    #[test]
    fn next_alert_is_in_the_future() {
        let settings = PrayerSettings::default();
        let now = Utc.with_ymd_and_hms(2026, 10, 28, 0, 0, 0).unwrap();
        let alert = next_alert(&settings, now).expect("next alert");

        assert!(alert.at > now);
    }

    #[test]
    fn kendari_and_baubau_are_wita_presets() {
        let kendari = CITIES
            .iter()
            .find(|city| city.name == "Kendari")
            .expect("Kendari");
        let baubau = CITIES
            .iter()
            .find(|city| city.name == "Baubau")
            .expect("Baubau");

        assert_eq!(kendari.region, "WITA");
        assert_eq!(baubau.region, "WITA");
        assert_eq!(kendari.utc_offset_minutes, 480);
        assert_eq!(baubau.utc_offset_minutes, 480);
        assert!(times_for(kendari, date(2026, 10, 28)).is_some());
        assert!(times_for(baubau, date(2026, 10, 28)).is_some());
    }

    #[test]
    fn settings_are_normalized() {
        let settings = PrayerSettings {
            enabled: true,
            city: 99,
            reminder_minutes: 999,
        }
        .normalized();

        assert_eq!(settings.city, (99 % CITIES.len()) as u32);
        assert_eq!(settings.reminder_minutes, MAX_REMINDER_MINUTES);
    }
}
