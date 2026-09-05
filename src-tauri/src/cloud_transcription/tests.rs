use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
};

fn mock_server(
    status: &str,
    body: &str,
) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/api/v1", server.local_addr().unwrap());
    let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let (mut socket, _) = server.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let count = socket.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
        }
        tx.send(String::from_utf8(bytes).unwrap()).unwrap();
        socket.write_all(response.as_bytes()).unwrap();
    });
    (address, rx, worker)
}

#[test]
fn correct_audio_encoding_and_mai_keyword_request() {
    let config = CloudTranscriptionSettings {
        keywords: vec!["jgengine".into(), "Bun".into()],
        language: "en".into(),
        ..Default::default()
    };
    let body = request_body(&config, &[0.0, 0.5, -0.5, 2.0]).unwrap();
    assert_eq!(body["model"], "microsoft/mai-transcribe-2");
    assert_eq!(body["language"], "en");
    assert_eq!(
        body["provider"]["options"]["azure"]["phraseList"]["phrases"],
        json!(["jgengine", "Bun"])
    );
    assert!(body.get("prompt").is_none());
    let wav = STANDARD
        .decode(body["input_audio"]["data"].as_str().unwrap())
        .unwrap();
    let mut reader = hound::WavReader::new(Cursor::new(wav)).unwrap();
    assert_eq!(reader.spec().sample_rate, 16000);
    assert_eq!(reader.spec().channels, 1);
    assert_eq!(
        reader
            .samples::<i16>()
            .map(Result::unwrap)
            .collect::<Vec<_>>(),
        vec![0, 16383, -16383, 32767]
    );
}

#[test]
fn hints_are_not_sent_to_unrelated_models_and_auto_language_is_omitted() {
    let config = CloudTranscriptionSettings {
        model: "openai/whisper-1".into(),
        keywords: vec!["Bun".into()],
        ..Default::default()
    };
    let body = request_body(&config, &[0.0]).unwrap();
    assert!(body.get("provider").is_none());
    assert!(body.get("language").is_none());
}

#[test]
fn endpoint_validation_prevents_accidental_credential_disclosure() {
    for url in [
        "http://example.com/v1",
        "https://user:secret@example.com/v1",
        "https://example.com/v1?key=secret",
        "file:///tmp/key",
    ] {
        assert!(normalized_base_url(url).is_err());
    }
    assert_eq!(
        normalized_base_url(" https://openrouter.ai/api/v1/ ").unwrap(),
        "https://openrouter.ai/api/v1"
    );
    assert!(normalized_base_url("http://127.0.0.1:1234/v1").is_ok());
}

#[test]
fn converts_provider_units_without_guessing_from_price_magnitude() {
    let endpoint = |tag: &str, price: &str| json!({"tag": tag, "pricing": {"prompt": price}});
    assert_eq!(
        hourly_price("microsoft/mai-transcribe-2", &endpoint("azure", "0.1")),
        Some(0.1)
    );
    assert_eq!(
        hourly_price("openai/whisper-large-v3-turbo", &endpoint("groq", "0.04")),
        Some(0.04)
    );
    assert!(
        (hourly_price(
            "openai/whisper-large-v3-turbo",
            &endpoint("deepinfra", "0.00000333")
        )
        .unwrap()
            - 0.011988)
            .abs()
            < 1e-9
    );
    assert_eq!(
        hourly_price("openai/whisper-1", &endpoint("openai", "0.006")),
        Some(0.36)
    );
    assert_eq!(
        hourly_price("future/model", &endpoint("unknown", "0.1")),
        None
    );
    assert_eq!(
        hourly_price("future/model", &endpoint("azure", "NaN")),
        None
    );
}

#[tokio::test]
async fn http_request_preserves_auth_model_audio_and_unicode_response() {
    let (url, request, worker) = mock_server(
        "200 OK",
        r#"{"text":"  jgengine — café  ","usage":{"cost":0.001}}"#,
    );
    let config = CloudTranscriptionSettings {
        base_url: url,
        ..Default::default()
    };
    assert_eq!(
        send_transcription(&config, "test-secret", &[0.0, 0.2])
            .await
            .unwrap(),
        "jgengine — café"
    );
    let request = request.recv().unwrap();
    assert!(request.starts_with("POST /api/v1/audio/transcriptions HTTP/1.1"));
    assert!(request
        .to_lowercase()
        .contains("authorization: bearer test-secret"));
    assert!(request
        .to_lowercase()
        .contains("http-referer: http://localhost/handyfork"));
    assert!(request
        .to_lowercase()
        .contains("x-openrouter-title: handyfork"));
    let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["input_audio"]["format"], "wav");
    assert_eq!(body["response_format"], "json");
    worker.join().unwrap();
}

#[tokio::test]
async fn rejects_http_errors_without_leaking_upstream_bodies() {
    let (url, _request, worker) = mock_server(
        "401 Unauthorized",
        r#"{"error":"test-secret and private transcript"}"#,
    );
    let config = CloudTranscriptionSettings {
        base_url: url,
        ..Default::default()
    };
    let error = send_transcription(&config, "test-secret", &[0.0])
        .await
        .unwrap_err();
    assert!(error.contains("401"));
    assert!(!error.contains("test-secret"));
    assert!(!error.contains("private transcript"));
    worker.join().unwrap();
}

#[tokio::test]
async fn rejects_malformed_success_response() {
    let (url, _request, worker) = mock_server("200 OK", r#"{"unexpected":"shape"}"#);
    let config = CloudTranscriptionSettings {
        base_url: url,
        ..Default::default()
    };
    assert!(send_transcription(&config, "test-key", &[0.0])
        .await
        .unwrap_err()
        .contains("invalid transcription response"));
    worker.join().unwrap();
}

#[test]
fn settings_have_no_api_key_and_preserve_endpoint_snapshot() {
    let config = CloudTranscriptionSettings::default();
    let json = serde_json::to_value(&config).unwrap();
    assert!(json.get("api_key").is_none());
    begin_recording(config.clone());
    assert_eq!(recording_config().unwrap().base_url, config.base_url);
}
