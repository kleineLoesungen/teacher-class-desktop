-- Bewertung in der App: Beschreibung je Prüfkriterium, Begründungen je Prüfung
-- und die erreichten Punkte je Schüler.

-- Optionale Beschreibung eines Prüfkriteriums. Leer = keine.
ALTER TABLE exam_items ADD COLUMN description TEXT NOT NULL DEFAULT '';

-- Ob zu jedem Prüfkriterium eine Begründung gepflegt wird. Das ändert auch den
-- Excel-Export (eine Bewertungskachel je Schüler statt einer Matrix).
ALTER TABLE exams ADD COLUMN with_notes INTEGER NOT NULL DEFAULT 0;

-- Erreichte Punkte und Begründung je Schüler und Prüfkriterium.
--
-- Das Prüfkriterium ist über seine Stelle (Gruppe, Punkt — nullbasiert)
-- bezeichnet, nicht über eine Kennung: Eine Prüfung wird beim Ändern als
-- Ganzes neu geschrieben, die Kennungen der Punkte wechseln dabei. Die
-- Stellen bleiben, solange das Schema seine Form behält.
--
-- `points_half` ist NULL, solange nichts eingetragen ist (nicht dasselbe wie 0).
CREATE TABLE exam_scores (
    exam_id     TEXT NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    student_id  TEXT NOT NULL REFERENCES students (id) ON DELETE CASCADE,
    group_pos   INTEGER NOT NULL,
    item_pos    INTEGER NOT NULL,
    points_half INTEGER,
    note        TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (exam_id, student_id, group_pos, item_pos)
) STRICT;

CREATE INDEX exam_scores_student ON exam_scores (student_id);
