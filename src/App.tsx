import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { OFFICIALLY_SUPPORTED_LOCALES } from "./i18n";

const MIN_TONE_COUNT = 2;
const MAX_TONE_COUNT = 16;
const DEFAULT_TONE_COUNT = 5;
const DEFAULT_DPI = 96;
const DEFAULT_MIN_AREA_PX2 = 16;

type Status =
  | { kind: "idle" }
  | { kind: "converting" }
  | { kind: "converted" }
  | { kind: "error"; message: string };

type ValidationReport = {
  totalPaths: number;
  openPaths: number;
  totalNodes: number;
  lightburnIncompatibilities: string[];
};

type ContentCategory =
  | "UniformBackground"
  | "Logo"
  | "Landscape"
  | "ComplexBackground"
  | "GenericPhoto";

type Classification = {
  category: ContentCategory;
  confidence: number;
};

type PresetName =
  | "Photo"
  | "Portrait"
  | "Animal"
  | "Logo"
  | "Drawing"
  | "Landscape";

type Preset = {
  name: PresetName;
  toneCount: number;
  minAreaPx2: number;
  includeLegend: boolean;
  mergeAdjacent: boolean;
};

type ConversionResult = {
  svg: string;
  validation: ValidationReport;
  classification: Classification;
  suggestedPreset: Preset;
};

const CATEGORY_LABEL_KEYS: Record<ContentCategory, string> = {
  UniformBackground: "category.uniformBackground",
  Logo: "category.logo",
  Landscape: "category.landscape",
  ComplexBackground: "category.complexBackground",
  GenericPhoto: "category.genericPhoto",
};

const PRESET_LABEL_KEYS: Record<PresetName, string> = {
  Photo: "preset.photo",
  Portrait: "preset.portrait",
  Animal: "preset.animal",
  Logo: "preset.logo",
  Drawing: "preset.drawing",
  Landscape: "preset.landscape",
};

