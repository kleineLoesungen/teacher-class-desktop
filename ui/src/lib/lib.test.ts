import { describe, expect, it } from "vitest";
import { ApiError } from "../api";
import { excerpt, formatDateTime } from "./format";
import { externalHref, markdownToHtml } from "./markdown";
import { isLightColor } from "./theme";

describe("markdown", () => {
  it("rendert gängiges Markdown", () => {
    const html = markdownToHtml("# Titel\n\n**fett** und `code`");
    expect(html).toContain("<h1>Titel</h1>");
    expect(html).toContain("<strong>fett</strong>");
    expect(html).toContain("<code>code</code>");
  });

  it("entfernt ausführbares HTML", () => {
    // Im Webview hätte ein Skript Zugriff auf jeden IPC-Befehl.
    const html = markdownToHtml('<img src="x" onerror="alert(1)"><script>alert(2)</script>');
    expect(html).not.toContain("onerror");
    expect(html).not.toContain("<script");
  });

  it("lässt Mermaid-Blöcke als Code stehen, damit lib/mermaid.ts sie findet", () => {
    const html = markdownToHtml("```mermaid\nflowchart LR\n  A --> B\n```");
    expect(html).toContain('<code class="language-mermaid">');
    expect(html).toContain("A --&gt; B");
  });

  it("lässt nur Verweise nach draußen durch", () => {
    expect(externalHref("https://mermaid.js.org")).toBe("https://mermaid.js.org");
    expect(externalHref("mailto:a@example.org")).toBe("mailto:a@example.org");
    expect(externalHref("javascript:alert(1)")).toBeUndefined();
    expect(externalHref("/relativ")).toBeUndefined();
    expect(externalHref(null)).toBeUndefined();
  });
});

describe("Anzeige", () => {
  it("nimmt die erste Textzeile ohne Markdown-Zeichen", () => {
    expect(excerpt("\n# Überschrift\nText")).toBe("Überschrift");
    expect(excerpt("x".repeat(100), 10)).toBe(`${"x".repeat(9)}…`);
  });

  it("zeigt einen unlesbaren Zeitstempel unverändert statt 'Invalid Date'", () => {
    expect(formatDateTime("kaputt")).toBe("kaputt");
    expect(formatDateTime("2026-09-15T06:39:51Z")).toMatch(/15\.09\.2026/);
  });

  it("wählt dunkle Schrift nur auf heller Akzentfarbe", () => {
    expect(isLightColor("#ffdd00")).toBe(true);
    expect(isLightColor("#3b6ea5")).toBe(false);
    expect(isLightColor("blau")).toBe(false);
  });
});

describe("ApiError", () => {
  it("übernimmt die Fehlerhülle aus Rust", () => {
    const err = ApiError.from({
      code: "validation_failed",
      message: "title: darf nicht leer sein",
      fields: [{ field: "title", message: "darf nicht leer sein" }],
    });
    expect(err.isValidation).toBe(true);
    expect(err.fieldMessage("title")).toBe("darf nicht leer sein");
    expect(err.fieldMessage("body")).toBeUndefined();
  });

  it("macht aus einem Text von Tauri selbst einen Programmfehler", () => {
    // So antwortet Tauri auf einen unbekannten Befehl.
    const err = ApiError.from("command note_craete not found");
    expect(err.code).toBe("internal_error");
    expect(err.isValidation).toBe(false);
  });
});
