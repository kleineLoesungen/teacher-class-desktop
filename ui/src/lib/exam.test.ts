import { describe, expect, it } from "vitest";
import type { Exam } from "../bindings";
import {
  draftFromExam,
  emptyExamDraft,
  formatPoints,
  parsePoints,
  sameDraft,
  toExamInput,
} from "./exam";

describe("Prüfungsentwurf", () => {
  it("beginnt mit den Noten 1 bis 6 und der 6 bei 0 Punkten", () => {
    const draft = emptyExamDraft();
    expect(draft.grades.map((g) => g.name)).toEqual(["1", "2", "3", "4", "5", "6"]);
    expect(draft.grades.filter((g) => g.from === "0").map((g) => g.name)).toEqual(["6"]);
  });

  it("liest Komma und Punkt als Dezimaltrenner", () => {
    expect(parsePoints("2,5")).toBe(2.5);
    expect(parsePoints(" 2.5 ")).toBe(2.5);
    expect(parsePoints("13")).toBe(13);
  });

  it("macht aus Unlesbarem -1, damit Rust es am Feld ablehnt", () => {
    for (const text of ["", "  ", "abc", "2,5,1"]) {
      expect(parsePoints(text)).toBe(-1);
    }
  });

  it("zeigt Punkte mit Komma", () => {
    expect(formatPoints(2.5)).toBe("2,5");
    expect(formatPoints(4)).toBe("4");
  });

  it("macht aus dem Gespeicherten und zurück denselben Entwurf", () => {
    const exam: Exam = {
      id: "e",
      name: "Test",
      class_ids: ["a"],
      groups: [
        {
          name: "G",
          items: [{ name: "a", description: "Rechenweg", max_points: 2.5 }],
          max_points: 2.5,
        },
      ],
      grades: [{ name: "1", from_points: 0, to_points: 2.5 }],
      max_points: 2.5,
      with_notes: true,
      created_at: "2026-09-15T06:39:51Z",
    };
    const draft = draftFromExam(exam);
    expect(draft.groups[0]?.items[0]?.max).toBe("2,5");
    expect(draft.groups[0]?.items[0]?.description).toBe("Rechenweg");
    expect(toExamInput(draft)).toEqual({
      name: "Test",
      class_ids: ["a"],
      groups: [{ name: "G", items: [{ name: "a", description: "Rechenweg", max_points: 2.5 }] }],
      grades: [{ name: "1", from_points: 0 }],
      with_notes: true,
    });
    expect(sameDraft(draft, draftFromExam(exam))).toBe(true);
    expect(sameDraft(draft, { ...draft, name: "anders" })).toBe(false);
  });
});
