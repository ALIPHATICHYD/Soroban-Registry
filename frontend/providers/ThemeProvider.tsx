"use client";

import React, { useEffect, useCallback, useState } from "react";
import { Theme, ThemeContext } from "../hooks/useTheme";
import { useAppDispatch, useAppSelector } from "@/store/hooks";
import { setTheme as setThemeAction } from "@/store/slices/themeSlice";

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const dispatch = useAppDispatch();
  const theme = useAppSelector((s) => s.theme.value) as Theme;
  const [resolvedTheme, setResolvedTheme] = useState<"light" | "dark">("light");

  const getSystemTheme = useCallback((): "light" | "dark" => {
    if (
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
    ) {
      return "dark";
    }
    return "light";
  }, []);

  const setTheme = useCallback(
    (newTheme: Theme) => {
      dispatch(setThemeAction(newTheme));
    },
    [dispatch],
  );

  useEffect(() => {
    // Apply theme class to document based on redux state
    const root = window.document.documentElement;
    const isDark =
      theme === "dark" || (theme === "system" && getSystemTheme() === "dark");
    queueMicrotask(() => setResolvedTheme(isDark ? "dark" : "light"));

    if (isDark) root.classList.add("dark");
    else root.classList.remove("dark");
  }, [theme, getSystemTheme]);

  // Listen for system theme changes only when 'system' selected
  useEffect(() => {
    if (theme !== "system") return;
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => {
      const isDark = mediaQuery.matches;
      setResolvedTheme(isDark ? "dark" : "light");
      const root = window.document.documentElement;
      if (isDark) root.classList.add("dark");
      else root.classList.remove("dark");
    };
    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [theme]);

  return (
    <ThemeContext.Provider value={{ theme, setTheme, resolvedTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}
