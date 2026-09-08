import { useTranslation } from "react-i18next";

import { OFFICIALLY_SUPPORTED_LOCALES } from "./i18n";

export default function App() {
  const { t, i18n } = useTranslation();

  return (
    <main>
      <h1>{t("app.title")}</h1>
      <p>{t("app.tagline")}</p>
      <p role="status">{t("app.status.phase0")}</p>

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
