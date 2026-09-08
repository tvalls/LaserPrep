import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { OFFICIALLY_SUPPORTED_LOCALES } from "./i18n";

const MIN_TONE_COUNT = 2;
const MAX_TONE_COUNT = 16;
const DEFAULT_TONE_COUNT = 5;
const DEFAULT_DPI = 96;

type Status =
  | { kind: "idle" }
  | { kind: "converting" }
  | { kind: "converted" }
  | { kind: "error"; message: string };

export default function App() {
  const { t, i18n } = useTranslation();
  const [toneCount, setToneCount] = useState(DEFAULT_TONE_COUNT);
  const [dpi, setDpi] = useState(DEFAULT_DPI);
  const [svg, setSvg] = useState<string | null>(null);
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
      const result = await invoke<string>("convert_image_file", {
        path,
        dpi,
        toneCount,
      });
      setSvg(result);
      setStatus({ kind: "converted" });
    } catch (error) {
      setSvg(null);
      setStatus({ kind: "error", message: String(error) });
    }
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
      </div>

      <button type="button" onClick={() => void handleImport()}>
        {t("import.button")}
      </button>
      <button type="button" onClick={() => void handleExport()} disabled={!svg}>
        {t("export.button")}
      </button>

      <p role="status">{statusText}</p>

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
