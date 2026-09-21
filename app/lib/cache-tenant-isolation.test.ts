import assert from "node:assert/strict";
import test from "node:test";
import { valkeyKeyAppConfig, valkeyKeyModuleAccessConfig } from "@/app/lib/valkey-keys";
import { executeBrainTool } from "@/app/lib/brain-ai/tools/execute";
import type { BrainToolCtx } from "@/app/lib/brain-ai/types";
import { PROXY_PUBLIC_ROUTE_MATCHERS } from "@/app/lib/public-routes";

test("valkey app-config — deux slugs = deux clés", () => {
  assert.notEqual(valkeyKeyAppConfig("college-a"), valkeyKeyAppConfig("college-b"));
  assert.equal(valkeyKeyAppConfig("college-a"), "scola:cfg:app:college-a");
  assert.notEqual(
    valkeyKeyModuleAccessConfig("college-a"),
    valkeyKeyModuleAccessConfig("college-b"),
  );
});

test("executeBrainTool — refus sans etablissementId", async () => {
  const ctx: BrainToolCtx = {
    userId: "u1",
    roles: ["admin"],
    isOrgAdmin: true,
    audience: "private",
    confirmed: false,
    etablissementId: null,
  };
  const result = await executeBrainTool("list_trips_brief", {}, ctx);
  assert.equal(result.ok, false);
  if (!result.ok) {
    assert.equal("code" in result && result.code, "MISSING_ETABLISSEMENT");
  }
});

test("executeBrainTool — refus etablissementId vide", async () => {
  const ctx: BrainToolCtx = {
    userId: "u1",
    roles: ["admin"],
    isOrgAdmin: true,
    audience: "private",
    confirmed: false,
    etablissementId: "   ",
  };
  const result = await executeBrainTool("list_trips_brief", {}, ctx);
  assert.equal(result.ok, false);
  if (!result.ok) {
    assert.equal("code" in result && result.code, "MISSING_ETABLISSEMENT");
  }
});

test("/api/chatbot n’est plus une route publique proxy", () => {
  assert.equal(
    (PROXY_PUBLIC_ROUTE_MATCHERS as readonly string[]).includes("/api/chatbot"),
    false,
  );
});
