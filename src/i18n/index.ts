import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";

import enUS from "../../locales/en-US.json";
import es from "../../locales/es.json";
import ptBR from "../../locales/pt-BR.json";
import zhCN from "../../locales/zh-CN.json";

export const SUPPORTED_LOCALES = ["en-US", "pt-BR", "es", "zh-CN"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

/**
 * Only en-US and pt-BR are surfaced in the UI language switcher for now.
 * `es`/`zh-CN` resources are kept translation-complete (validated by
 * `scripts/validate-locales.mjs`) and reachable via OS-locale detection,
 * but not yet marketed as officially supported/QA'd.
 * See docs/adr/0005-i18n-locale-scope.md.
 */
export const OFFICIALLY_SUPPORTED_LOCALES: SupportedLocale[] = [
  "en-US",
  "pt-BR",
];

// Exported so tests (and anything else that needs it) can await
// initialization instead of racing it.
export const i18nReady = i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      "en-US": { translation: enUS },
      "pt-BR": { translation: ptBR },
      es: { translation: es },
      "zh-CN": { translation: zhCN },
    },
    // Fallback order (Standard 5, rule 3): exact locale -> base language
    // -> en-US.
    fallbackLng: {
      pt: ["pt-BR", "en-US"],
      default: ["en-US"],
    },
    supportedLngs: [...SUPPORTED_LOCALES],
    interpolation: { escapeValue: false },
  });

export default i18n;
