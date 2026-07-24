"use client";

import { Sun, Moon, Monitor } from "lucide-react";
import { useTheme, Theme } from "@/hooks/useTheme";

export default function ThemeToggle() {
  const { theme, setTheme, resolvedTheme } = useTheme();

  const cycleTheme = () => {
    const themes: Theme[] = ["light", "dark", "system"];
    const currentIndex = themes.indexOf(theme);
    const nextIndex = (currentIndex + 1) % themes.length;
    setTheme(themes[nextIndex]);
  };

  return (
    <button
      onClick={cycleTheme}
      className="p-2 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-all"
      title={`Current theme: ${theme}. Click to change.`}
      aria-label="Toggle theme"
    >
      {theme === "system" ? (
        <Monitor className="w-4 h-4" />
      ) : resolvedTheme === "dark" ? (
        <Moon className="w-4 h-4" />
      ) : (
        <Sun className="w-4 h-4" />
      )}
    </button>
  );
}
