#!/usr/bin/env node
// Validates the locale resources required by the auto-ci skill's
// Standard 5: all required locales exist, all share the same key set,
// none has duplicate keys, and all parse as valid JSON.
//
// English-only tooling script (auto-ci Standard 4) — English error
// messages, even though the resources it validates contain localized
// user-facing strings.

import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const REQUIRED_LOCALES = ["en-US", "pt-BR", "es", "zh-CN"];
const CANONICAL_LOCALE = "en-US";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const localesDir = path.join(scriptDir, "..", "locales");

/** Finds duplicate top-level JSON keys via the raw text, since
 * `JSON.parse` silently keeps only the last occurrence of a repeated
 * key and would hide the problem. Assumes flat, one-key-per-line files
 * as used in this project's locale resources.
 */
function findDuplicateKeys(rawText) {
  const seen = new Map();
  const duplicates = [];
  const keyPattern = /^\s*"((?:[^"\\]|\\.)+)"\s*:/gm;

  for (const match of rawText.matchAll(keyPattern)) {
    const key = match[1];
    seen.set(key, (seen.get(key) ?? 0) + 1);
  }

  for (const [key, count] of seen) {
    if (count > 1) duplicates.push(key);
  }

  return duplicates;
}

function main() {
  const errors = [];
  const keysByLocale = new Map();

  for (const locale of REQUIRED_LOCALES) {
    const filePath = path.join(localesDir, `${locale}.json`);

    if (!existsSync(filePath)) {
      errors.push(`Missing required locale file: locales/${locale}.json`);
      continue;
    }

    const raw = readFileSync(filePath, "utf-8");

    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch (err) {
      errors.push(`locales/${locale}.json is not valid JSON: ${err.message}`);
      continue;
    }

    const duplicates = findDuplicateKeys(raw);
    if (duplicates.length > 0) {
      errors.push(
        `locales/${locale}.json has duplicate keys: ${duplicates.join(", ")}`,
      );
    }

    keysByLocale.set(locale, new Set(Object.keys(parsed)));
  }

  const canonicalKeys = keysByLocale.get(CANONICAL_LOCALE);
  if (canonicalKeys) {
    for (const [locale, keys] of keysByLocale) {
      if (locale === CANONICAL_LOCALE) continue;

      const missing = [...canonicalKeys].filter((k) => !keys.has(k));
      const extra = [...keys].filter((k) => !canonicalKeys.has(k));

      if (missing.length > 0) {
        errors.push(
          `locales/${locale}.json is missing keys present in ${CANONICAL_LOCALE}: ${missing.join(", ")}`,
        );
      }
      if (extra.length > 0) {
        errors.push(
          `locales/${locale}.json has keys not present in ${CANONICAL_LOCALE}: ${extra.join(", ")}`,
        );
      }
    }
  }

  if (errors.length > 0) {
    console.error("Locale validation failed:\n");
    for (const err of errors) console.error(`  - ${err}`);
    process.exit(1);
  }

  console.log(
    `Locale validation passed: ${REQUIRED_LOCALES.length} locales, ${canonicalKeys?.size ?? 0} keys each.`,
  );
}

main();
