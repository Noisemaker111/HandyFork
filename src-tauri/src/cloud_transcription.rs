//! Hosted STT for HandyFork. Credentials never enter the settings store or logs.
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{stream, StreamExt};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Cursor,
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

static RECORDING_CONFIG: Mutex<Option<CloudTranscriptionSettings>> = Mutex::new(None);

pub fn begin_recording(config: CloudTranscriptionSettings) {
    if let Ok(mut snapshot) = RECORDING_CONFIG.lock() {
        *snapshot = Some(config);
    }
}

pub fn recording_config() -> Option<CloudTranscriptionSettings> {
    RECORDING_CONFIG
        .lock()
        .ok()
        .and_then(|config| config.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct CloudTranscriptionSettings {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub language: String,
    pub keywords: Vec<String>,
}

impl Default for CloudTranscriptionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "https://openrouter.ai/api/v1".into(),
            model: "microsoft/mai-transcribe-2".into(),
            language: String::new(),
            keywords: Vec::new(),
        }
    }
}

fn normalized_base_url(base: &str) -> Result<String, String> {
    let url = Url::parse(base.trim()).map_err(|_| "Enter a valid API base URL.")?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("The API URL must use HTTPS (HTTP is allowed only on localhost).".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("The API base URL cannot contain credentials, a query, or a fragment.".into());
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn normalize(mut config: CloudTranscriptionSettings) -> Result<CloudTranscriptionSettings, String> {
    config.base_url = normalized_base_url(&config.base_url)?;
    config.model = config.model.trim().to_string();
    if config.model.is_empty() {
        return Err("Enter a transcription model ID.".into());
    }
    config.language = config.language.trim().to_lowercase();
    if !config.language.is_empty()
        && (config.language.len() != 2 || !config.language.bytes().all(|b| b.is_ascii_lowercase()))
    {
        return Err(
            "Use a two-letter language code, or leave it empty for automatic detection.".into(),
        );
    }
    config.keywords = config
        .keywords
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if config.keywords.len() > 100 || config.keywords.iter().any(|s| s.len() > 200) {
        return Err("Use at most 100 keyword hints, each under 200 bytes.".into());
    }
    Ok(config)
}

#[cfg(target_os = "windows")]
fn credential(base: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(
        "com.jk101.handyfork.transcription",
        &normalized_base_url(base)?,
    )
    .map_err(|_| "Windows Credential Manager is unavailable.".into())
}

fn read_key(base: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        match credential(base)?.get_password() {
            Ok(key) => Ok(key),
            Err(keyring::Error::NoEntry) => Ok(String::new()),
            Err(_) => {
                Err("Could not read the transcription key from Windows Credential Manager.".into())
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = base;
        Err("Cloud credentials in this fork currently require Windows.".into())
    }
}

fn write_key(base: &str, key: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let entry = credential(base)?;
        if key.trim().is_empty() {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => Err("Could not remove the saved transcription key.".into()),
            }
        } else {
            entry.set_password(key.trim()).map_err(|_| {
                "Could not save the transcription key in Windows Credential Manager.".into()
            })
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (base, key);
        Err("Cloud credentials in this fork currently require Windows.".into())
    }
}

#[tauri::command]
#[specta::specta]
pub fn cloud_key_is_saved(base_url: String) -> Result<bool, String> {
    Ok(!read_key(&base_url)?.is_empty())
}

#[tauri::command]
#[specta::specta]
pub fn save_cloud_transcription_settings(
    app: AppHandle,
    config: CloudTranscriptionSettings,
    api_key: Option<String>,
) -> Result<(), String> {
    let config = normalize(config)?;
    if app
        .state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>()
        .is_recording()
    {
        return Err("Finish recording before changing transcription settings.".into());
    }
    if let Some(key) = api_key {
        write_key(&config.base_url, &key)?;
    }
    // Saving setup without a key is intentional; the recording path reports a
    // useful error until the user supplies one. Never fall back to a local model.
    let mut settings = crate::settings::get_settings(&app);
    if config.enabled {
        settings.onboarding_completed = true;
    }
    settings.cloud_transcription = config;
    crate::settings::write_settings(&app, settings);
    app.store(crate::portable::store_path(
        crate::settings::SETTINGS_STORE_PATH,
    ))
    .map_err(|_| "Could not open the settings store.")?
    .save()
    .map_err(|_| "Could not save transcription settings to disk.")?;
    let _ = app.emit(
        "settings-changed",
        json!({"setting": "cloud_transcription"}),
    );
    crate::tray::update_tray_menu(&app);
    Ok(())
}

fn client() -> Result<&'static Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(90))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| "Could not initialize the transcription HTTP client.".to_string())
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn samples_to_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|_| "Could not encode recording.")?;
        for sample in samples {
            writer
                .write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .map_err(|_| "Could not encode recording.")?;
        }
        writer
            .finalize()
            .map_err(|_| "Could not finish encoding recording.")?;
    }
    Ok(cursor.into_inner())
}

