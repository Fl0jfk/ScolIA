import { test } from "node:test";
import assert from "node:assert/strict";
import { getTenantAppUrl, runWithTenant, tenantAbsolutePath } from "@/app/lib/tenant-context";
import type { TenantConfig } from "@/app/lib/tenant-types";

/**
 * Les tâches lancées par instrumentation.ts n'ont pas de requête HTTP : `headers()` de Next y lève une erreur.
 * Ces tests appellent le vrai code hors de toute requête (aucune base, aucun réseau).
 */
const tenant = (slug: string): TenantConfig =>
  ({ slug, hostnames: [`${slug}.exemple.test`], appUrl: `https://${slug}.exemple.test` }) as unknown as TenantConfig;

test("tâche serveur : l'adresse est celle de l'établissement courant", async () => {
  const url = await runWithTenant(tenant("ecole-a"), () => getTenantAppUrl());
  assert.equal(url, "https://ecole-a.exemple.test");
});

test("tâche serveur : liens absolus propres à chaque établissement, jamais NEXT_PUBLIC_APP_URL", async () => {
  const before = process.env.NEXT_PUBLIC_APP_URL;
  process.env.NEXT_PUBLIC_APP_URL = "https://autre.exemple.test";
  try {
    const links = await Promise.all(
      ["ecole-a", "ecole-b"].map((slug) => runWithTenant(tenant(slug), () => tenantAbsolutePath("travels/t1"))),
    );
    assert.deepEqual(links, ["https://ecole-a.exemple.test/travels/t1", "https://ecole-b.exemple.test/travels/t1"]);
  } finally {
    if (before === undefined) delete process.env.NEXT_PUBLIC_APP_URL;
    else process.env.NEXT_PUBLIC_APP_URL = before;
  }
});

test("sans requête ni établissement explicite : l'erreur reste levée (pas d'adresse devinée)", async () => {
  await assert.rejects(() => getTenantAppUrl(), /outside a request scope/);
});
