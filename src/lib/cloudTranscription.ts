import { invoke } from "@tauri-apps/api/core";

export interface CloudConfig {
  enabled: boolean;
  base_url: string;
  model: string;
  language: string;
  keywords: string[];
}

export interface CloudModel {
  id: string;
  name: string;
  description: string;
  hourly_min: number | null;
  hourly_max: number | null;
  price_estimated: boolean;
  native_streaming: boolean;
}

export const defaultCloudConfig: CloudConfig = {
  enabled: false,
  base_url: "https://openrouter.ai/api/v1",
  model: "microsoft/mai-transcribe-2",
  language: "",
  keywords: [],
};

export const fetchCloudModels = (baseUrl: string) =>
  invoke<CloudModel[]>("fetch_cloud_transcription_models", { baseUrl });
export const cloudKeyIsSaved = (baseUrl: string) =>
  invoke<boolean>("cloud_key_is_saved", { baseUrl });
export const saveCloudConfig = (config: CloudConfig, apiKey: string | null) =>
  invoke<void>("save_cloud_transcription_settings", { config, apiKey });

export function formatHourlyPrice(
  model: CloudModel,
  locale: string,
): string | null {
  if (model.hourly_min === null || !Number.isFinite(model.hourly_min))
    return null;
  const format = new Intl.NumberFormat(locale, {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 3,
  });
  const low = format.format(model.hourly_min);
  return model.hourly_max !== null &&
    model.hourly_max > model.hourly_min * 1.001
    ? `${low}–${format.format(model.hourly_max)}`
    : low;
}
