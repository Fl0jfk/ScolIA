import assert from "node:assert/strict";
import test from "node:test";
import {
  auditRequestContextFromRequest,
  validatedClientIpFromForwardedFor,
} from "@/app/lib/audit-request-context";
import { clientIpFromRequest } from "@/app/lib/memory-rate-limit";

test("clientIpFromRequest — première entrée X-Forwarded-For (rate limit)", () => {
  const req = new Request("https://example.test/x", {
    headers: {
      "x-forwarded-for": "203.0.113.10, 10.0.0.1, 10.0.0.2",
    },
  });
  assert.equal(clientIpFromRequest(req), "203.0.113.10");
});

test("auditRequestContextFromRequest — IP validée + XFF brut + Envoy", () => {
  const req = new Request("https://example.test/x", {
    headers: {
      "x-forwarded-for": "203.0.113.10, 10.0.0.1",
      "x-envoy-external-address": "198.51.100.5",
    },
  });
  const ctx = auditRequestContextFromRequest(req);
  assert.equal(ctx.clientIp, "203.0.113.10");
  assert.equal(ctx.forwardedFor, "203.0.113.10, 10.0.0.1");
  assert.equal(ctx.envoyExternalAddress, "198.51.100.5");
});

test("validatedClientIpFromForwardedFor — invalide → null", () => {
  assert.equal(validatedClientIpFromForwardedFor("not-an-ip, 1.2.3.4"), null);
  assert.equal(validatedClientIpFromForwardedFor(null), null);
});
