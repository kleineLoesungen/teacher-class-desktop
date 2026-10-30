// Markdown → bereinigtes HTML.
//
// Die einzige Stelle, an der HTML aus Benutzertext entsteht. Der Weg ist
// immer: marked erzeugt HTML, DOMPurify entfernt alles Ausführbare, erst dann
// kommt es ins Dokument. Ein `<img onerror=…>` in einer Notiz darf nichts tun
// können — im Webview hätte ein Skript Zugriff auf alle IPC-Befehle.
//
// crates/core/tests/layering.rs verbietet `innerHTML` in den Komponenten und
// `dangerouslySetInnerHTML` überall. Wer HTML anzeigen will, geht hier durch.

import DOMPurify from "dompurify";
import { Marked } from "marked";

// Eine eigene Instanz statt des globalen `marked`: Einstellungen, die ein
// anderes Modul am globalen Objekt vornimmt, wirken hier nicht mit.
const marked = new Marked({ gfm: true, breaks: false });

export function markdownToHtml(source: string): string {
  const raw = marked.parse(source, { async: false });
  return DOMPurify.sanitize(raw, { USE_PROFILES: { html: true } });
}

/** Schreibt Markdown als bereinigtes HTML in ein Element. */
export function renderMarkdownInto(element: HTMLElement, source: string): void {
  element.innerHTML = markdownToHtml(source);
}

/**
 * Welche Verweise nach draußen dürfen.
 *
 * Alles andere — relative Pfade, `javascript:`, `file:` — wird verschluckt.
 * Ein Klick darf das Fenster der Anwendung nie wegnavigieren.
 */
export function externalHref(href: string | null): string | undefined {
  if (!href) return undefined;
  return /^(https?:|mailto:)/i.test(href) ? href : undefined;
}
