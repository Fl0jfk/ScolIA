import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import type { StageConvention } from "@/app/lib/stage-types";

const SKIP_ROOT = new Set(["id", "status", "updatedAt"]);

export function roundTripConvention(c: StageConvention): StageConvention {
  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(c as unknown as Record<string, unknown>)) {
    if (SKIP_ROOT.has(k)) continue;
    rest[k] = v;
  }
  const attrs = flattenToAttrs(rest);
  const inflated = inflateFromAttrs(attrs);
  return {
    ...inflated,
    id: c.id,
    status: c.status,
    updatedAt: c.updatedAt,
  } as StageConvention;
}

export const sample: StageConvention = {
  id: "stg_conv_test_index",
  status: "signatures_pending",
  schoolYear: "2025-2026",
  internshipKind: "pfmp",
  createdAt: "2026-03-01T10:00:00.000Z",
  updatedAt: "2026-03-20T12:00:00.000Z",
  student: {
    firstName: "Marion",
    lastName: "BOREL",
    className: "3B",
    level: "3ème",
    email: "parent@example.com",
  },
  company: {
    name: "ACME SAS",
    address: "1 rue Test",
    activity: "Informatique",
    siret: "12345678900012",
    tutorName: "Jean Tutor",
    tutorEmail: "tutor@acme.test",
    tutorPhone: "0600000000",
  },
  teacherReferent: {
    name: "Prof Ref",
    email: "prof@school.test",
  },
  schedule: {
    mode: "uniform_week",
    periodStart: "2026-04-01",
    periodEnd: "2026-04-15",
    days: [
      {
        weekday: 1,
        hasLunchBreak: true,
        morningStart: "08:30",
        morningEnd: "12:00",
        afternoonStart: "13:30",
        afternoonEnd: "17:00",
      },
    ],
    presenceWeekdays: [1, 2, 3, 4, 5],
  },
  signatures: [],
  history: [],
  createdBy: { role: "eleve", name: "Marion BOREL" },
};
