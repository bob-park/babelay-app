import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";
import type { Lang, UiLang } from "./types";

/** 지원 언어의 로케일 키. 그 밖의 코드는 Intl 로, 그것도 안 되면 대문자 코드로 보여준다. */
export const LANG_KEY = { ko: "general.langKo", en: "general.langEn", ja: "general.langJa" } as const;

export function langName(code: string, t: (key: string) => string, uiLang: string): string {
  const key = (LANG_KEY as Record<string, string>)[code];
  if (key) return t(key);
  try {
    return new Intl.DisplayNames([uiLang], { type: "language" }).of(code) ?? code.toUpperCase();
  } catch {
    return code.toUpperCase();
  }
}

export function resolveLang(pref: UiLang, navigatorLang: string): Lang {
  const code = pref === "system" ? navigatorLang : pref;
  const primary = code.split(/[-_]/)[0]?.toLowerCase();
  return primary === "ko" || primary === "ja" ? primary : "en";
}

export async function initI18n(lang: Lang) {
  if (i18next.isInitialized) return i18next.changeLanguage(lang);
  return i18next.use(initReactI18next).init({
    lng: lang,
    fallbackLng: "en",
    resources: { ko: { translation: ko }, en: { translation: en }, ja: { translation: ja } },
    interpolation: { escapeValue: false },
  });
}
