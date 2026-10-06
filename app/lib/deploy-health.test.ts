import assert from "node:assert/strict";
import test from "node:test";
import { resolveLatestMigrationTag } from "./deploy-health-logic";

test("resolveLatestMigrationTag — plus haut idx journal, pas created_at désordonné", () => {
  const journalEntries = [
    { tag: "0059_example", when: 1 },
    { tag: "0060_metier_event", when: 1787745000000 },
    { tag: "0061_vs_absence_eleve_annulee", when: 2 },
    { tag: "0062_eleve_date_sortie_actifs", when: 1787419839851 },
  ];
  const tagToContentHash = new Map([
    ["0060_metier_event", "sha0060"],
    ["0061_vs_absence_eleve_annulee", "sha0061"],
    ["0062_eleve_date_sortie_actifs", "sha0062"],
  ]);
  const applied = new Set(["sha0060", "sha0061", "sha0062"]);

  const tag = resolveLatestMigrationTag(journalEntries, tagToContentHash, applied);
  assert.equal(tag, "0062_eleve_date_sortie_actifs");
});

test("resolveLatestMigrationTag — hash legacy tag-only en BDD", () => {
  const journalEntries = [{ tag: "0001_init", when: 1 }];
  const tagToContentHash = new Map([["0001_init", "sha0001"]]);
  const applied = new Set(["0001_init"]);

  assert.equal(
    resolveLatestMigrationTag(journalEntries, tagToContentHash, applied),
    "0001_init",
  );
});

test("resolveLatestMigrationTag — aucune correspondance journal", () => {
  const journalEntries = [{ tag: "0001_init", when: 1 }];
  const tagToContentHash = new Map([["0001_init", "sha0001"]]);
  const applied = new Set(["unknown_hash"]);

  assert.equal(resolveLatestMigrationTag(journalEntries, tagToContentHash, applied), null);
});
