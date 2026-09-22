import assert from "node:assert/strict";
import test from "node:test";
import {
  badgesByEleveId,
  mergeSuggestedStatut,
  occupancyTagToAppelBadge,
} from "./vs-appel-occupancy";

test("appel occupancy — en_sortie badge exclut bulletin + dispense", () => {
  const b = occupancyTagToAppelBadge({
    eleveId: "e1",
    tag: "en_sortie",
    source: "travel:t1",
  });
  assert.equal(b.excludeFromBulletin, true);
  assert.equal(b.suggestedStatut, "dispense");
  assert.match(b.detailFr || "", /pas une absence bulletin/i);
});

test("appel occupancy — absent_vs ≠ en_sortie", () => {
  const sick = occupancyTagToAppelBadge({
    eleveId: "e2",
    tag: "absent_vs",
    source: "vs_absence:a1",
    detail: { motif: "malade" },
  });
  assert.equal(sick.excludeFromBulletin, false);
  assert.equal(sick.suggestedStatut, "absent");
  assert.equal(sick.labelFr, "Déjà signalé absent");
});

test("appel occupancy — a_infirmerie exclut bulletin + dispense", () => {
  const b = occupancyTagToAppelBadge({
    eleveId: "e3",
    tag: "a_infirmerie",
    source: "infirmerie_passage:i1",
    detail: { motif: "Maux de tête" },
  });
  assert.equal(b.excludeFromBulletin, true);
  assert.equal(b.suggestedStatut, "dispense");
  assert.equal(b.labelFr, "À l’infirmerie");
});

test("appel occupancy — hors_etablissement = absent bulletin possible", () => {
  const b = occupancyTagToAppelBadge({
    eleveId: "e4",
    tag: "hors_etablissement",
    source: "passage:p1",
  });
  assert.equal(b.excludeFromBulletin, false);
  assert.equal(b.suggestedStatut, "absent");
  assert.match(b.labelFr, /hors/i);
});

test("appel occupancy — en_cours masqué de la map UI", () => {
  const map = badgesByEleveId([
    { eleveId: "a", tag: "en_cours", source: "scolarite" },
    { eleveId: "b", tag: "en_sortie", source: "travel:t" },
  ]);
  assert.equal(map.has("a"), false);
  assert.equal(map.get("b")?.tag, "en_sortie");
});

test("mergeSuggestedStatut — conserve saisie existante", () => {
  assert.equal(
    mergeSuggestedStatut({
      existingStatut: "retard",
      occupancy: occupancyTagToAppelBadge({
        eleveId: "e",
        tag: "en_sortie",
        source: "travel:t",
      }),
    }),
    "retard",
  );
  assert.equal(
    mergeSuggestedStatut({
      occupancy: occupancyTagToAppelBadge({
        eleveId: "e",
        tag: "en_sortie",
        source: "travel:t",
      }),
    }),
    "dispense",
  );
});
