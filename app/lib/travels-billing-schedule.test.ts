import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  computeMonthlySchedule,
  DEFAULT_LAST_BILLING_MONTH_NUMBER,
  defaultLastBillingMonth,
  monthLabelFr,
} from "@/app/lib/travels-billing-schedule";

function sumCents(months: { amount: number }[]): number {
  return months.reduce((acc, m) => acc + Math.round(m.amount * 100), 0);
}

describe("lissage mensuel", () => {
  it("division exacte", () => {
    const r = computeMonthlySchedule(300, "2027-01", "2027-06");
    assert.ok(r.ok);
    assert.equal(r.count, 6);
    assert.equal(r.monthlyAmount, 50);
    assert.equal(r.lastDiffers, false);
    assert.equal(r.months[0].month, "2027-01");
    assert.equal(r.months[5].month, "2027-06");
  });
  it("le reste d'arrondi va sur le dernier mois, somme exacte au centime", () => {
    const r = computeMonthlySchedule(100, "2027-01", "2027-03");
    assert.ok(r.ok);
    assert.deepEqual(r.months.map((m) => m.amount), [33.33, 33.33, 33.34]);
    assert.equal(r.lastDiffers, true);
    assert.equal(sumCents(r.months), 10000);
  });
  it("somme exacte sur de nombreux cas", () => {
    for (const price of [0.01, 0.05, 5, 99.99, 123.45, 250, 387.77, 1000.1]) {
      for (let n = 1; n <= 12; n++) {
        const last = `2027-${String(n).padStart(2, "0")}`;
        const r = computeMonthlySchedule(price, "2027-01", last);
        assert.ok(r.ok);
        assert.equal(sumCents(r.months), Math.round(price * 100));
        assert.ok(r.months.every((m) => m.amount >= 0));
      }
    }
  });
  it("un ou deux mois restants : réparti sur ces mois", () => {
    const one = computeMonthlySchedule(120, "2027-06", "2027-06");
    assert.ok(one.ok);
    assert.deepEqual(one.months.map((m) => m.amount), [120]);
    const two = computeMonthlySchedule(120.01, "2027-06", "2027-07");
    assert.ok(two.ok);
    assert.equal(two.count, 2);
    assert.equal(sumCents(two.months), 12001);
  });
  it("passe d'une année à l'autre", () => {
    const r = computeMonthlySchedule(90, "2026-11", "2027-01");
    assert.ok(r.ok);
    assert.deepEqual(r.months.map((m) => m.month), ["2026-11", "2026-12", "2027-01"]);
  });
  it("0 € : pas de lissage, même sans mois", () => {
    const r = computeMonthlySchedule(0, undefined, undefined);
    assert.ok(r.ok);
    assert.equal(r.count, 0);
    assert.deepEqual(r.months, []);
  });
  it("erreurs claires : mois manquants, premier après dernier, période trop longue, montant invalide", () => {
    assert.equal(computeMonthlySchedule(100, "", "2027-07").ok, false);
    assert.equal(computeMonthlySchedule(100, "2027-01", undefined).ok, false);
    const bad = computeMonthlySchedule(100, "2027-08", "2027-07");
    assert.equal(bad.ok, false);
    if (!bad.ok) assert.match(bad.error, /août 2027.*après le dernier.*juillet 2027/);
    assert.equal(computeMonthlySchedule(100, "2020-01", "2027-07").ok, false);
    assert.equal(computeMonthlySchedule(-5, "2027-01", "2027-02").ok, false);
    assert.equal(computeMonthlySchedule(100, "2027-13", "2027-14").ok, false);
  });
  it("réutilisable sur un autre montant (reste à charge d'une famille aidée)", () => {
    const r = computeMonthlySchedule(37.5, "2027-02", "2027-07");
    assert.ok(r.ok);
    assert.equal(sumCents(r.months), 3750);
  });
  it("dernier mois par défaut : juillet de l'année scolaire en cours", () => {
    assert.equal(DEFAULT_LAST_BILLING_MONTH_NUMBER, 7);
    assert.equal(defaultLastBillingMonth("2026-11-24"), "2027-07");
    assert.equal(defaultLastBillingMonth("2027-03-02"), "2027-07");
    assert.equal(defaultLastBillingMonth("2027-09-01"), "2028-07");
  });
  it("libellé de mois en français", () => {
    assert.equal(monthLabelFr("2027-02"), "février 2027");
    assert.equal(monthLabelFr("2027-12"), "décembre 2027");
  });
});
