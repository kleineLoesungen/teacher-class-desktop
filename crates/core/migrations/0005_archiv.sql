-- Archiv: Klassen und Prüfungen lassen sich ausblenden, ohne sie zu löschen.
-- 0 = aktiv, 1 = archiviert.
ALTER TABLE classes ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE exams ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
