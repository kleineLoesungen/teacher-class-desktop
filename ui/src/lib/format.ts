// Anzeige von Werten, die Rust in maschinenlesbarer Form schickt.
//
// Zeitstempel kommen als RFC-3339-Text in UTC (siehe crates/core/src/timestamp.rs).
// Umgerechnet wird erst hier, für die Anzeige — nie zurück nach Rust.

const dateTime = new Intl.DateTimeFormat("de-DE", {
  day: "2-digit",
  month: "2-digit",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

/** `2026-09-15T06:39:51Z` → `15.09.2026, 08:39` in der Zeitzone des Rechners. */
export function formatDateTime(rfc3339: string): string {
  const date = new Date(rfc3339);
  return Number.isNaN(date.getTime()) ? rfc3339 : dateTime.format(date);
}

/** Erste nicht leere Zeile des Textes, ohne Markdown-Zeichen am Anfang. */
export function excerpt(body: string, max = 80): string {
  const line =
    body
      .split("\n")
      .map((l) => l.replace(/^[#>*\-\s`]+/, "").trim())
      .find((l) => l.length > 0) ?? "";
  return line.length > max ? `${line.slice(0, max - 1)}…` : line;
}
