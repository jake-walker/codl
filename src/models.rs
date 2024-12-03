use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampMilliSeconds};

#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoCobalt {
    pub version: String,
    pub url: String,
    #[serde_as(as = "TimestampMilliSeconds<String>")]
    pub start_time: SystemTime,
    pub duration_limit: i64,
    pub services: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfoGit {
    pub commit: String,
    pub branch: String,
    pub remote: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo {
    pub cobalt: ServerInfoCobalt,
    pub git: ServerInfoGit,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    video_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_bitrate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    download_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    youtube_video_codec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    youtube_dub_lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    always_proxy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tiktok_full_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tiktok_h265: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    twitter_gif: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    youtube_hls: Option<bool>,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        ProcessOptions {
            video_quality: None,
            audio_format: None,
            audio_bitrate: None,
            filename_style: None,
            download_mode: None,
            youtube_video_codec: None,
            youtube_dub_lang: None,
            always_proxy: None,
            disable_metadata: None,
            tiktok_full_audio: None,
            tiktok_h265: None,
            twitter_gif: None,
            youtube_hls: None,
        }
    }
}
