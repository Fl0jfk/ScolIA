import assert from "node:assert/strict";
import {
  notificationCountForModule,
  notificationCountForPillar,
  type DashboardNotification,
} from "@/app/lib/dashboard-signals";

const notifs: DashboardNotification[] = [
  {
    id: "travels-dir",
    moduleId: "travels",
    label: "Sorties",
    count: 3,
    href: "/travels",
  },
  {
    id: "travels-messages",
    moduleId: "travels",
    label: "Messages",
    count: 3,
    href: "/travels",
  },
  {
    id: "stages-sign",
    moduleId: "stages",
    label: "Stages",
    count: 2,
    href: "/stages",
  },
  {
    id: "photocopies",
    moduleId: "photocopies-couleur",
    label: "Photocopies",
    count: 1,
    href: "/photocopies",
  },
];

assert.equal(
  notificationCountForModule("travels", notifs),
  3,
  "même module : max, pas 3+3",
);

assert.equal(
  notificationCountForPillar("services", notifs),
  4,
  "services = max(travels)=3 + photocopies=1",
);

assert.equal(
  notificationCountForPillar("administratif", notifs),
  2,
  "administratif = stages seulement",
);

console.log("dashboard-signals-pillar-count: ok");
