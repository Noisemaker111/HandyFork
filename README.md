# HandyFork

Windows dictation with OpenRouter speech-to-text. A personal fork of [Handy](https://github.com/cjpais/Handy). **Not affiliated with or endorsed by Handy.**

Press **Ctrl+Alt+Space**, speak, and HandyFork pastes your words into whatever app you are using. Cloud transcription defaults to **Microsoft MAI-Transcribe-2**.

## Download

Windows installer: **[latest release](https://github.com/Noisemaker111/HandyFork/releases/latest)**

The installer is unsigned, so Windows SmartScreen may warn on first run. Choose **More info → Run anyway**.

HandyFork uses its own app ID and settings, so it can sit next to upstream Handy. The default shortcut is **Ctrl+Alt+Space** so the two do not collide.

## Setup

1. Install HandyFork and grant microphone access if Windows asks.
2. Open **General → Cloud transcription**.
3. Enable **Use cloud**, paste an [OpenRouter](https://openrouter.ai/) API key, and click **Save settings**.
4. Leave the default model (`microsoft/mai-transcribe-2`) or pick another from the searchable catalog.
5. Hold **Ctrl+Alt+Space**, talk, release.

API keys are stored in Windows Credential Manager, not in settings JSON. Audio is uploaded only when cloud transcription is enabled.

Details, pricing notes, and streaming limits: [CLOUD_TRANSCRIPTION.md](CLOUD_TRANSCRIPTION.md).

## What this fork adds

- OpenRouter speech-to-text from the settings UI
- Searchable model picker with live hourly price ranges
- Microsoft MAI-Transcribe-2 selected by default
- Vocabulary hints for MAI (`phraseList.phrases`)
- Separate settings, icon, and disabled upstream auto-updates

Local Whisper / Parakeet / ONNX models still work if you turn cloud transcription off.

## Build from source

```powershell
bun install --frozen-lockfile
bun run tauri build --bundles nsis
```

Windows local inference in this fork is built with CPU backends, so the Vulkan SDK is not required. See [BUILD.md](BUILD.md) for the upstream Handy build notes.

## License

MIT. Original Handy © 2025 CJ Pais. HandyFork modifications © 2026 Noisemaker111.
