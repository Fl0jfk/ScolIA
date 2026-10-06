import assert from "node:assert/strict";
import {
  canUseTravelInternalThread,
  countUnreadTravelMessages,
  viewerIsTravelThreadAudience,
} from "@/app/lib/travels-thread-unread";
import type { Establishment } from "@/app/lib/app-config-schemas";

const establishments: Establishment[] = [
  { id: "ecole", label: "École", kind: "ecole", active: true },
  { id: "college", label: "Collège", kind: "college", active: true },
  { id: "lycee", label: "Lycée", kind: "lycee", active: true },
];

const tripCollege = {
  ownerId: "prof-1",
  ownerName: "Mme Dupont",
  data: { etablissement: "Collège" },
};

const creator = {
  user: { id: "prof-1", fullName: "Mme Dupont" },
  roles: ["professeur"],
};

const collegeDir = {
  user: { id: "dir-col", fullName: "Direction collège" },
  roles: ["direction_college"],
};

const lyceeDir = {
  user: { id: "dir-lyc", fullName: "Direction lycée" },
  roles: ["direction_lycee"],
};

const compta = {
  user: { id: "cpta-1", fullName: "Compta" },
  roles: ["comptabilite"],
};

const otherProf = {
  user: { id: "prof-2", fullName: "M. Martin" },
  roles: ["professeur"],
};

assert.equal(
  viewerIsTravelThreadAudience(tripCollege, creator, establishments),
  true,
  "créateur notifié",
);
assert.equal(
  viewerIsTravelThreadAudience(tripCollege, collegeDir, establishments),
  true,
  "direction collège notifiée sur un séjour collège",
);
assert.equal(
  viewerIsTravelThreadAudience(tripCollege, lyceeDir, establishments),
  false,
  "direction lycée non notifiée sur un séjour collège",
);
assert.equal(
  viewerIsTravelThreadAudience(tripCollege, compta, establishments),
  true,
  "compta notifiée",
);
assert.equal(
  viewerIsTravelThreadAudience(tripCollege, otherProf, establishments),
  false,
  "autre prof non notifié",
);

assert.equal(
  canUseTravelInternalThread(tripCollege, lyceeDir, establishments),
  true,
  "une autre direction peut quand même ouvrir le fil",
);

const groupeTrip = {
  ownerId: "prof-1",
  data: { etablissement: "Groupe Scolaire" },
};
assert.equal(
  viewerIsTravelThreadAudience(groupeTrip, lyceeDir, establishments),
  true,
  "Groupe scolaire : toutes les directions",
);

const lastRead = new Date("2026-10-06T10:00:00.000Z");
const unread = countUnreadTravelMessages({
  messages: [
    { authorUserId: "cpta-1", date: "2026-10-06T09:00:00.000Z" },
    { authorUserId: "prof-1", date: "2026-10-06T11:00:00.000Z" },
    { authorUserId: "dir-col", date: "2026-10-06T11:30:00.000Z" },
  ],
  lastReadAt: lastRead,
  viewerUserIds: ["cpta-1"],
});
assert.equal(unread, 2, "l’expéditeur ne compte pas ses messages ; les plus récents comptent");

console.log("travels-thread-unread.test.ts ok");
