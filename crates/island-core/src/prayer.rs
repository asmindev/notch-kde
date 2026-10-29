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
}

impl PrayerTimes {
    pub fn iter(&self) -> impl Iterator<Item = (Prayer, DateTime<FixedOffset>)> + '_ {
        Prayer::ALL.into_iter().zip(self.times.iter().copied())
    }
}

pub fn times_for(city: &City, date: NaiveDate) -> Option<PrayerTimes> {
    let day_of_year = date.ordinal() as f64;
    let gamma = TAU / 365.0 * (day_of_year - 1.0 + 0.5);
    let equation_of_time = 229.18
        * (0.000075 + 0.001868 * gamma.cos()
            - 0.032077 * gamma.sin()
            - 0.014615 * (2.0 * gamma).cos()
            - 0.040849 * (2.0 * gamma).sin());
    let declination = 0.006918 - 0.399912 * gamma.cos()
        + 0.070257 * gamma.sin()
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

    let fajr = solar_noon_utc_minutes - 4.0 * hour_angle(-20.0)? + ihtiyati_minutes;
    let dhuhr = solar_noon_utc_minutes + ihtiyati_minutes;
    let asr = solar_noon_utc_minutes + 4.0 * hour_angle(asr_altitude)? + ihtiyati_minutes;
    let maghrib = solar_noon_utc_minutes + 4.0 * hour_angle(-0.833)? + ihtiyati_minutes;
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
    })
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
                title: format!("{} {} menit lagi", prayer.name_id(), settings.reminder_minutes),
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
        assert!(alerts
            .iter()
            .all(|alert| alert.kind == PrayerAlertKind::Adhan));
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
        let kendari = CITIES.iter().find(|city| city.name == "Kendari").expect("Kendari");
        let baubau = CITIES.iter().find(|city| city.name == "Baubau").expect("Baubau");

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
