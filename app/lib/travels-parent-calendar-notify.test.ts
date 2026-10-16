import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { computeParentCalendarNotifyHash } from "@/app/lib/travels-parent-calendar";
import {
  mergeNotifiedParentEmails,
  parentCalendarNotifiedEmailsForHash,
  remainingParentEmailsToNotify,
} from "@/app/lib/travels-parent-email-batch";
import type { TravelsParentCalendar } from "@/app/lib/travels-types";

const tripData = {
  startDate: "2026-05-10",
  endDate: "2026-05-12",
  startTime: "08:00",
  endTime: "18:00",
};

const calendarA: TravelsParentCalendar = {
  includeTripSpan: true,
  points: [
    {
      id: "pt_a1",
      kind: "depot",
      date: "2026-05-10",
      time: "07:30",
      place: "Cour",
    },
    {
      id: "pt_a2",
      kind: "recuperation",
      date: "2026-05-12",
      time: "19:00",
      place: "Cour",
    },
  ],
};

const calendarB: TravelsParentCalendar = {
  ...calendarA,
  points: [
    calendarA.points[0],
    {
      ...calendarA.points[1],
      id: "pt_b2",
      time: "18:30",
    },
  ],
};

describe("parent calendar notify hash", () => {
  it("réinitialise la liste notifiée quand le calendrier change", () => {
    const hashA = computeParentCalendarNotifyHash(tripData, calendarA);
    const hashB = computeParentCalendarNotifyHash(tripData, calendarB);
    assert.notEqual(hashA, hashB);

    const stored = ["parent1@x.fr", "parent2@x.fr"];
    const effective = parentCalendarNotifiedEmailsForHash(stored, hashA, hashB);
    assert.deepEqual(effective, []);

    const pending = remainingParentEmailsToNotify(
      ["parent1@x.fr", "parent2@x.fr", "parent3@x.fr"],
      effective,
    );
    assert.deepEqual(pending, ["parent1@x.fr", "parent2@x.fr", "parent3@x.fr"]);
  });

  it("conserve le dédoublonnage pour un calendrier identique", () => {
    const hash = computeParentCalendarNotifyHash(tripData, calendarA);
    const stored = mergeNotifiedParentEmails(undefined, ["parent1@x.fr", "parent2@x.fr"]);
    const effective = parentCalendarNotifiedEmailsForHash(stored, hash, hash);
    const pending = remainingParentEmailsToNotify(
      ["parent1@x.fr", "parent2@x.fr", "parent3@x.fr"],
      effective,
    );
    assert.deepEqual(pending, ["parent3@x.fr"]);
  });
});
