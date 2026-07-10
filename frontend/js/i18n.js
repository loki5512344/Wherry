// Minimal i18n layer. UI code is written entirely in English strings pulled
// through t(), never inline literals, so adding a language later is just:
//   1. add js/i18n/xx.js (only needs to cover the keys it wants to change)
//   2. register it in `dictionaries` below
//   3. call setLocale('xx') — every already-rendered t() call site updates
//      on next re-render, nothing else in the app needs to change.
import en from "./i18n/en.js";
import ru from "./i18n/ru.js";
import es from "./i18n/es.js";
import fr from "./i18n/fr.js";
import de from "./i18n/de.js";
import it from "./i18n/it.js";
import pt from "./i18n/pt.js";
import pl from "./i18n/pl.js";
import zh from "./i18n/zh.js";
import ja from "./i18n/ja.js";
import ko from "./i18n/ko.js";
import tr from "./i18n/tr.js";

const dictionaries = { en, ru, es, fr, de, it, pt, pl, zh, ja, ko, tr };

let currentLocale = "en";
const listeners = new Set();

function lookup(dict, path) {
  return path.split(".").reduce((o, k) => (o && o[k] !== undefined ? o[k] : undefined), dict);
}

export function availableLocales() {
  return Object.keys(dictionaries);
}

export function getLocale() {
  return currentLocale;
}

export function setLocale(locale) {
  if (!dictionaries[locale] || locale === currentLocale) return;
  currentLocale = locale;
  listeners.forEach((fn) => fn(locale));
}

export function onLocaleChange(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function t(key, vars) {
  let str = lookup(dictionaries[currentLocale], key);
  if (str === undefined) str = lookup(dictionaries.en, key);
  if (str === undefined) return key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      str = str.replaceAll(`{${k}}`, String(v));
    }
  }
  return str;
}
