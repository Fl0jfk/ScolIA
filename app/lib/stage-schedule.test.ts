import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  defaultDayHoursTemplate,
  formatDaySlotTimeParts,
  normalizeStageSchedule,
  validateStageSchedule,
} from "@/app/lib/stage-schedule";
import type { StageSchedule } from "@/app/lib/stage-types";

function baseSchedule(days: StageSchedule["days"]): StageSchedule {
  return {
    mode: "uniform_week",
    periodStart: "2026-03-09",
    periodEnd: "2026-03-13",
    presenceWeekdays: [1, 2, 3, 4, 5],
    days,
  };
}

describe("validateStageSchedule — demi-journées optionnelles", () => {
  it("accepte matin seulement (pas d'après-midi)", () => {
    const schedule = baseSchedule(
      ([1, 2, 3, 4, 5] as const).map((weekday) => ({
        weekday,
        hasLunchBreak: true,
        morningStart: "08:00",
        morningEnd: "12:00",
        afternoonStart: null,
        afternoonEnd: null,
      })),
    );
    assert.equal(validateStageSchedule(schedule), null);
  });

  it("accepte après-midi seulement (pas de matin)", () => {
    const schedule = baseSchedule(
      ([1, 2, 3, 4, 5] as const).map((weekday) => ({
        weekday,
        hasLunchBreak: true,
        morningStart: null,
        morningEnd: null,
        afternoonStart: "13:00",
        afternoonEnd: "17:00",
      })),
    );
    assert.equal(validateStageSchedule(schedule), null);
  });

  it("refuse aucune demi-journée", () => {
    const schedule = baseSchedule(
      ([1, 2, 3, 4, 5] as const).map((weekday) => ({
        weekday,
        hasLunchBreak: true,
        morningStart: null,
        morningEnd: null,
        afternoonStart: null,
        afternoonEnd: null,
      })),
    );
    assert.match(
      validateStageSchedule(schedule) || "",
      /au moins une demi-journée/i,
    );
  });

  it("accepte matin + après-midi (comportement inchangé)", () => {
    const template = defaultDayHoursTemplate();
    const schedule = baseSchedule(
      ([1, 2, 3, 4, 5] as const).map((weekday) => ({ ...template, weekday })),
    );
    assert.equal(validateStageSchedule(schedule), null);
  });
});

describe("formatDaySlotTimeParts — absent", () => {
  it("affiche Absent pour une demi-journée vide", () => {
    const parts = formatDaySlotTimeParts({
      hasLunchBreak: true,
      morningStart: "08:00",
      morningEnd: "12:00",
      afternoonStart: null,
      afternoonEnd: null,
    });
    assert.equal(parts.morning, "08:00–12:00");
    assert.equal(parts.afternoon, "Absent");
  });
});

describe("normalizeStageSchedule — créneaux partiels", () => {
  it("efface un matin incomplet", () => {
    const normalized = normalizeStageSchedule({
      mode: "uniform_week",
      periodStart: "2026-03-09",
      periodEnd: "2026-03-13",
      presenceWeekdays: [1],
      days: [
        {
          weekday: 1,
          hasLunchBreak: true,
          morningStart: "08:00",
          morningEnd: null,
          afternoonStart: "13:00",
          afternoonEnd: "16:00",
        },
      ],
    });
    assert.equal(normalized.days[0]?.morningStart, null);
    assert.equal(normalized.days[0]?.morningEnd, null);
    assert.equal(normalized.days[0]?.afternoonStart, "13:00");
  });
});
