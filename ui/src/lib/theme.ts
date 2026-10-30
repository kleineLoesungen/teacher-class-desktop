// Einstellungen aus Rust → Aussehen im Dokument.
//
// Farben selbst stehen nur in theme.css. Hier wird nur umgeschaltet: welches
// Farbschema, welche Akzentfarbe, und ob die Schrift auf dem Akzent hell oder
// dunkel sein muss.

import { useEffect, useState } from "react";
import type { Config, Theme } from "../bindings";

/**
 * Ist die Akzentfarbe hell? Dann braucht sie dunkle Schrift.
 *
 * Gewichtet nach wahrgenommener Helligkeit: Grün trägt am meisten bei, Blau
 * am wenigsten. Ohne diese Rechnung wird weiße Schrift auf `#ffdd00` unlesbar.
 */
export function isLightColor(hex: string): boolean {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!match) return false;
  const [r, g, b] = match.slice(1).map((part) => Number.parseInt(part, 16)) as [
    number,
    number,
    number,
  ];
  return 0.299 * r + 0.587 * g + 0.114 * b > 140;
}

/** Setzt Farbschema und Akzent am Wurzelelement. theme.css tut den Rest. */
export function applyTheme(config: Config, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = config.theme;
  root.dataset.accentLight = String(isLightColor(config.accent));
  root.style.setProperty("--accent", config.accent);
}

const DARK_QUERY = "(prefers-color-scheme: dark)";

/** Wird gerade dunkel gezeichnet? Folgt bei `system` dem Betriebssystem. */
export function useIsDark(theme: Theme): boolean {
  const [systemDark, setSystemDark] = useState(() => window.matchMedia(DARK_QUERY).matches);

  useEffect(() => {
    const query = window.matchMedia(DARK_QUERY);
    const onChange = () => setSystemDark(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  return theme === "dark" || (theme === "system" && systemDark);
}
