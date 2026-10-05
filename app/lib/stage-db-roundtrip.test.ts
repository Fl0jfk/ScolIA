import assert from "node:assert/strict";
import test from "node:test";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import type { StageConvention } from "@/app/lib/stage-types";

const SKIP_ROOT = new Set(["id", "status", "updatedAt"]);

function roundTripConvention(c: StageConvention): StageConvention {
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

const sample: StageConvention = {
  id: "stg_conv_test_1",
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
  signatures: [
    {
      id: "sig_dir_1",
      role: "direction",
      label: "Direction de l'établissement scolaire",
      status: "signe",
      signEmail: "dumouchel@example.com",
      signedAt: "2026-03-18T09:00:00.000Z",
      signedBy: "Mme Dumouchel",
      signMethod: "code_confirm",
    },
    {
      id: "sig_dir_2",
      role: "direction",
      label: "Direction de l'établissement scolaire",
      status: "en_attente",
      signEmail: "dumouchel@example.com",
      signToken: "tok_pending_xyz",
    },
  ],
  history: [
    { at: "2026-03-01T10:00:00.000Z", by: "Système", action: "CREATED" },
    {
      at: "2026-03-18T09:00:00.000Z",
      by: "Mme Dumouchel",
      action: "SIGNATURE",
      note: "direction:code_confirm",
    },
  ],
  createdBy: { role: "eleve", name: "Marion BOREL" },
};

test("stage convention EAV round-trip conserve signatures direction", () => {
  const out = roundTripConvention(sample);
  assert.equal(out.id, sample.id);
  assert.equal(out.status, sample.status);
  assert.equal(out.student.lastName, "BOREL");
  assert.equal(out.company.name, "ACME SAS");
  assert.equal(out.signatures.length, 2);
  assert.equal(out.signatures[0]?.status, "signe");
  assert.equal(out.signatures[0]?.signEmail, "dumouchel@example.com");
  assert.equal(out.signatures[1]?.status, "en_attente");
  assert.equal(out.signatures[1]?.signToken, "tok_pending_xyz");
  assert.equal(out.schedule.periodStart, "2026-04-01");
  assert.equal(out.history?.length, 2);
});

test("stage convention round-trip — double slot direction détectable", () => {
  const out = roundTripConvention(sample);
  const dirs = out.signatures.filter((s) => s.role === "direction");
  assert.equal(dirs.length, 2);
  assert.equal(dirs.filter((d) => d.status === "signe").length, 1);
  assert.equal(dirs.filter((d) => d.status === "en_attente").length, 1);
});
