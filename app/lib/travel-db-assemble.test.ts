import assert from "node:assert/strict";
import test from "node:test";

/**
 * Régression : dates uniquement dans travel_attr ne doivent pas être écrasées par colonnes vides.
 * Logique extraite de assembleTravel (travel-db.ts).
 */
test("assembleTravel data — repli attr si colonnes travel vides", () => {
  const m = {
    startDate: null as string | null,
    endDate: null as string | null,
    siteLabel: "Collège",
    title: "Sortie",
    destination: "Caen",
    classes: null as string | null,
    startTime: null,
    endTime: null,
    nbEleves: null,
    nbAccompagnateurs: null,
    listeElevesStatus: null,
  };
  const dataFromAttrs = {
    startDate: "2026-10-14",
    endDate: "2026-10-15",
    date: "2026-10-14",
  };
  const attrStart =
    typeof dataFromAttrs.startDate === "string" ? dataFromAttrs.startDate : undefined;
  const attrEnd = typeof dataFromAttrs.endDate === "string" ? dataFromAttrs.endDate : undefined;
  const attrDate = typeof dataFromAttrs.date === "string" ? dataFromAttrs.date : undefined;
  const data = {
    ...dataFromAttrs,
    startDate: m.startDate ?? attrStart ?? attrDate ?? undefined,
    endDate: m.endDate ?? attrEnd ?? undefined,
    date: attrDate ?? m.startDate ?? undefined,
  };
  assert.equal(data.startDate, "2026-10-14");
  assert.equal(data.endDate, "2026-10-15");
});
