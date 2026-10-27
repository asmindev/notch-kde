use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IslandView {
    #[default]
    Compact,
    Wide,
    Expand,
}

/// Backwards compatibility alias
pub type IslandMode = IslandView;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ActiveModule {
    #[default]
    System,
    Media,
    Alert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(i32)]
pub enum AlertKind {
    #[default]
    None = 0,
    Charging = 1,
    LowBattery = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(i32)]
pub enum MusicVisualMode {
    #[default]
    ArtworkPrimary = 0,
    PetPrimary = 1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryState {
    pub percentage: u8,
    pub is_charging: bool,
}

impl Default for BatteryState {
    fn default() -> Self {
        Self {
            percentage: 100,
            is_charging: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MediaTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: String,
    pub is_playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MediaSource {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub is_playing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MediaUpdate {
    pub track: MediaTrack,
    pub sources: Vec<MediaSource>,
    pub active_player: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TimeInfo {
    pub time_str: String,
    pub date_str: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSpectrum {
    pub bars: [f32; 8],
}

impl Default for AudioSpectrum {
    fn default() -> Self {
        Self { bars: [0.0; 8] }
    }
}