fn request_body(config: &CloudTranscriptionSettings, samples: &[f32]) -> Result<Value, String> {
    let mut body = json!({
        "model": config.model,
        "input_audio": { "data": STANDARD.encode(samples_to_wav(samples)?), "format": "wav" },
        "response_format": "json"
    });
    if !config.language.is_empty() {
        body["language"] = json!(config.language);
    }
    if config.model.starts_with("microsoft/mai-transcribe-") && !config.keywords.is_empty() {
        body["provider"] =
            json!({"options": {"azure": {"phraseList": {"phrases": config.keywords}}}});
    }
    Ok(body)
}

fn status_error(status: StatusCode) -> String {
    let detail = match status.as_u16() {
        401 | 403 => "The API key was rejected. Check your saved key and account permissions.",
        402 => "Your provider account needs credit.",
        429 => "The provider is rate-limiting requests. Try again shortly.",
        400 | 404 | 422 => {
            "Check the transcription model ID, language, keyword hints, and API base URL."
        }
        413 => "The recording is too large. Try a shorter dictation.",
        500..=599 => "The transcription provider is temporarily unavailable.",
        _ => "The provider rejected the transcription request.",
    };
    format!(
        "Cloud transcription failed (HTTP {}). {detail}",
        status.as_u16()
    )
}

async fn send_transcription(
    config: &CloudTranscriptionSettings,
    key: &str,
    samples: &[f32],
) -> Result<String, String> {
    let config = normalize(config.clone())?;
    let response = client()?
        .post(format!("{}/audio/transcriptions", config.base_url))
        .bearer_auth(key)
        // Desktop app identity: OpenRouter requires both URL and title.
        .header("HTTP-Referer", "http://localhost/handyfork")
        .header("X-OpenRouter-Title", "HandyFork")
        .json(&request_body(&config, samples)?)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Cloud transcription timed out. Try again with a shorter recording."
            } else {
                "Could not reach the transcription provider. Check your connection and API URL."
            }
            .to_string()
        })?;
    if !response.status().is_success() {
        return Err(status_error(response.status()));
    }
    // Do not surface/log raw upstream bodies: they may contain audio, transcripts,
    // or reflected credentials. Parse only the documented text field.
    #[derive(Deserialize)]
    struct Transcript {
        text: String,
    }
    let result: Transcript = response
        .json()
        .await
        .map_err(|_| "The provider returned an invalid transcription response.".to_string())?;
    Ok(result.text.trim().to_string())
}

pub async fn transcribe(
    config: &CloudTranscriptionSettings,
    samples: &[f32],
) -> Result<String, String> {
    if samples.is_empty() {
        return Ok(String::new());
    }
    if samples.len() > 16_000 * 60 * 15 {
        return Err("Cloud dictation is limited to 15 minutes per recording.".into());
    }
    let key = read_key(&config.base_url)?;
    if key.is_empty() {
        return Err("Add your OpenRouter API key in General → Cloud transcription.".into());
    }
    send_transcription(config, &key, samples).await
}

