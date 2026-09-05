# HandyFork

Personal Windows fork of [Handy](https://github.com/cjpais/Handy), with OpenRouter speech-to-text. Not affiliated with or endorsed by Handy.

## Using the app

Open **General → Cloud transcription**, enable **Use cloud**, choose a model, paste your OpenRouter key, and click **Save settings**. No terminal commands or model IDs are needed. Setup can be completed before adding the key. The default selection is Microsoft MAI-Transcribe-2.

The searchable catalog refreshes from OpenRouter's public STT API. Model names, descriptions, and provider prices come from the service. MAI vocabulary hints are sent through Azure's `phraseList.phrases` option. Spoken language is auto-detected unless selected explicitly.

Audio is uploaded only for a cloud transcription. Keys are stored per API endpoint in Windows Credential Manager, outside settings JSON. Blank key input preserves the saved key; **Remove key** deletes it. Audio history stays in the fork's own app-data directory. Cancellation aborts the request and suppresses paste. Provider errors do not trigger a local-model fallback or automatic billable retry.

OpenRouter activity attribution uses `X-OpenRouter-Title: HandyFork` with `HTTP-Referer: http://localhost/handyfork`, a stable desktop app identifier. OpenRouter requires both headers for localhost apps: https://openrouter.ai/docs/app-attribution. Attribution applies to new requests.

## Streaming

As verified on 2026-09-04, OpenRouter's `/audio/transcriptions` API accepts an audio file and returns a completed JSON transcript. It does not document a live microphone/WebSocket or streamed STT-response interface. Consequently, all OpenRouter models are correctly labeled **Transcribes after recording stops**, even when their model names mention streaming. This fork preserves Handy's existing native streaming path for local models. It does not simulate streaming by repeatedly uploading overlapping recordings (which adds cost and changes recognition context).

Reference: https://openrouter.ai/docs/guides/overview/multimodal/stt

## Pricing

OpenRouter's public catalog exposes `pricing.prompt` without the STT billing unit. Its endpoint API also exposes the provider tag. The picker normalizes known provider billing units using the table below, and displays the minimum–maximum hourly prices of the returned providers. Unknown units or failed price lookups appear as unavailable. It does not infer a billing unit from a price's magnitude.

| Provider                                   | Input pricing unit                                         |
| ------------------------------------------ | ---------------------------------------------------------- |
| Azure, xAI, Groq                           | Audio hour                                                 |
| DeepInfra, Alibaba, Fish Audio             | Audio second                                               |
| Together, Mistral, Deepgram, Google Vertex | Audio minute                                               |
| OpenAI Whisper 1 / GPT Transcribe          | Audio minute                                               |
| OpenAI GPT-4o Transcribe / Mini            | Token billing; displayed hourly estimates of $0.36 / $0.18 |

Rates refresh live; the known provider-unit mapping and OpenAI token-model estimates were reviewed on 2026-09-04. If a provider changes its billing units, update `hourly_price`. The picker labels estimates with ≈. OpenRouter's actual request billing remains authoritative.

Sources:

- https://openrouter.ai/api/v1/models?output_modalities=transcription
- https://openrouter.ai/api/v1/models/microsoft/mai-transcribe-2/endpoints
- https://openrouter.ai/openai/whisper-large-v3-turbo/pricing
- https://openrouter.ai/qwen/qwen3-asr-flash-2026-02-10/pricing
- https://openrouter.ai/collections/speech-to-text-models
- https://developers.openai.com/api/docs/pricing

## Build and validation (developer reference)

The normal user workflow is entirely in Settings. These commands are only for rebuilding the app:

```powershell
bun install --frozen-lockfile
bun run build
bun run lint
bun test scripts/cloud-pricing.test.ts
cargo test --manifest-path src-tauri/Cargo.toml cloud_transcription --lib
bun run tauri build --bundles nsis
```

Windows local inference is built with CPU backends in this fork, so the Vulkan SDK is not required. Cloud transcription does not use a local ASR model. Local ONNX models and native streaming remain available. This fork has its own product name, identifier (`com.jk101.handyfork`), app icon, and settings. Upstream Handy auto-updates are disabled. The installer is locally built and unsigned.

The implementation adapts the remote recording/upload idea from Handy PR #804 to the current streaming-aware pipeline, using OpenRouter's JSON audio API to carry MAI keyword options.
