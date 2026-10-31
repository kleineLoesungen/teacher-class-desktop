// Der Entwurf einer Prüfung in der Oberfläche.
//
// Punkte stehen im Entwurf als Text, damit „2,5“ und halbfertige Eingaben wie
// „2,“ tippbar bleiben. Beim Abschicken wird der Text zur Zahl. Ob die Zahl
// gültig ist (größer 0, ganze oder halbe Punkte, …), entscheidet Rust.

import type { Exam, ExamInput } from "../bindings";

export type ItemDraft = { name: string; max: string };
export type GroupDraft = { name: string; items: ItemDraft[] };
export type GradeDraft = { name: string; from: string };

export type ExamDraft = {
  name: string;
  class_ids: string[];
  groups: GroupDraft[];
  grades: GradeDraft[];
};

/** Die üblichen Noten 1 bis 6. Nur die 6 beginnt von vornherein bei 0. */
const DEFAULT_GRADES = ["1", "2", "3", "4", "5", "6"];

export function emptyExamDraft(): ExamDraft {
  return {
    name: "",
    class_ids: [],
    groups: [{ name: "", items: [{ name: "", max: "" }] }],
    grades: DEFAULT_GRADES.map((name) => ({ name, from: name === "6" ? "0" : "" })),
  };
}

/** Punkte für die Anzeige: `2.5` → `2,5`. */
export function formatPoints(points: number): string {
  return String(points).replace(".", ",");
}

export function draftFromExam(exam: Exam): ExamDraft {
  return {
    name: exam.name,
    class_ids: [...exam.class_ids],
    groups: exam.groups.map((g) => ({
      name: g.name,
      items: g.items.map((i) => ({ name: i.name, max: formatPoints(i.max_points) })),
    })),
    // Rust liefert die Matrix aufsteigend nach „ab“; die Reihenfolge bleibt.
    grades: exam.grades.map((g) => ({ name: g.name, from: formatPoints(g.from_points) })),
  };
}

/**
 * Text → Zahl. Was keine Zahl ist, wird -1: Rust lehnt das mit einer Meldung
 * am Feld ab. (`NaN` ginge als `null` über die Grenze und scheiterte dort an
 * der Deserialisierung, ohne dass jemand erführe, welches Feld gemeint ist.)
 */
export function parsePoints(text: string): number {
  const value = Number(text.trim().replace(",", "."));
  return text.trim() === "" || !Number.isFinite(value) ? -1 : value;
}

export function toExamInput(draft: ExamDraft): ExamInput {
  return {
    name: draft.name,
    class_ids: draft.class_ids,
    groups: draft.groups.map((g) => ({
      name: g.name,
      items: g.items.map((i) => ({ name: i.name, max_points: parsePoints(i.max) })),
    })),
    grades: draft.grades.map((g) => ({ name: g.name, from_points: parsePoints(g.from) })),
  };
}

/** Wurde der Entwurf gegenüber dem gespeicherten Stand geändert? */
export function sameDraft(a: ExamDraft, b: ExamDraft): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
