"use client";

import { createContext, useContext } from "react";

export type Theme = "light" | "dark" | "system";

interface ThemeContextType {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  resolvedTheme: "light" | "dark";
}

export const ThemeContext = createContext<ThemeContextType | undefined>(
  undefined,
);

export function useTheme() {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    // During SSR prerendering there may be no ThemeProvider in scope.
    // Return a default no-op fallback — real call-sites are 'use client'
    // components and useTheme is generally not called during SSR.
    return {
      theme: "system" as Theme,
      setTheme: () => {},
      resolvedTheme: "light" as const,
    };
  }

  return context;
}
