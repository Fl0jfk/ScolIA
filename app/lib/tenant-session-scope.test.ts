import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { resolveTenantScope, scopedOrgAdminFlag } from "@/app/lib/tenant-session-scope";
import { isLocalDevHostname } from "@/app/lib/local-host-keys";
import { tenantOrigin } from "@/app/lib/tenant-auth-urls";
import type { TenantConfig } from "@/app/lib/tenant-types";

const deps = (member: boolean) => ({
  belongs: async () => member,
  tenantEtablissementId: async () => "etab-visite",
});
const account = { homeEtablissementId: "etab-origine", platformAdmin: false, tenantSlug: "ecole-b" };

test("un compte d'un autre établissement est traité comme non connecté", async () => {
  assert.equal(await resolveTenantScope(account, deps(false)), null);
});

test("un membre est évalué sur l'établissement visité, pas sur son établissement d'origine", async () => {
  assert.deepEqual(await resolveTenantScope(account, deps(true)), { activeEtablissementId: "etab-visite" });
});

test("la plateforme et son administrateur gardent leur établissement d'origine", async () => {
  const expected = { activeEtablissementId: "etab-origine" };
  assert.deepEqual(await resolveTenantScope({ ...account, platformAdmin: true }, deps(false)), expected);
  assert.deepEqual(await resolveTenantScope({ ...account, tenantSlug: "platform" }, deps(false)), expected);
});

test("le drapeau admin du compte ne suit pas dans un autre établissement", () => {
  assert.equal(scopedOrgAdminFlag(true, "a", "a"), true);
  assert.equal(scopedOrgAdminFlag(true, "a", "b"), false);
  assert.equal(scopedOrgAdminFlag(true, null, null), false);
  assert.equal(scopedOrgAdminFlag(false, "a", "a"), false);
});

const tenant = {
  slug: "ecole",
  label: "École",
  hostnames: ["ecole.scolia.fr"],
  appUrl: "https://ecole.scolia.fr",
} as unknown as TenantConfig;

function inProduction(run: () => void) {
  const env = process.env as Record<string, string | undefined>;
  const previous = env.NODE_ENV;
  env.NODE_ENV = "production";
  try {
    run();
  } finally {
    env.NODE_ENV = previous;
  }
}

test("en production, un nom d'hôte inventé ou un tunnel ne finit jamais dans un lien", () => {
  inProduction(() => {
    for (const host of ["pirate.ngrok-free.app", "pirate.trycloudflare.com", "pirate.example"]) {
      assert.equal(tenantOrigin(tenant, host), "https://ecole.scolia.fr", host);
    }
    assert.equal(isLocalDevHostname("pirate.loca.lt"), false);
    assert.equal(isLocalDevHostname("localhost"), true);
  });
});

test("une supervision ne vaut que sur l'établissement où elle a été ouverte", () => {
  const source = readFileSync("app/lib/supervision.ts", "utf8");
  assert.match(source, /if \(cookie\.etablissementId !== visited\) return null;/);
  assert.match(source, /scopedOrgAdminFlag\(Boolean\(row\.orgAdmin\)/);
  assert.match(readFileSync("proxy.ts", "utf8"), /cookie: supervisionCookie,\s+tenant,/);
});

test("aucune relance ni aucun lien sensible n'est construit depuis les en-têtes du navigateur", () => {
  for (const path of [
    "app/api/agentIAOCR/batch-job/route.ts",
    "app/api/certificates/programs/[id]/bulk/route.ts",
    "app/api/rh/deposit/bulk/job/route.ts",
    "app/api/rh/deposit/bulk/process/route.ts",
    "app/api/account/security/route.ts",
  ]) {
    assert.doesNotMatch(readFileSync(path, "utf8"), /x-forwarded-(host|proto)/, path);
  }
});
