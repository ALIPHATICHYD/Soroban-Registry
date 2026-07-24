"use client";

import { useEffect } from "react";
import { useCookies } from "react-cookie";
import { createInstance } from "i18next";
import { initReactI18next, useTranslation as useTranslationOrg } from "react-i18next";
import resourcesToBackend from "i18next-resources-to-backend";
import LanguageDetector from "i18next-browser-languagedetector";

import { fallbackLng, getOptions, languages } from "./settings";

const cookieName = "i18next";

function getI18nextOptions(lng = fallbackLng, ns = "common") {
  return {
    ...getOptions(lng, ns),
    lng,
  };
}

// Initialise i18next client instance once
const i18n = createInstance();

i18n
  .use(initReactI18next)
  .use(LanguageDetector)
  .use(
    resourcesToBackend(
      (language: string, namespace: string) => import(`../../public/locales/${language}/${namespace}.json`)
    )
  )
  .init(getI18nextOptions());

export function useTranslation(lng: string = fallbackLng, ns = "common") {
  const [cookies, setCookie] = useCookies([cookieName]);

  const ret = useTranslationOrg(ns);

  // Ensure i18next language matches the requested lng
  useEffect(() => {
    if (!lng || i18n.resolvedLanguage === lng) return;
    i18n.changeLanguage(lng);
  }, [lng]);

  // Persist language choice to cookie (for next reload).
  // NOTE: `cookies` is intentionally omitted from deps — including it causes
  // an infinite loop because setCookie triggers a cookie state update which
  // re-runs this effect, which calls setCookie again, ad infinitum.
  useEffect(() => {
    if (!lng) return; // guard against empty language values
    if (cookies[cookieName] === lng) return;
    setCookie(cookieName, lng, { path: "/" });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lng, setCookie]);

  return ret;
}

export { languages };
