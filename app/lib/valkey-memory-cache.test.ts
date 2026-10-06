import assert from "node:assert/strict";
import test from "node:test";
import {
  resetValkeyMemoryCacheForTests,
  valkeyCached,
  valkeyDel,
  valkeySetJson,
} from "@/app/lib/valkey";

test("valkeyCached — single-flight sans Valkey", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  let runs = 0;
  const loader = async () => {
    runs += 1;
    await new Promise((r) => setTimeout(r, 30));
    return { n: runs };
  };
  const [a, b, c] = await Promise.all([
    valkeyCached({ key: "test:sf:1", ttlSeconds: 20, loader }),
    valkeyCached({ key: "test:sf:1", ttlSeconds: 20, loader }),
    valkeyCached({ key: "test:sf:1", ttlSeconds: 20, loader }),
  ]);
  assert.equal(runs, 1);
  assert.deepEqual(a, b);
  assert.deepEqual(b, c);
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});

test("valkeyDel — invalide le repli mémoire", async () => {
  process.env.VALKEY_DISABLED = "1";
  resetValkeyMemoryCacheForTests();
  await valkeySetJson("test:inv:1", { v: 1 }, 30);
  let runs = 0;
  const first = await valkeyCached({
    key: "test:inv:1",
    ttlSeconds: 30,
    loader: async () => {
      runs += 1;
      return { v: 2 };
    },
  });
  assert.equal(runs, 0);
  assert.deepEqual(first, { v: 1 });
  await valkeyDel("test:inv:1");
  const second = await valkeyCached({
    key: "test:inv:1",
    ttlSeconds: 30,
    loader: async () => {
      runs += 1;
      return { v: 2 };
    },
  });
  assert.equal(runs, 1);
  assert.deepEqual(second, { v: 2 });
  delete process.env.VALKEY_DISABLED;
  resetValkeyMemoryCacheForTests();
});
