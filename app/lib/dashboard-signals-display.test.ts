import assert from "node:assert/strict";
import test from "node:test";
import {
  dashboardNotificationBadgeProps,
  dashboardPillarBadgeProps,
  hasAnyDashboardSourceFailure,
  isDashboardNotificationUnavailable,
} from "@/app/lib/dashboard-signals-display";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";

const OK = Object.fromEntries(
  ["internat", "rh", "demandes", "absences", "documents", "stages", "others"].map((id) => [
    id,
    "ok" as SignalSourceStatus,
  ]),
) as Record<DashboardSignalSourceId, SignalSourceStatus>;

test("isDashboardNotificationUnavailable — utilise signalSourceId explicite", () => {
  const status = { ...OK, absences: "timeout" as SignalSourceStatus };
  assert.equal(
    isDashboardNotificationUnavailable(
      { id: "x", moduleId: "travels", signalSourceId: "absences" },
      status,
    ),
    true,
  );
  assert.equal(
    isDashboardNotificationUnavailable({ id: "x", moduleId: "travels" }, status),
    false,
  );
});

test("vs-absences — indisponible si source absences en timeout", () => {
  const status = { ...OK, absences: "timeout" as SignalSourceStatus };
  assert.equal(
    isDashboardNotificationUnavailable({ id: "vs-abs-1", moduleId: "vs-absences" }, status),
    true,
  );
  assert.deepEqual(
    dashboardNotificationBadgeProps(2, { id: "vs-abs-1", moduleId: "vs-absences" }, status),
    { unavailable: true, compact: undefined },
  );
});

test("absences source timeout — file absences-pending indisponible (pas masquée)", () => {
  const status = { ...OK, absences: "timeout" as SignalSourceStatus };
  const item = { id: "absences-pending", moduleId: "absences" };
  assert.equal(isDashboardNotificationUnavailable(item, status), true);
  assert.deepEqual(dashboardNotificationBadgeProps(4, item, status), {
    unavailable: true,
    compact: undefined,
  });
});

test("rh source timeout — éléments module absences non affectés", () => {
  const status = { ...OK, rh: "timeout" as SignalSourceStatus };
  const item = { id: "absences-pending", moduleId: "absences" };
  assert.equal(isDashboardNotificationUnavailable(item, status), false);
  assert.deepEqual(dashboardNotificationBadgeProps(4, item, status), { count: 4 });
});

test("rh source timeout — module rh affecté", () => {
  const status = { ...OK, rh: "timeout" as SignalSourceStatus };
  assert.equal(
    isDashboardNotificationUnavailable({ id: "rh-x", moduleId: "rh" }, status),
    true,
  );
});

test("dashboardNotificationBadgeProps — indisponible sans chiffre", () => {
  const status = { ...OK, stages: "error" as SignalSourceStatus };
  assert.deepEqual(
    dashboardNotificationBadgeProps(
      0,
      { id: "stages-pend", moduleId: "stages", signalSourceId: "stages" },
      status,
    ),
    { unavailable: true, compact: undefined },
  );
});

test("dashboardNotificationBadgeProps — null si 0 et source ok", () => {
  assert.equal(
    dashboardNotificationBadgeProps(0, { id: "x", moduleId: "stages", signalSourceId: "stages" }, OK),
    null,
  );
});

test("hasAnyDashboardSourceFailure — source KO sans notification", () => {
  const status = { ...OK, absences: "timeout" as SignalSourceStatus };
  assert.equal(hasAnyDashboardSourceFailure(status), true);
  assert.equal(hasAnyDashboardSourceFailure(OK), false);
});

test("dashboardPillarBadgeProps — partialUnavailable si un signal KO et compteurs OK", () => {
  const status = { ...OK, absences: "timeout" as SignalSourceStatus };
  const badge = dashboardPillarBadgeProps(
    "vie_scolaire",
    [
      {
        id: "vs-abs-1",
        moduleId: "vs-absences",
        label: "Abs",
        count: 2,
        href: "/x",
        detail: "d",
      },
      {
        id: "internat-1",
        moduleId: "internat",
        label: "Internat",
        count: 3,
        href: "/internat",
        detail: "d",
      },
    ],
    [],
    status,
  );
  assert.deepEqual(badge, { count: 3, partialUnavailable: true, compact: undefined });
});

test("dashboardPillarBadgeProps — Comptabilité RH : pas de ×2 via raccourci absences", () => {
  // Régression : notifs (19+4+1=24) + recompte via slides du raccourci (23) → 47.
  const notifs = [
    {
      id: "absences-admin-queue",
      moduleId: "absences",
      label: "Dossiers à traiter",
      count: 19,
      href: "/rh?tab=dashboard&section=absences&view=traitement",
      detail: "19 dossiers",
    },
    {
      id: "absences-pending",
      moduleId: "absences",
      label: "Absences à valider",
      count: 4,
      href: "/rh?tab=dashboard&section=absences&view=a-traiter",
      detail: "4 à valider",
    },
    {
      id: "rh-sst-emargement",
      moduleId: "rh",
      label: "Registre SST",
      count: 1,
      href: "/rh?tab=dashboard&section=sst",
      detail: "Émargement",
    },
  ];
  const shortcuts = [
    {
      id: "absences-queue",
      moduleId: "absences",
      pillarId: "compta_rh" as const,
      slides: [
        { id: "absences-pending", label: "Absences à valider", count: 4 },
        { id: "absences-admin-queue", label: "Dossiers à traiter", count: 19 },
      ],
    },
    {
      id: "rh-sst-registre",
      moduleId: "rh",
      pillarId: "compta_rh" as const,
    },
  ];
  const badge = dashboardPillarBadgeProps("compta_rh", notifs, shortcuts, OK);
  assert.deepEqual(badge, { count: 24 });
});
