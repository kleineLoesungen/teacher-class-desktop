/// <reference types="vitest/config" />
// Baut die Oberfläche aus ui/ nach ui/dist. Tauri lädt im Entwicklungsbetrieb
// den Vite-Server (devUrl) und im fertigen Bündel ui/dist (frontendDist) —
// beides steht in crates/desktop/tauri.conf.json.
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  root: "ui",
  plugins: [react()],

  // Tauri erwartet den Server auf genau diesem Port. Ist er belegt, soll Vite
  // abbrechen statt auf einen anderen auszuweichen — sonst lädt das Fenster
  // eine leere Seite.
  server: {
    port: 1420,
    strictPort: true,
    // Rust-Änderungen baut Tauri selbst neu. Ohne diesen Eintrag lädt Vite
    // bei jedem `cargo build` die Seite neu.
    watch: { ignored: ["**/crates/**", "**/target/**"] },
  },
  clearScreen: false,

  build: {
    outDir: "dist",
    emptyOutDir: true,
    // Das Webview ist bekannt: WebKit auf macOS und Linux. Kein Grund, für
    // alte Browser zurückzuübersetzen.
    target: "safari16",
    // Mermaid ist groß und wird erst geladen, wenn eine Notiz ein Diagramm
    // enthält (siehe ui/src/lib/mermaid.ts). Die Warnung über große Stücke
    // betrifft genau diese nachgeladenen Teile.
    chunkSizeWarningLimit: 4000,
  },

  test: {
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
