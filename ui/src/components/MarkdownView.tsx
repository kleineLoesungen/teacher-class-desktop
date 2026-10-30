// Zeigt Markdown mit Mermaid-Diagrammen an.
//
// Zeichnet nur. Ein Klick auf einen Verweis wird nach oben gemeldet —
// geöffnet wird er in App.tsx über api.openExternal.

import { type MouseEvent, useEffect, useRef } from "react";
import { externalHref, renderMarkdownInto } from "../lib/markdown";
import { renderDiagrams } from "../lib/mermaid";

type Props = {
  source: string;
  dark: boolean;
  onOpenLink: (url: string) => void;
  /** Wartezeit nach dem letzten Tastendruck. In Tests 0. */
  delayMs?: number;
};

export function MarkdownView({ source, dark, onOpenLink, delayMs = 150 }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    let current = true;
    const timer = window.setTimeout(() => {
      renderMarkdownInto(element, source);
      void renderDiagrams(element, { dark, isCurrent: () => current });
    }, delayMs);

    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [source, dark, delayMs]);

  // Ein Verweis im Webview würde das Fenster der Anwendung wegnavigieren.
  // Deshalb wird jeder Klick abgefangen und nur nach draußen weitergegeben.
  function onClick(event: MouseEvent<HTMLDivElement>) {
    const anchor = (event.target as HTMLElement).closest("a");
    if (!anchor) return;
    event.preventDefault();
    const url = externalHref(anchor.getAttribute("href"));
    if (url) onOpenLink(url);
  }

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents lint/a11y/noStaticElementInteractions: fängt nur Klicks auf die Verweise darin ab, die selbst per Tastatur erreichbar sind
    <div ref={ref} className="markdown" onClick={onClick} data-testid="markdown" />
  );
}
