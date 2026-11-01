-- Abschluss-Satz: eine freie Nachricht je Schüler und Prüfung, neben der Note
-- ("Tolle Leistung! Weiter so"). Leer = keine; dann gibt es auch keine Zeile.
CREATE TABLE exam_remarks (
    exam_id    TEXT NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    student_id TEXT NOT NULL REFERENCES students (id) ON DELETE CASCADE,
    text       TEXT NOT NULL,
    PRIMARY KEY (exam_id, student_id)
) STRICT;
