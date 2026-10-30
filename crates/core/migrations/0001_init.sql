-- Beispielressource. Zum Kopieren gedacht: eine Tabelle, ein Modul in
-- src/note.rs, ein Unterbefehl in der CLI, eine Ansicht in der Oberfläche.
--
-- STRICT erzwingt die Spaltentypen. Ohne STRICT nimmt SQLite in einer
-- TEXT-Spalte klaglos eine Zahl entgegen.
CREATE TABLE notes (
    id         TEXT NOT NULL PRIMARY KEY,
    title      TEXT NOT NULL,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX notes_updated_at ON notes (updated_at DESC);
