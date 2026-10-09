import assert from "node:assert/strict";
import {
  canUseTravelInternalThread,
  countUnreadTravelMessages,
  formatTravelMessageAuthorLabel,
  viewerIsDirectionForTravelTrip,
  viewerIsTravelThreadAudience,
} from "@/app/lib/travels-thread-unread";
import type { Establishment } from "@/app/lib/app-config-schemas";

const establishments: Establishment[] = [
  {
    id: "ecole",
    label: "École",
    kind: "ecole",
    active: true,
    roleSlugs: ["direction_ecole"],
    directorExternalUserId: "dir-ecole-id",
  },
  {
    id: "college",
    label: "Collège",
    kind: "college",
    active: true,
    roleSlugs: ["direction_college", "direction collège"],
    directorExternalUserId: "dir-col-id",
  },
  {
    id: "lycee",
    label: "Lycée",
    kind: "lycee",
    active: true,
    roleSlugs: ["direction_lycee"],
    directorExternalUserId: "dir-lyc-id",
  },
];

const tripCollege = {
  ownerId: "prof-1",
  ownerName: "Mme Dupont",
  data: { etablissement: "Collège" },
};

const tripLycee = {
  ownerId: "prof-3",
  data: { etablissement: "Lycée" },
};

const tripEcole = {
  ownerId: "prof-4",
  data: { etablissement: "École" },
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

const ecoleDir = {
  user: { id: "dir-ecole", fullName: "Direction école" },
  roles: ["direction_ecole"],
};

const genericDirection = {
  user: { id: "dir-generic", fullName: "Direction générique" },
  roles: ["direction"],
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
  viewerIsTravelThreadAudience(tripCollege, ecoleDir, establishments),
  false,
  "direction école non notifiée sur un séjour collège",
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
  viewerIsTravelThreadAudience(tripLycee, lyceeDir, establishments),
  true,
  "direction lycée notifiée sur un séjour lycée",
);
assert.equal(
  viewerIsTravelThreadAudience(tripLycee, collegeDir, establishments),
  false,
  "direction collège non notifiée sur un séjour lycée",
);
assert.equal(
  viewerIsTravelThreadAudience(tripEcole, ecoleDir, establishments),
  true,
  "direction école notifiée sur un séjour école",
);
assert.equal(
  viewerIsTravelThreadAudience(tripEcole, lyceeDir, establishments),
  false,
  "direction lycée non notifiée sur un séjour école",
);

assert.equal(
  viewerIsDirectionForTravelTrip(tripCollege, genericDirection, establishments),
  false,
  "rôle générique « direction » ne croise pas tous les sites",
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
  "Groupe scolaire : direction lycée concernée",
);
assert.equal(
  viewerIsTravelThreadAudience(groupeTrip, collegeDir, establishments),
  true,
  "Groupe scolaire : direction collège concernée",
);
assert.equal(
  viewerIsTravelThreadAudience(groupeTrip, ecoleDir, establishments),
  true,
  "Groupe scolaire : direction école concernée",
);
assert.equal(
  viewerIsTravelThreadAudience(groupeTrip, genericDirection, establishments),
  true,
  "Groupe scolaire : rôle direction générique concerné",
);

const collegeDirById = {
  user: { id: "dir-col-id", fullName: "Directrice collège (id)" },
  roles: ["professeur"],
  extraUserIds: ["dir-col-id"],
};
assert.equal(
  viewerIsTravelThreadAudience(tripCollege, collegeDirById, establishments),
  true,
  "directrice collège notifiée via directorExternalUserId",
);
assert.equal(
  viewerIsTravelThreadAudience(tripLycee, collegeDirById, establishments),
  false,
  "directrice collège non notifiée sur un séjour lycée",
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

assert.equal(
  formatTravelMessageAuthorLabel({
    firstName: "Marie",
    lastName: "Dupont",
    name: "Utilisateur",
    email: "compta@example.com",
  }),
  "Marie Dupont",
  "prénom + nom prioritaire sur le libellé générique",
);
assert.equal(
  formatTravelMessageAuthorLabel({
    firstName: null,
    lastName: null,
    name: "Utilisateur",
    email: "sophie.martin@example.com",
  }),
  "sophie.martin",
  "fallback e-mail si pas de prénom/nom",
);
assert.equal(
  formatTravelMessageAuthorLabel({
    firstName: "",
    lastName: "Bernard",
    name: "Comptabilité",
  }),
  "Bernard",
  "un seul champ identité suffit",
);

console.log("travels-thread-unread.test.ts ok");
