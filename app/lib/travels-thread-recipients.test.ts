import assert from "node:assert/strict";
import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  selectTravelThreadEmailRecipients,
  type TravelThreadMemberRef,
} from "@/app/lib/travels-thread-recipients";

const establishments: Establishment[] = [
  { id: "ecole", label: "École", kind: "ecole", active: true },
  { id: "college", label: "Collège", kind: "college", active: true },
  { id: "lycee", label: "Lycée", kind: "lycee", active: true },
];

const member = (
  id: string,
  email: string,
  roles: string[],
  extra: Partial<TravelThreadMemberRef> = {},
): TravelThreadMemberRef => ({
  userId: id,
  externalUserId: id,
  email,
  firstName: id,
  lastName: "Test",
  roles,
  pending: false,
  ...extra,
});

const members: TravelThreadMemberRef[] = [
  member("prof-1", "prof1@ex.fr", ["professeur"]),
  member("prof-2", "prof2@ex.fr", ["professeur"]),
  member("dir-col", "dircol@ex.fr", ["direction_college"]),
  member("dir-lyc", "dirlyc@ex.fr", ["direction_lycee"]),
  member("cpta-1", "compta@ex.fr", ["comptabilite"]),
  member("cpta-2", "COMPTA@ex.fr", ["compta"]),
  member("cpta-pending", "pending@ex.fr", ["comptabilite"], { pending: true }),
];

const tripCollege = {
  ownerId: "prof-1",
  ownerName: "Mme Dupont",
  ownerEmail: "prof1@ex.fr",
  data: { etablissement: "Collège" },
};

const emails = (list: { email: string }[]) => list.map((r) => r.email.toLowerCase()).sort();

// Message du professeur : compta + direction collège, pas lui-même, pas le lycée, pas les autres profs.
assert.deepEqual(
  emails(
    selectTravelThreadEmailRecipients({
      trip: tripCollege,
      members,
      establishments,
      authorUserIds: ["prof-1"],
      authorEmail: "prof1@ex.fr",
    }),
  ),
  ["compta@ex.fr", "dircol@ex.fr"],
);

// Message de la compta : créateur + direction collège ; l’auteur est exclu même via une 2e fiche à la même adresse.
assert.deepEqual(
  emails(
    selectTravelThreadEmailRecipients({
      trip: tripCollege,
      members,
      establishments,
      authorUserIds: ["cpta-1"],
      authorEmail: "compta@ex.fr",
    }),
  ),
  ["dircol@ex.fr", "prof1@ex.fr"],
);

// Créateur absent de l’annuaire : repli sur l’e-mail du dossier.
assert.deepEqual(
  emails(
    selectTravelThreadEmailRecipients({
      trip: { ...tripCollege, ownerId: "ancien", ownerEmail: "ancien@ex.fr" },
      members,
      establishments,
      authorUserIds: ["dir-col"],
      authorEmail: "dircol@ex.fr",
    }),
  ),
  ["ancien@ex.fr", "compta@ex.fr"],
);

// Groupe scolaire : toutes les directions.
assert.deepEqual(
  emails(
    selectTravelThreadEmailRecipients({
      trip: { ...tripCollege, data: { etablissement: "Groupe Scolaire" } },
      members,
      establishments,
      authorUserIds: ["cpta-1"],
      authorEmail: "compta@ex.fr",
    }),
  ),
  ["dircol@ex.fr", "dirlyc@ex.fr", "prof1@ex.fr"],
);

console.log("travels-thread-recipients: OK");
