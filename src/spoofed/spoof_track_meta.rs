use serde::{Deserialize, Serialize}; // Include if you are still using serde on this spoofed model

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub track_num: String,
    pub genre: String,
    pub time: String,
    pub size: String,
    pub rating: usize,
    pub codec: String,
    pub bitrate: String,
    pub sample_rate: String,
}