#[tauri::command]
#[specta::specta]
pub async fn fetch_cloud_transcription_models(base_url: String) -> Result<Vec<CloudModel>, String> {
    let base = normalized_base_url(&base_url)?;
    let response = client()?
        .get(format!("{base}/models?output_modalities=transcription"))
        .send()
        .await
        .map_err(|_| "Could not fetch transcription models.".to_string())?;
    if !response.status().is_success() {
        return Err(status_error(response.status()));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| "Invalid model list.".to_string())?;
    let data: Vec<Value> = body["data"]
        .as_array()
        .ok_or("Invalid model list.")?
        .iter()
        .filter(|m| {
            m["architecture"]["output_modalities"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v == "transcription"))
        })
        .cloned()
        .collect();
    let http = client()?.clone();
    let mut models: Vec<CloudModel> = stream::iter(data)
        .map(|m| {
            let base = base.clone();
            let http = http.clone();
            async move {
                let id = m["id"].as_str().unwrap_or_default().to_string();
                let mut model = CloudModel {
                    name: m["name"].as_str().unwrap_or(&id).to_string(),
                    description: m["description"].as_str().unwrap_or_default().to_string(),
                    id,
                    hourly_min: None,
                    hourly_max: None,
                    price_estimated: false,
                    native_streaming: false,
                };
                // Public catalog prices omit the billing unit. Endpoint provider tags
                // let us normalize known units without guessing from the magnitude.
                // Unknown/new providers remain unpriced, never silently shown as free.
                if let Ok(response) = http
                    .get(format!("{base}/models/{}/endpoints", model.id))
                    .send()
                    .await
                {
                    if response.status().is_success() {
                        if let Ok(details) = response.json::<Value>().await {
                            if let Some(endpoints) = details["data"]["endpoints"].as_array() {
                                let prices: Vec<f64> = endpoints
                                    .iter()
                                    .filter_map(|e| hourly_price(&model.id, e))
                                    .collect();
                                model.hourly_min = prices.iter().copied().reduce(f64::min);
                                model.hourly_max = prices.iter().copied().reduce(f64::max);
                                model.price_estimated = model.id == "openai/gpt-4o-transcribe"
                                    || model.id == "openai/gpt-4o-mini-transcribe";
                            }
                        }
                    }
                }
                model
            }
        })
        .buffer_unordered(6)
        .collect()
        .await;
    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CloudModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub hourly_min: Option<f64>,
    pub hourly_max: Option<f64>,
    pub price_estimated: bool,
    /// OpenRouter's STT endpoint currently has no documented streaming transport.
    /// Never infer API streaming from a model's name or its upstream capabilities.
    pub native_streaming: bool,
}

fn hourly_price(model: &str, endpoint: &Value) -> Option<f64> {
    let prompt = endpoint["pricing"]["prompt"]
        .as_str()?
        .parse::<f64>()
        .ok()?;
    if !prompt.is_finite() || prompt < 0.0 {
        return None;
    }
    let provider = endpoint["tag"].as_str()?.split('/').next()?;
    // Billing-unit references are recorded in CLOUD_TRANSCRIPTION.md.
    let multiplier = match provider {
        "azure" | "xai" | "groq" => 1.0,
        "deepinfra" | "alibaba" | "fish-audio" => 3600.0,
        "together" | "mistral" | "deepgram" | "google-vertex" => 60.0,
        "openai" if model == "openai/gpt-4o-transcribe" => return Some(0.36),
        "openai" if model == "openai/gpt-4o-mini-transcribe" => return Some(0.18),
        "openai" if model == "openai/whisper-1" || model == "openai/gpt-transcribe" => 60.0,
        _ => return None,
    };
    Some(prompt * multiplier)
}

#[cfg(test)]
mod tests;
