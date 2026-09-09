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

type LaserOptimizationScore = {
  travelDistance: number;
  optimalTravelDistance: number;
  efficiency: number;
};

type ConversionResult = {
  svg: string;
  validation: ValidationReport;
  classification: Classification;
  suggestedPreset: Preset;
  optimizationScore: LaserOptimizationScore;
};

type ProjectParams = {
  dpi: number;
  toneCount: number;
  minAreaPx2: number;
  includeLegend: boolean;
  mergeAdjacent: boolean;
  orderPaths: boolean;
};

type OpenedProject = {
  sourceFileName: string | null;
  params: ProjectParams;
  preset: PresetName | null;
  classification: Classification | null;
  result: {
    svg: string;
    validation: ValidationReport;
    optimizationScore: LaserOptimizationScore | null;
  } | null;
};

type ParamsSnapshot = {
  dpi: number;
  toneCount: number;
  minAreaPx2: number;
  includeLegend: boolean;
  mergeAdjacent: boolean;
  orderPaths: boolean;
  presetName: PresetName | null;
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

/**
 * Distinct per-layer colors for the "LightBurn-style" preview mode,
 * echoing how LightBurn itself distinguishes cut layers by color
 * rather than literally reproducing its renderer.
 */
const LAYER_COLORS = [
  "#e63946",
  "#1d4ed8",
  "#15803d",
  "#f97316",
  "#7c3aed",
  "#0891b2",
];

/**
 * Applies per-tone visibility and optional layer coloring to a
 * generated SVG document, purely by injecting a `<style>` block that
 * targets the `id="tone-N"` groups every generated document already
 * has — no need to touch svggen or re-run the pipeline for a preview
 * change. Browsers apply embedded SVG stylesheets even when the SVG
 * is only ever used as an `<img>` source, so this works without
 * inlining the SVG into the page's own DOM. Never used for
 * export/save — those always use the pristine, unmodified SVG.
 */
function buildPreviewSvg(
  svg: string,
  toneVisibility: boolean[],
  colorizeByLayer: boolean,
): string {
  const rules = toneVisibility
    .map((visible, tone) => {
      if (!visible) {
        return `#tone-${tone}{display:none}`;
      }
      if (colorizeByLayer) {
        const color = LAYER_COLORS[tone % LAYER_COLORS.length];
        return `#tone-${tone} path{fill:none;stroke:${color};stroke-width:1px}`;
      }
      return null;
    })
    .filter((rule): rule is string => rule !== null);

  if (rules.length === 0) {
    return svg;
  }
  return svg.replace(/^(<svg[^>]*>)/, `$1<style>${rules.join("")}</style>`);
}

export default function App() {
  const { t, i18n } = useTranslation();
  const [toneCount, setToneCount] = useState(DEFAULT_TONE_COUNT);
  const [dpi, setDpi] = useState(DEFAULT_DPI);
  const [minAreaPx2, setMinAreaPx2] = useState(DEFAULT_MIN_AREA_PX2);
  // Each number input shows its own draft text rather than binding
  // directly to the committed number: while `value` is controlled by
  // `toneCount` etc., clearing the field to retype it would otherwise
  // have nothing to display until a full new number parses, and
  // `Number("")` is `0` (not `NaN`), so a fully-cleared field would
  // silently commit `0`. These stay in sync with the committed values
  // via the effect below, including when undo/redo or importing/
  // opening a project changes them programmatically.
  const [toneCountText, setToneCountText] = useState(String(DEFAULT_TONE_COUNT));
  const [dpiText, setDpiText] = useState(String(DEFAULT_DPI));
  const [minAreaText, setMinAreaText] = useState(String(DEFAULT_MIN_AREA_PX2));
  const [includeLegend, setIncludeLegend] = useState(false);
  const [mergeAdjacent, setMergeAdjacent] = useState(false);
  const [orderPaths, setOrderPaths] = useState(false);
  const [svg, setSvg] = useState<string | null>(null);
  const [validation, setValidation] = useState<ValidationReport | null>(null);
  const [optimizationScore, setOptimizationScore] =
    useState<LaserOptimizationScore | null>(null);
  const [classification, setClassification] = useState<Classification | null>(
    null,
  );
  const [suggestedPreset, setSuggestedPreset] = useState<Preset | null>(null);
  const [presetName, setPresetName] = useState<PresetName | null>(null);
  const [hasSourceImage, setHasSourceImage] = useState(false);
  const [sourceFileName, setSourceFileName] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [originalImageUrl, setOriginalImageUrl] = useState<string | null>(null);
  const [toneVisibility, setToneVisibility] = useState<boolean[]>([]);
  const [colorizeByLayer, setColorizeByLayer] = useState(false);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [batchItems, setBatchItems] = useState<BatchItem[]>([]);
  const [batchRunning, setBatchRunning] = useState(false);
  const [paramsHistory, setParamsHistory] = useState<{
    past: ParamsSnapshot[];
    future: ParamsSnapshot[];
  }>({ past: [], future: [] });

  function currentParamsSnapshot(): ParamsSnapshot {
    return {
      dpi,
      toneCount,
      minAreaPx2,
      includeLegend,
      mergeAdjacent,
      orderPaths,
      presetName,
    };
  }

  /** The subset of `currentParamsSnapshot` the backend's `ConversionParams` expects (no `presetName`). */
  function currentParams(): ProjectParams {
    return { dpi, toneCount, minAreaPx2, includeLegend, mergeAdjacent, orderPaths };
  }

  function applyParamsSnapshot(snapshot: ParamsSnapshot) {
    setDpi(snapshot.dpi);
    setToneCount(snapshot.toneCount);
    setMinAreaPx2(snapshot.minAreaPx2);
    setIncludeLegend(snapshot.includeLegend);
    setMergeAdjacent(snapshot.mergeAdjacent);
    setOrderPaths(snapshot.orderPaths);
    setPresetName(snapshot.presetName);
  }

  /**
   * Records the current parameters as an undo point, then applies
   * `next`. Every user-initiated parameter change (manual edits,
   * applying a preset) goes through this — importing an image or
   * opening a project instead starts a fresh editing session (see
   * `resetParamsHistory`), since undoing into a different image's
   * parameter history wouldn't make sense.
   */
  function commitParamsChange(next: ParamsSnapshot) {
    setParamsHistory((history) => ({
      past: [...history.past, currentParamsSnapshot()],
      future: [],
    }));
    applyParamsSnapshot(next);
  }

  function resetParamsHistory() {
    setParamsHistory({ past: [], future: [] });
  }

  function handleUndo() {
    if (paramsHistory.past.length === 0) {
      return;
    }
    const previous = paramsHistory.past[paramsHistory.past.length - 1];
    setParamsHistory({
      past: paramsHistory.past.slice(0, -1),
      future: [currentParamsSnapshot(), ...paramsHistory.future],
    });
    applyParamsSnapshot(previous);
  }

  function handleRedo() {
    if (paramsHistory.future.length === 0) {
      return;
    }
    const [next, ...rest] = paramsHistory.future;
    setParamsHistory({
      past: [...paramsHistory.past, currentParamsSnapshot()],
      future: rest,
    });
    applyParamsSnapshot(next);
  }

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "z") {
        return;
      }
      event.preventDefault();
      if (event.shiftKey) {
        handleRedo();
      } else {
        handleUndo();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  useEffect(() => {
    setToneCountText(String(toneCount));
  }, [toneCount]);

  useEffect(() => {
    setDpiText(String(dpi));
  }, [dpi]);

  useEffect(() => {
    setMinAreaText(String(minAreaPx2));
  }, [minAreaPx2]);

  const previewSvg = svg ? buildPreviewSvg(svg, toneVisibility, colorizeByLayer) : null;

  useEffect(() => {
    if (!previewSvg) {
      setPreviewUrl(null);
      return;
    }

    const url = URL.createObjectURL(
      new Blob([previewSvg], { type: "image/svg+xml" }),
    );
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [previewSvg]);

  function applyConversionResult(result: ConversionResult) {
    setSvg(result.svg);
    setValidation(result.validation);
    setClassification(result.classification);
    setSuggestedPreset(result.suggestedPreset);
    setOptimizationScore(result.optimizationScore);
    setToneVisibility(Array(result.validation.toneCount).fill(true));
  }

  function clearConversionResult() {
    setSvg(null);
    setValidation(null);
    setClassification(null);
    setSuggestedPreset(null);
    setOptimizationScore(null);
    setToneVisibility([]);
  }

  /** Fetches the original (undecoded) source image for the before/after preview. */
  async function refreshOriginalImagePreview() {
    try {
      const dataUrl = await invoke<string>("current_source_image_data_url");
      setOriginalImageUrl(typeof dataUrl === "string" ? dataUrl : null);
    } catch {
      setOriginalImageUrl(null);
    }
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
        params: currentParams(),
      });
      applyConversionResult(result);
      setPresetName(null);
      resetParamsHistory();
      setHasSourceImage(true);
      setSourceFileName(path.split(/[/\\]/).pop() ?? path);
      setStatus({ kind: "converted" });
      void refreshOriginalImagePreview();
    } catch (error) {
      clearConversionResult();
      setHasSourceImage(false);
      setSourceFileName(null);
      setOriginalImageUrl(null);
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
        params: currentParams(),
      });
      applyConversionResult(result);
      setStatus({ kind: "converted" });
    } catch (error) {
      setStatus({ kind: "error", message: String(error) });
    }
  }

  function applyPreset(preset: Preset) {
    commitParamsChange({
      dpi,
      toneCount: preset.toneCount,
      minAreaPx2: preset.minAreaPx2,
      includeLegend: preset.includeLegend,
      mergeAdjacent: preset.mergeAdjacent,
      orderPaths,
      presetName: preset.name,
    });
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
          params: currentParams(),
          preset: presetName,
          classification,
          result:
            svg && validation
              ? { svg, validation, optimizationScore }
              : null,
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
      applyParamsSnapshot({ ...project.params, presetName: project.preset });
      resetParamsHistory();
      setClassification(project.classification);
      setSuggestedPreset(null);
      if (project.result) {
        setSvg(project.result.svg);
        setValidation(project.result.validation);
        setOptimizationScore(project.result.optimizationScore);
        setToneVisibility(Array(project.result.validation.toneCount).fill(true));
      } else {
        setSvg(null);
        setValidation(null);
        setOptimizationScore(null);
        setToneVisibility([]);
      }
      setHasSourceImage(true);
      setSourceFileName(project.sourceFileName);
      setStatus({ kind: "projectOpened" });
      void refreshOriginalImagePreview();
    } catch (error) {
      setOriginalImageUrl(null);
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
          params: currentParams(),
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

  /**
   * `Number("")` is `0`, not `NaN` — without the blank check below,
   * clearing a number input mid-edit would silently commit `0` as a
   * real value (and, with undo history tracking every committed
   * change, would insert a spurious undo step for it).
   */
  function parseNumberInput(value: string): number | null {
    if (value.trim() === "") {
      return null;
    }
    const next = Number(value);
    return Number.isNaN(next) ? null : next;
  }

  function handleToneCountChange(value: string) {
    setToneCountText(value);
    const next = parseNumberInput(value);
    if (next !== null && next !== toneCount) {
      commitParamsChange({ ...currentParamsSnapshot(), toneCount: next, presetName: null });
    }
  }

  function handleDpiChange(value: string) {
    setDpiText(value);
    const next = parseNumberInput(value);
    if (next !== null && next !== dpi) {
      commitParamsChange({ ...currentParamsSnapshot(), dpi: next, presetName: null });
    }
  }

  function handleMinAreaChange(value: string) {
    setMinAreaText(value);
    const next = parseNumberInput(value);
    if (next !== null && next !== minAreaPx2) {
      commitParamsChange({ ...currentParamsSnapshot(), minAreaPx2: next, presetName: null });
    }
  }

  function handleIncludeLegendChange(checked: boolean) {
    commitParamsChange({ ...currentParamsSnapshot(), includeLegend: checked, presetName: null });
  }

  function handleMergeAdjacentChange(checked: boolean) {
    commitParamsChange({ ...currentParamsSnapshot(), mergeAdjacent: checked, presetName: null });
  }

  function handleOrderPathsChange(checked: boolean) {
    commitParamsChange({ ...currentParamsSnapshot(), orderPaths: checked, presetName: null });
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
          value={toneCountText}
          onChange={(event) => handleToneCountChange(event.target.value)}
        />

        <label htmlFor="dpi">{t("dpi.label")}</label>
        <input
          id="dpi"
          type="number"
          min={1}
          value={dpiText}
          onChange={(event) => handleDpiChange(event.target.value)}
        />

        <label htmlFor="min-area">{t("minArea.label")}</label>
        <input
          id="min-area"
          type="number"
          min={1}
          value={minAreaText}
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

        <label htmlFor="order-paths">{t("orderPaths.label")}</label>
        <input
          id="order-paths"
          type="checkbox"
          checked={orderPaths}
          onChange={(event) => handleOrderPathsChange(event.target.checked)}
        />
      </div>

      <button
        type="button"
        onClick={handleUndo}
        disabled={paramsHistory.past.length === 0}
      >
        {t("undo.button")}
      </button>
      <button
        type="button"
        onClick={handleRedo}
        disabled={paramsHistory.future.length === 0}
      >
        {t("redo.button")}
      </button>

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

      {optimizationScore && (
        <p>
          {t("optimizationScore.summary", {
            efficiency: Math.round(optimizationScore.efficiency * 100),
          })}
        </p>
      )}

      {(originalImageUrl || previewUrl) && (
        <section aria-label={t("preview.heading")}>
          <h2>{t("preview.heading")}</h2>
          <div>
            {originalImageUrl && (
              <img src={originalImageUrl} alt={t("preview.original.alt")} />
            )}
            {previewUrl && (
              <img src={previewUrl} alt={t("preview.generated.alt")} />
            )}
          </div>

          {toneVisibility.length > 0 && (
            <div>
              {toneVisibility.map((visible, tone) => (
                <label key={tone}>
                  <input
                    type="checkbox"
                    checked={visible}
                    onChange={(event) =>
                      setToneVisibility((current) =>
                        current.map((v, i) =>
                          i === tone ? event.target.checked : v,
                        ),
                      )
                    }
                  />
                  {t("preview.toneLabel", { tone })}
                </label>
              ))}

              <label htmlFor="colorize-by-layer">
                {t("preview.colorizeByLayer.label")}
              </label>
              <input
                id="colorize-by-layer"
                type="checkbox"
                checked={colorizeByLayer}
                onChange={(event) => setColorizeByLayer(event.target.checked)}
              />
            </div>
          )}
        </section>
      )}

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
