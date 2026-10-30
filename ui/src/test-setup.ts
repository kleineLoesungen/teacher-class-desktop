// Läuft vor jeder Testdatei. jsdom ist kein Browser — was fehlt, kommt hier.

import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// jsdom kennt matchMedia nicht. lib/theme.ts fragt damit nach dem Farbschema.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  }),
});

afterEach(() => {
  cleanup();
  clearMocks();
  // `applyTheme` schreibt an <html>, und das überlebt `cleanup()`. Ohne
  // dieses Aufräumen könnte ein Test grün sein, weil der Test davor das
  // Aussehen gesetzt hat — der unangenehmste Fehler, den eine Testreihe
  // haben kann.
  const wurzel = document.documentElement;
  for (const name of Object.keys(wurzel.dataset)) {
    delete wurzel.dataset[name];
  }
  wurzel.removeAttribute("style");
});