export default function App() {
  const { t, i18n } = useTranslation();
  const [toneCount, setToneCount] = useState(DEFAULT_TONE_COUNT);
  const [dpi, setDpi] = useState(DEFAULT_DPI);
  const [minAreaPx2, setMinAreaPx2] = useState(DEFAULT_MIN_AREA_PX2);
  const [includeLegend, setIncludeLegend] = useState(false);
  const [mergeAdjacent, setMergeAdjacent] = useState(false);
  const [svg, setSvg] = useState<string | null>(null);
  const [validation, setValidation] = useState<ValidationReport | null>(null);
  const [classification, setClassification] = useState<Classification | null>(
    null,
  );
  const [suggestedPreset, setSuggestedPreset] = useState<Preset | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  useEffect(() => {
    if (!svg) {
      setPreviewUrl(null);
      return;
    }

    const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [svg]);

  async function handleImport() {
    const path = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }],
    });
    if (typeof path !== "string") {
      return;
    }

    setStatus({ kind: "converting" });
    try {
      const result = await invoke<ConversionResult>("convert_image_file", {
        path,
        dpi,
        toneCount,
        minAreaPx2,
        includeLegend,
        mergeAdjacent,
      });
      setSvg(result.svg);
      setValidation(result.validation);
      setClassification(result.classification);
      setSuggestedPreset(result.suggestedPreset);
      setStatus({ kind: "converted" });
    } catch (error) {
      setSvg(null);
      setValidation(null);
      setClassification(null);
      setSuggestedPreset(null);
      setStatus({ kind: "error", message: String(error) });
    }
  }

  function applyPreset(preset: Preset) {
    setToneCount(preset.toneCount);
    setMinAreaPx2(preset.minAreaPx2);
    setIncludeLegend(preset.includeLegend);
    setMergeAdjacent(preset.mergeAdjacent);
  }

  async function handleExport() {
    if (!svg) {
      return;
    }

    const path = await save({ filters: [{ name: "SVG", extensions: ["svg"] }] });
    if (!path) {
      return;
    }

    try {
      await invoke("save_svg_file", { path, svg });
    } catch (error) {
      setStatus({ kind: "error", message: String(error) });
    }
  }

  function handleToneCountChange(value: string) {
    const next = Number(value);
    if (!Number.isNaN(next)) {
      setToneCount(next);
    }
  }

  function handleDpiChange(value: string) {
    const next = Number(value);
    if (!Number.isNaN(next)) {
      setDpi(next);
    }
  }

  function handleMinAreaChange(value: string) {
    const next = Number(value);
    if (!Number.isNaN(next)) {
      setMinAreaPx2(next);
    }
  }

  const statusText =
    status.kind === "converting"
      ? t("app.status.converting")
      : status.kind === "converted"
        ? t("app.status.converted", { toneCount })
        : status.kind === "error"
          ? t("app.status.error", { message: status.message })
          : t("app.status.idle");

  return (
    <main>
      <h1>{t("app.title")}</h1>
      <p>{t("app.tagline")}</p>

      <div>
        <label htmlFor="tone-count">{t("toneCount.label")}</label>
        <input
          id="tone-count"
          type="number"
          min={MIN_TONE_COUNT}
          max={MAX_TONE_COUNT}
          value={toneCount}
          onChange={(event) => handleToneCountChange(event.target.value)}
        />

        <label htmlFor="dpi">{t("dpi.label")}</label>
        <input
          id="dpi"
          type="number"
          min={1}
          value={dpi}
          onChange={(event) => handleDpiChange(event.target.value)}
        />

        <label htmlFor="min-area">{t("minArea.label")}</label>
        <input
          id="min-area"
          type="number"
          min={1}
          value={minAreaPx2}
          onChange={(event) => handleMinAreaChange(event.target.value)}
        />

        <label htmlFor="include-legend">{t("legend.label")}</label>
        <input
          id="include-legend"
          type="checkbox"
          checked={includeLegend}
          onChange={(event) => setIncludeLegend(event.target.checked)}
        />

        <label htmlFor="merge-adjacent">{t("mergeAdjacent.label")}</label>
        <input
          id="merge-adjacent"
          type="checkbox"
          checked={mergeAdjacent}
          onChange={(event) => setMergeAdjacent(event.target.checked)}
        />
      </div>

      <button type="button" onClick={() => void handleImport()}>
        {t("import.button")}
      </button>
      <button type="button" onClick={() => void handleExport()} disabled={!svg}>
        {t("export.button")}
      </button>

      <p role="status">{statusText}</p>

      {classification && (
        <p>
          {t("classification.summary", {
            category: t(CATEGORY_LABEL_KEYS[classification.category]),
            confidence: Math.round(classification.confidence * 100),
          })}
        </p>
      )}

      {suggestedPreset && (
        <p>
          {t("preset.suggestion", {
            preset: t(PRESET_LABEL_KEYS[suggestedPreset.name]),
          })}{" "}
          <button type="button" onClick={() => applyPreset(suggestedPreset)}>
            {t("preset.applyButton")}
          </button>
        </p>
      )}

      {validation && (
        <p>
          {t("validation.summary", {
            totalPaths: validation.totalPaths,
            totalNodes: validation.totalNodes,
          })}
          {validation.openPaths > 0 && (
            <> {t("validation.openPathsWarning", { count: validation.openPaths })}</>
          )}
          {validation.lightburnIncompatibilities.length > 0 && (
            <> {t("validation.lightburnWarning")}</>
          )}
        </p>
      )}

      {previewUrl && <img src={previewUrl} alt={t("app.title")} />}

      <label htmlFor="language-switcher">
        {t("language.switcher.label")}
      </label>
      <select
        id="language-switcher"
        value={i18n.resolvedLanguage}
        onChange={(event) => {
          void i18n.changeLanguage(event.target.value);
        }}
      >
        {OFFICIALLY_SUPPORTED_LOCALES.map((locale) => (
          <option key={locale} value={locale}>
            {locale}
          </option>
        ))}
      </select>
    </main>
  );
}
