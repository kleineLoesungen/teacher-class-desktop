-- Klassen und Schüler ersetzen die Beispielressource „notes".
--
-- 0001 bleibt unverändert (veröffentlichte Migrationen werden nie geändert),
-- die Tabelle wird hier entfernt.
DROP TABLE notes;

CREATE TABLE classes (
    id         TEXT NOT NULL PRIMARY KEY,
    name       TEXT NOT NULL,
    year       INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (name, year)
) STRICT;

CREATE TABLE students (
    id         TEXT NOT NULL PRIMARY KEY,
    class_id   TEXT NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    last_name  TEXT NOT NULL,
    first_name TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX students_class ON students (class_id);
