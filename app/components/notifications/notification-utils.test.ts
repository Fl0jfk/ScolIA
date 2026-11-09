import assert from "node:assert/strict";
import test from "node:test";
import {
  findIncreasedSignals,
  isInQuietHours,
  minuteToTime,
  safeInternalUrl,
  timeToMinute,
} from "./notification-utils";

const n = (id: string, count: number) => ({ id, label: id, count, href: "/", detail: "" });

test("signaux : première référence silencieuse", () => {
  assert.deepEqual(findIncreasedSignals(null, [n("a", 3)]), []);
});

test("signaux : seule une augmentation notifie", () => {
  const res = findIncreasedSignals({ a: 2, b: 1 }, [n("a", 3), n("b", 1), n("c", 1), n("d", 0)]);
  assert.deepEqual(res.map((x) => x.id), ["a", "c"]);
});

test("heures calmes à cheval sur minuit", () => {
  assert.equal(isInQuietHours(20 * 60, 19 * 60, 7 * 60 + 30), true);
  assert.equal(isInQuietHours(3 * 60, 19 * 60, 7 * 60 + 30), true);
  assert.equal(isInQuietHours(12 * 60, 19 * 60, 7 * 60 + 30), false);
  assert.equal(isInQuietHours(12 * 60, null, null), false);
});

test("heures et URL", () => {
  assert.equal(minuteToTime(450), "07:30");
  assert.equal(timeToMinute("19:00"), 1140);
  assert.equal(timeToMinute("25:00"), null);
  assert.equal(safeInternalUrl("//evil.com"), "/");
  assert.equal(safeInternalUrl("/messagerie?c=1"), "/messagerie?c=1");
});
