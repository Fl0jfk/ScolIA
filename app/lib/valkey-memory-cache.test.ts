import assert from "node:assert/strict";
import { mock, test } from "node:test";
import { valkeyKeyStagesConventionsIndex } from "@/app/lib/valkey-keys";
import {
  isScopedMemoryCacheKey,
  resetValkeyMemoryCacheForTests,
  valkeyCached,
  valkeyDel,
  valkeySetJson,
} from "@/app/lib/valkey";

const SCOPED_KEY_A = valkeyKeyStagesConventionsIndex("etab-a");
const SCOPED_KEY_B = valkeyKeyStagesConventionsIndex("etab-b");
const UNSCOPED_KEY = "scola:travels:index:v2:etab-a";

test("isScopedMemoryCacheKey — index, roster, élèves uniquement", () => {
  assert.equal(isScopedMemoryCacheKey(SCOPED_KEY_A), true);
  assert.equal(isScopedMemoryCacheKey("scola:stages:roster:e1:2025-2026:1a"), true);
  assert.equal(isScopedMemoryCacheKey("scola:eleves:reg:e1:inscrit"), true);
  assert.equal(isScopedMemoryCacheKey(UNSCOPED_KEY), false);
  assert.equal(isScopedMemoryCacheKey("scola:stages:conv:e1:c1"), false);
});

test("valkeyCached scopé — single-flight sans Valkey", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  let runs = 0;
  const loader = async () => {
    runs += 1;
    await new Promise((r) => setTimeout(r, 25));
    return { n: runs };
  };
  const [a, b, c] = await Promise.all([
    valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 45, loader }),
    valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 45, loader }),
    valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 45, loader }),
  ]);
  assert.equal(runs, 1);
  assert.deepEqual(a, b);
  assert.deepEqual(b, c);
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("valkeyCached non scopé — pas de cache mémoire sans Valkey", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  let runs = 0;
  const loader = async () => {
    runs += 1;
    return runs;
  };
  await valkeyCached({ key: UNSCOPED_KEY, ttlSeconds: 30, loader });
  await valkeyCached({ key: UNSCOPED_KEY, ttlSeconds: 30, loader });
  assert.equal(runs, 2);
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("valkeyDel — invalide le cache scopé (génération)", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  let runs = 0;
  const loader = async () => {
    runs += 1;
    return { v: runs };
  };
  const first = await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 30, loader });
  assert.equal(first.v, 1);
  await valkeyDel(SCOPED_KEY_A);
  const second = await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 30, loader });
  assert.equal(second.v, 2);
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("isolation tenant — clés scopées distinctes", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  await valkeyCached({
    key: SCOPED_KEY_A,
    ttlSeconds: 30,
    loader: async () => ({ tenant: "a" }),
  });
  const b = await valkeyCached({
    key: SCOPED_KEY_B,
    ttlSeconds: 30,
    loader: async () => ({ tenant: "b" }),
  });
  assert.deepEqual(b, { tenant: "b" });
  const aAgain = await valkeyCached({
    key: SCOPED_KEY_A,
    ttlSeconds: 30,
    loader: async () => ({ tenant: "reload" }),
  });
  assert.deepEqual(aAgain, { tenant: "a" });
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("génération — lecture concurrente invalidée ne réécrit pas un résultat périmé", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  let runs = 0;
  const loader = async () => {
    runs += 1;
    await new Promise((r) => setTimeout(r, 40));
    return { v: runs };
  };
  const p1 = valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 30, loader });
  await new Promise((r) => setTimeout(r, 10));
  await valkeyDel(SCOPED_KEY_A);
  const r1 = await p1;
  assert.equal(r1.v, 1);
  const r2 = await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 30, loader });
  assert.equal(r2.v, 2);
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("valkeySetJson sans Valkey — n’alimente pas le cache mémoire global", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  const ok = await valkeySetJson(UNSCOPED_KEY, { secret: true }, 60);
  assert.equal(ok, false);
  let runs = 0;
  const out = await valkeyCached({
    key: UNSCOPED_KEY,
    ttlSeconds: 30,
    loader: async () => {
      runs += 1;
      return { loaded: true };
    },
  });
  assert.equal(runs, 1);
  assert.deepEqual(out, { loaded: true });
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("invalidation type save — sauvegardes fréquentes relancent le loader (pas de cache périmé)", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  let runs = 0;
  const loader = async () => {
    runs += 1;
    return { generation: runs };
  };
  await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 120, loader });
  assert.equal(runs, 1);
  for (let i = 0; i < 8; i += 1) {
    await valkeyDel(SCOPED_KEY_A);
    const row = await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 120, loader });
    assert.equal(row.generation, i + 2);
  }
  assert.equal(runs, 9);
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("TTL mémoire scopé — plafond 30 s, rechargement même sans invalidation", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  mock.timers.enable({ apis: ["Date"] });
  let runs = 0;
  const loader = async () => {
    runs += 1;
    return runs;
  };
  const first = await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 120, loader });
  assert.equal(first, 1);
  assert.equal(runs, 1);

  mock.timers.tick(5_000);
  const stillCached = await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 120, loader });
  assert.equal(stillCached, 1);
  assert.equal(runs, 1);

  mock.timers.tick(26_000);
  const reloaded = await valkeyCached({ key: SCOPED_KEY_A, ttlSeconds: 120, loader });
  assert.equal(reloaded, 2);
  assert.equal(runs, 2);

  mock.timers.reset();
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});
