import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cloud, RefreshCw, Check, Search } from "lucide-react";
import { useSettingsStore } from "@/stores/settingsStore";
import {
  cloudKeyIsSaved,
  defaultCloudConfig,
  fetchCloudModels,
  formatHourlyPrice,
  saveCloudConfig,
  type CloudConfig,
  type CloudModel,
} from "@/lib/cloudTranscription";

const inputClass =
  "w-full rounded-md border border-mid-gray/30 bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-logo-primary";

export function CloudTranscriptionSettings({
  onComplete,
}: {
  onComplete?: () => void;
}) {
  const { t, i18n } = useTranslation();
  const stored = useSettingsStore((s) => s.settings?.cloud_transcription);
  const refreshSettings = useSettingsStore((s) => s.refreshSettings);
  const [config, setConfig] = useState<CloudConfig>(() => ({
    ...defaultCloudConfig,
    ...stored,
    ...(onComplete ? { enabled: true } : {}),
  }));
  const [apiKey, setApiKey] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [models, setModels] = useState<CloudModel[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [catalogError, setCatalogError] = useState("");
  const [saveError, setSaveError] = useState("");
  const [saved, setSaved] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (stored && !dirty)
      setConfig({ ...stored, ...(onComplete ? { enabled: true } : {}) });
  }, [stored, dirty, onComplete]);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setCatalogError("");
    fetchCloudModels(config.base_url)
      .then((data) => {
        if (active) setModels(data);
      })
      .catch((error: unknown) => {
        if (active) setCatalogError(String(error));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    cloudKeyIsSaved(config.base_url)
      .then((value) => {
        if (active) setHasKey(value);
      })
      .catch(() => {
        if (active) setHasKey(false);
      });
    return () => {
      active = false;
    };
  }, [config.base_url, refresh]);

  function changeConfig(update: Partial<CloudConfig>) {
    setDirty(true);
    setConfig((previous) => ({ ...previous, ...update }));
    setSaved(false);
  }

  async function save(removeKey = false) {
    setSaving(true);
    setSaveError("");
    try {
      await saveCloudConfig(config, removeKey ? "" : apiKey.trim() || null);
      setApiKey("");
      await refreshSettings();
      setDirty(false);
      setHasKey(await cloudKeyIsSaved(config.base_url));
      setSaved(true);
      if (!removeKey) onComplete?.();
    } catch (error) {
      setSaveError(String(error));
    } finally {
      setSaving(false);
    }
  }

  const visibleModels = models.filter((m) =>
    `${m.name} ${m.id}`.toLowerCase().includes(query.toLowerCase()),
  );
  const selected = models.find((m) => m.id === config.model);

  return (
    <section
      className="rounded-xl border border-mid-gray/20 bg-background p-5 space-y-4 text-left"
      aria-label={t("cloud.title")}
    >
      <div className="flex items-center gap-3">
        <Cloud className="text-logo-primary" size={24} aria-hidden />
        <div className="flex-1">
          <h2 className="text-base font-semibold">{t("cloud.title")}</h2>
          <p className="text-xs text-text/60">{t("cloud.subtitle")}</p>
        </div>
        <label className="flex gap-2 items-center text-sm">
          <input
            type="checkbox"
            checked={config.enabled}
            onChange={(e) => changeConfig({ enabled: e.target.checked })}
          />
          {t("cloud.useCloud")}
        </label>
      </div>
      <p className="text-xs text-text/60">{t("cloud.audioNotice")}</p>
      <div className="space-y-1">
        <label htmlFor="cloud-api-key" className="text-sm font-medium">
          {t("cloud.apiKey")}
        </label>
        <div className="flex gap-2">
          <input
            id="cloud-api-key"
            className={inputClass}
            type="password"
            autoComplete="off"
            spellCheck={false}
            value={apiKey}
            onChange={(e) => {
              setApiKey(e.target.value);
              setSaved(false);
            }}
            placeholder={t(
              hasKey ? "cloud.keySavedPlaceholder" : "cloud.keyPlaceholder",
            )}
          />
          {hasKey && (
            <button
              type="button"
              disabled={saving}
              onClick={() => save(true)}
              className="px-2 text-xs whitespace-nowrap"
            >
              {t("cloud.removeKey")}
            </button>
          )}
        </div>
        <p className="text-xs text-text/60">
          {t(hasKey ? "cloud.keySaved" : "cloud.keyLater")}
        </p>
      </div>
      <div className="flex gap-2 items-center">
        <Search size={16} aria-hidden />
        <input
          className={inputClass}
          aria-label={t("cloud.search")}
          placeholder={t("cloud.search")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button
          type="button"
          className="flex gap-1 items-center text-xs whitespace-nowrap px-2 py-2"
          onClick={() => setRefresh((n) => n + 1)}
          disabled={loading}
        >
          <RefreshCw
            size={14}
            className={loading ? "animate-spin" : ""}
            aria-hidden
          />
          {t("cloud.refresh")}
        </button>
      </div>
      {catalogError && (
        <p role="alert" className="text-sm text-red-500">
          {t("cloud.catalogError", { error: catalogError })}
        </p>
      )}
      <div
        role="radiogroup"
        aria-label={t("cloud.models")}
        aria-busy={loading}
        className="max-h-72 overflow-y-auto space-y-2 pr-1"
      >
        {visibleModels.map((model) => {
          const price = formatHourlyPrice(model, i18n.language);
          return (
            <label
              key={model.id}
              className={`flex gap-3 items-start rounded-lg border p-3 cursor-pointer ${config.model === model.id ? "border-logo-primary bg-logo-primary/5" : "border-mid-gray/20 hover:bg-mid-gray/5"}`}
            >
              <input
                type="radio"
                name="cloud-model"
                value={model.id}
                checked={config.model === model.id}
                onChange={() => changeConfig({ model: model.id })}
                className="mt-1"
              />
              <span className="flex-1 min-w-0">
                <span className="block text-sm font-medium">{model.name}</span>
                <span className="block text-xs text-text/60 mt-1">
                  {t("cloud.afterStop")}
                </span>
              </span>
              <span className="text-right whitespace-nowrap text-sm font-medium">
                {price
                  ? t(
                      model.price_estimated
                        ? "cloud.estimatedPrice"
                        : "cloud.hourlyPrice",
                      { price },
                    )
                  : t("cloud.priceUnknown")}
              </span>
            </label>
          );
        })}
        {!loading && !visibleModels.length && (
          <p className="text-sm text-text/60 p-3">{t("cloud.noModels")}</p>
        )}
        {loading && !models.length && (
          <p role="status" className="text-sm text-text/60 p-3">
            {t("cloud.loading")}
          </p>
        )}
      </div>
      <div className="rounded-lg bg-mid-gray/5 p-3 space-y-2 text-xs text-text/70">
        <p>{t("cloud.selected", { model: selected?.name ?? config.model })}</p>
        {selected && <p>{selected.description}</p>}
        <p>{t("cloud.streamingNotice")}</p>
        <p>{t("cloud.pricingNotice")}</p>
      </div>
      <div className="space-y-1">
        <label htmlFor="cloud-language" className="text-sm font-medium">
          {t("cloud.language")}
        </label>
        <select
          id="cloud-language"
          className={inputClass}
          value={config.language}
          onChange={(e) => changeConfig({ language: e.target.value })}
        >
          <option value="">{t("cloud.autoLanguage")}</option>
          {[
            "en",
            "es",
            "fr",
            "de",
            "it",
            "pt",
            "nl",
            "ja",
            "ko",
            "zh",
            "hi",
            "ar",
            "ru",
          ].map((code) => (
            <option key={code} value={code}>
              {new Intl.DisplayNames([i18n.language], { type: "language" }).of(
                code,
              )}
            </option>
          ))}
        </select>
      </div>
      {config.model.startsWith("microsoft/mai-transcribe-") && (
        <div className="space-y-1">
          <label htmlFor="cloud-keywords" className="text-sm font-medium">
            {t("cloud.keywords")}
          </label>
          <textarea
            id="cloud-keywords"
            className={inputClass}
            rows={2}
            value={config.keywords.join("\n")}
            placeholder={t("cloud.keywordPlaceholder")}
            onChange={(e) =>
              changeConfig({ keywords: e.target.value.split("\n") })
            }
          />
          <p className="text-xs text-text/60">{t("cloud.keywordHint")}</p>
        </div>
      )}
      {saveError && (
        <p role="alert" className="text-sm text-red-500">
          {saveError}
        </p>
      )}
      <div className="flex gap-3 items-center">
        <button
          type="button"
          disabled={saving || (!!onComplete && !config.enabled)}
          onClick={() => save()}
          className="rounded-lg bg-logo-primary px-4 py-2 text-sm text-white font-medium disabled:opacity-50"
        >
          {t(
            saving
              ? "cloud.saving"
              : onComplete
                ? "cloud.continue"
                : "cloud.save",
          )}
        </button>
        {saved && (
          <span role="status" className="flex gap-1 items-center text-sm">
            <Check size={16} aria-hidden />
            {t("cloud.saved")}
          </span>
        )}
      </div>
    </section>
  );
}
