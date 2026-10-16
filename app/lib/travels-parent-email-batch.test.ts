import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  mergeNotifiedParentEmails,
  remainingParentEmailsToNotify,
  sendParentEmailsInBatches,
} from "@/app/lib/travels-parent-email-batch";

describe("travels-parent-email-batch", () => {
  it("ignore les e-mails déjà notifiés", () => {
    const all = ["parent1@x.fr", "parent2@x.fr", "parent3@x.fr"];
    const pending = remainingParentEmailsToNotify(all, ["parent2@x.fr", "PARENT2@x.fr"]);
    assert.deepEqual(pending, ["parent1@x.fr", "parent3@x.fr"]);
  });

  it("fusionne les listes notifiées sans doublon", () => {
    const merged = mergeNotifiedParentEmails(["a@x.fr"], ["B@x.fr", "a@x.fr", "c@x.fr"]);
    assert.equal(merged.length, 3);
    assert.ok(merged.includes("a@x.fr"));
    assert.ok(merged.some((e) => e.toLowerCase() === "b@x.fr"));
  });

  it("conserve le décompte après échec au milieu des lots", async () => {
    const emails = Array.from({ length: 5 }, (_, i) => `p${i}@test.fr`);
    let batchCalls = 0;
    const result = await sendParentEmailsInBatches(emails, 2, async () => {
      batchCalls += 1;
      if (batchCalls === 2) {
        throw new Error("SMTP timeout");
      }
    });
    assert.equal(result.completed, false);
    assert.equal(result.sentCount, 2);
    assert.deepEqual(result.sentEmails, ["p0@test.fr", "p1@test.fr"]);
    assert.equal(result.error instanceof Error, true);
  });
});
