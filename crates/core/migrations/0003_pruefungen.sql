-- Prüfungen mit Prüfungsschema (Gruppen und Punkte) und Bewertungsmatrix.
--
-- Punkte werden in Halbpunkten als ganze Zahlen gespeichert (`*_half`):
-- 2,5 Punkte sind 5. So gibt es keine Rundungsfehler, und Halbpunkte sind die
-- kleinste erlaubte Einheit.
CREATE TABLE exams (
    id         TEXT NOT NULL PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

-- Für welche Klassen die Prüfung gilt.
CREATE TABLE exam_classes (
    exam_id  TEXT NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    class_id TEXT NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    PRIMARY KEY (exam_id, class_id)
) STRICT;

CREATE TABLE exam_groups (
    id       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    exam_id  TEXT NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name     TEXT NOT NULL
) STRICT;

CREATE INDEX exam_groups_exam ON exam_groups (exam_id);

CREATE TABLE exam_items (
    id       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL REFERENCES exam_groups (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name     TEXT NOT NULL,
    max_half INTEGER NOT NULL
) STRICT;

CREATE INDEX exam_items_group ON exam_items (group_id);

-- Bewertungsmatrix: nur „ab“. Das „bis“ errechnet sich aus der nächsten Note.
CREATE TABLE exam_grades (
    exam_id   TEXT NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    name      TEXT NOT NULL,
    from_half INTEGER NOT NULL,
    PRIMARY KEY (exam_id, from_half)
) STRICT;
