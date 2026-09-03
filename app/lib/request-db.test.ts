import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";

describe("request attrs roundtrip (codec)", () => {
  it("preserve routing + claimedBy + comment meta", () => {
    const blob = {
      routing: {
        source: "ai" as const,
        confidence: 0.9,
        reason: "test",
        suggestedRouteId: "admin_college",
      },
      assignedExtras: {
        ccEmails: ["a@b.c"],
        claimedBy: {
          email: "x@y.z",
          name: "X",
          userId: "u1",
          at: "2026-01-01T00:00:00.000Z",
        },
      },
      commentsMeta: {
        c1: { toRequester: true },
      },
      attachmentsMeta: {
        a1: { size: 12, uploadedAt: "2026-01-02T00:00:00.000Z" },
      },
    };
    const attrs = flattenToAttrs(blob);
    const back = inflateFromAttrs(attrs) as typeof blob;
    assert.equal(back.routing.source, "ai");
    assert.equal(back.routing.confidence, 0.9);
    assert.deepEqual(back.assignedExtras.ccEmails, ["a@b.c"]);
    assert.equal(back.assignedExtras.claimedBy?.email, "x@y.z");
    assert.equal(back.commentsMeta.c1.toRequester, true);
    assert.equal(back.attachmentsMeta.a1.size, 12);
  });
});
