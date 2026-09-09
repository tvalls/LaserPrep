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
  | { kind: "savingProject" }
  | { kind: "projectSaved" }
  | { kind: "openingProject" }
  | { kind: "projectOpened" }
  | { kind: "error"; message: string };

type ValidationReport = {
  width: number;
  height: number;
  toneCount: number;
  totalPaths: number;
  closedPaths: number;
  openPaths: number;
  degeneratePaths: number;
  totalNodes: number;
  invalidCoordinatePaths: number;
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

type ProjectParams = {
  dpi: number;
  toneCount: number;
  minAreaPx2: number;
  includeLegend: boolean;
  mergeAdjacent: boolean;
};

type OpenedProject = {
  sourceFileName: string | null;
  params: ProjectParams;
  preset: PresetName | null;
  classification: Classification | null;
  result: { svg: string; validation: ValidationReport } | null;
};

type BatchItemStatus = "pending" | "converting" | "success" | "error";

type BatchItem = {
  path: string;
  fileName: string;
  status: BatchItemStatus;
  svg?: string;
  errorMessage?: string;
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

const BATCH_STATUS_LABEL_KEYS: Record<BatchItemStatus, string> = {
  pending: "batch.status.pending",
  converting: "batch.status.converting",
  success: "batch.status.success",
  error: "batch.status.error",
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
  const [presetName, setPresetName] = useState<PresetName | null>(null);
  const [hasSourceImage, setHasSourceImage] = useState(false);
  const [sourceFileName, setSourceFileName] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [batchItems, setBatchItems] = useState<BatchItem[]>([]);
  const [batchRunning, setBatchRunning] = useState(false);

  useEffect(() => {
    if (!svg) {
      setPreviewUrl(null);
      return;
    }

    const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [svg]);

  function applyConversionResult(result: ConversionResult) {
    setSvg(result.svg);
    setValidation(result.validation);
    setClassification(result.classification);
    setSuggestedPreset(result.suggestedPreset);
  }

  function clearConversionResult() {
    setSvg(null);
    setValidation(null);
    setClassification(null);
    setSuggestedPreset(null);
  }

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
      applyConversionResult(result);
      setPresetName(null);
      setHasSourceImage(true);
      setSourceFileName(path.split(/[/\\]/).pop() ?? path);
      setStatus({ kind: "converted" });
    } catch (error) {
      clearConversionResult();
      setHasSourceImage(false);
      setSourceFileName(null);
      setStatus({ kind: "error", message: String(error) });
    }
  }

  async function handleReconvert() {
    if (!hasSourceImage) {
      return;
    }

    setStatus({ kind: "converting" });
    try {
      const result = await invoke<ConversionResult>("convert_current_source", {
        dpi,
        toneCount,
        minAreaPx2,
        includeLegend,
        mergeAdjacent,
      });
      applyConversionResult(result);
      setStatus({ kind: "converted" });
    } catch (error) {
      setStatus({ kind: "error", message: String(error) });
    }
  }

  function applyPreset(preset: Preset) {
    setToneCount(preset.toneCount);
    setMinAreaPx2(preset.minAreaPx2);
    setIncludeLegend(preset.includeLegend);
    setMergeAdjacent(preset.mergeAdjacent);
    setPresetName(preset.name);
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

  async function handleSaveProject() {
    if (!hasSourceImage) {
      return;
    }

    const path = await save({
      filters: [{ name: "LaserPrep Project", extensions: ["lvp"] }],
    });
    if (!path) {
      return;
    }

    setStatus({ kind: "savingProject" });
    try {
      await invoke("save_project", {
        request: {
          projectPath: path,
          params: { dpi, toneCount, minAreaPx2, includeLegend, mergeAdjacent },
          preset: presetName,
          classification,
          result: svg && validation ? { svg, validation } : null,
        },
      });
      setStatus({ kind: "projectSaved" });
    } catch (error) {
      setStatus({ kind: "error", message: String(error) });
    }
  }

  async function handleOpenProject() {
    const path = await open({
      multiple: false,
      filters: [{ name: "LaserPrep Project", extensions: ["lvp"] }],
    });
    if (typeof path !== "string") {
      return;
    }

    setStatus({ kind: "openingProject" });
    try {
      const project = await invoke<OpenedProject>("open_project", { path });
      setDpi(project.params.dpi);
      setToneCount(project.params.toneCount);
      setMinAreaPx2(project.params.minAreaPx2);
      setIncludeLegend(project.params.includeLegend);
      setMergeAdjacent(project.params.mergeAdjacent);
      setPresetName(project.preset);
      setClassification(project.classification);
      setSuggestedPreset(null);
      if (project.result) {
        setSvg(project.result.svg);
        setValidation(project.result.validation);
      } else {
        setSvg(null);
        setValidation(null);
      }
      setHasSourceImage(true);
      setSourceFileName(project.sourceFileName);
      setStatus({ kind: "projectOpened" });
    } catch (error) {
      setStatus({ kind: "error", message: String(error) });
    }
  }

  /**
   * Converts each selected file one at a time (CLAUDE.md Section 12:
   * "fila, progresso, sucesso/erro por item"). Sequential rather than
   * parallel: `convert_image_file` also records its source image in
   * the backend's shared "current source" state (used by Reconvert /
   * Save Project), and concurrent calls would race on that.
   */
  async function handleBatchImport() {
    const paths = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }],
    });
    const selected = Array.isArray(paths) ? paths : [];
    if (selected.length === 0) {
      return;
    }

    setBatchItems(
      selected.map((path) => ({
        path,
        fileName: path.split(/[/\\]/).pop() ?? path,
        status: "pending",
      })),
    );
    setBatchRunning(true);

    for (let index = 0; index < selected.length; index++) {
      const path = selected[index];
      setBatchItems((items) =>
        items.map((item, i) =>
          i === index ? { ...item, status: "converting" } : item,
        ),
      );

      try {
        const result = await invoke<ConversionResult>("convert_image_file", {
          path,
          dpi,
          toneCount,
          minAreaPx2,
          includeLegend,
          mergeAdjacent,
        });
        setBatchItems((items) =>
          items.map((item, i) =>
            i === index ? { ...item, status: "success", svg: result.svg } : item,
          ),
        );
      } catch (error) {
        setBatchItems((items) =>
          items.map((item, i) =>
            i === index
              ? { ...item, status: "error", errorMessage: String(error) }
              : item,
          ),
        );
      }
    }

    setBatchRunning(false);
  }

  async function handleSaveAllBatchSvgs() {
    const successfulItems = batchItems.filter(
      (item) => item.status === "success" && item.svg,
    );
    if (successfulItems.length === 0) {
      return;
    }

    const directory = await open({ directory: true });
    if (typeof directory !== "string") {
      return;
    }

    const files = successfulItems.map((item) => ({
      path: `${directory}/${item.fileName.replace(/\.[^./\\]+$/, "")}.svg`,
      svg: item.svg as string,
    }));

    try {
      await invoke("save_svg_files", { files });
    } catch (error) {
      setStatus({ kind: "error", message: String(error) });
    }
  }

  function handleToneCountChange(value: string) {
    const next = Number(value);
    if (!Number.isNaN(next)) {
      setToneCount(next);
      setPresetName(null);
    }
  }

  function handleDpiChange(value: string) {
    const next = Number(value);
    if (!Number.isNaN(next)) {
      setDpi(next);
      setPresetName(null);
    }
  }

  function handleMinAreaChange(value: string) {
    const next = Number(value);
    if (!Number.isNaN(next)) {
      setMinAreaPx2(next);
      setPresetName(null);
    }
  }

  function handleIncludeLegendChange(checked: boolean) {
    setIncludeLegend(checked);
    setPresetName(null);
  }

  function handleMergeAdjacentChange(checked: boolean) {
    setMergeAdjacent(checked);
    setPresetName(null);
  }

  const batchCompletedCount = batchItems.filter(
    (item) => item.status === "success" || item.status === "error",
  ).length;
  const batchSuccessCount = batchItems.filter(
    (item) => item.status === "success",
  ).length;

  const statusText =
    status.kind === "converting"
      ? t("app.status.converting")
      : status.kind === "converted"
        ? t("app.status.converted", { toneCount })
        : status.kind === "savingProject"
          ? t("app.status.savingProject")
          : status.kind === "projectSaved"
            ? t("app.status.projectSaved")
            : status.kind === "openingProject"
              ? t("app.status.openingProject")
              : status.kind === "projectOpened"
                ? t("app.status.projectOpened", {
                    fileName: sourceFileName ?? "",
                  })
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
          onChange={(event) => handleIncludeLegendChange(event.target.checked)}
        />

        <label htmlFor="merge-adjacent">{t("mergeAdjacent.label")}</label>
        <input
          id="merge-adjacent"
          type="checkbox"
          checked={mergeAdjacent}
          onChange={(event) => handleMergeAdjacentChange(event.target.checked)}
        />
      </div>

      <button type="button" onClick={() => void handleImport()}>
        {t("import.button")}
      </button>
      <button type="button" onClick={() => void handleExport()} disabled={!svg}>
        {t("export.button")}
      </button>
      <button
        type="button"
        onClick={() => void handleReconvert()}
        disabled={!hasSourceImage}
      >
        {t("reconvert.button")}
      </button>
      <button
        type="button"
        onClick={() => void handleSaveProject()}
        disabled={!hasSourceImage}
      >
        {t("saveProject.button")}
      </button>
      <button type="button" onClick={() => void handleOpenProject()}>
        {t("openProject.button")}
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

      <section aria-label={t("batch.heading")}>
        <h2>{t("batch.heading")}</h2>
        <button
          type="button"
          onClick={() => void handleBatchImport()}
          disabled={batchRunning}
        >
          {t("batch.importButton")}
        </button>

        {batchItems.length > 0 && (
          <>
            <p>
              {t("batch.progress", {
                completed: batchCompletedCount,
                total: batchItems.length,
              })}
            </p>
            <ul>
              {batchItems.map((item) => (
                <li key={item.path}>
                  {item.fileName}
                  {": "}
                  {item.status === "error"
                    ? t(BATCH_STATUS_LABEL_KEYS.error, {
                        message: item.errorMessage,
                      })
                    : t(BATCH_STATUS_LABEL_KEYS[item.status])}
                </li>
              ))}
            </ul>
            <button
              type="button"
              onClick={() => void handleSaveAllBatchSvgs()}
              disabled={batchRunning || batchSuccessCount === 0}
            >
              {t("batch.saveAllButton")}
            </button>
          </>
        )}
      </section>

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
