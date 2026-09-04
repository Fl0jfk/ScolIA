import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  mergeTravelAttachments,
  travelAttachmentIdentityKey,
} from "./travels-attachments-merge";

describe("travels-attachments-merge", () => {
  it("ne laisse pas une liste client courte écraser le serveur", () => {
    const server = [
      { name: "a.pdf", url: "https://x/attachments/1-a.pdf", s3Key: "attachments/1-a.pdf" },
      { name: "b.pdf", url: "https://x/attachments/2-b.pdf", s3Key: "attachments/2-b.pdf" },
      { name: "c.pdf", url: "https://x/attachments/3-c.pdf", s3Key: "attachments/3-c.pdf" },
    ];
    const client = [
      { name: "a.pdf", url: "https://x/attachments/1-a.pdf", s3Key: "attachments/1-a.pdf" },
    ];
    const merged = mergeTravelAttachments({ fromClient: client, fromServer: server });
    assert.equal(merged.length, 3);
  });

  it("retire uniquement les clés explicitement demandées", () => {
    const server = [
      { name: "a.pdf", url: "https://x/attachments/1-a.pdf", s3Key: "attachments/1-a.pdf" },
      { name: "b.pdf", url: "https://x/attachments/2-b.pdf", s3Key: "attachments/2-b.pdf" },
    ];
    const client = [
      { name: "a.pdf", url: "https://x/attachments/1-a.pdf", s3Key: "attachments/1-a.pdf" },
    ];
    const merged = mergeTravelAttachments({
      fromClient: client,
      fromServer: server,
      removedKeys: ["attachments/2-b.pdf"],
    });
    assert.equal(merged.length, 1);
    assert.equal(merged[0]?.s3Key, "attachments/1-a.pdf");
  });

  it("ajoute les nouvelles PJ client", () => {
    const server = [
      { name: "a.pdf", url: "https://x/attachments/1-a.pdf", s3Key: "attachments/1-a.pdf" },
    ];
    const client = [
      { name: "a.pdf", url: "https://x/attachments/1-a.pdf", s3Key: "attachments/1-a.pdf" },
      { name: "new.pdf", url: "https://x/attachments/9-new.pdf", s3Key: "attachments/9-new.pdf" },
    ];
    const merged = mergeTravelAttachments({ fromClient: client, fromServer: server });
    assert.equal(merged.length, 2);
  });

  it("dérive une clé depuis l’URL S3", () => {
    const key = travelAttachmentIdentityKey({
      name: "x.pdf",
      url: "https://s3.fr-par.scw.cloud/bucket/attachments/123-x.pdf",
    });
    assert.equal(key, "s3:attachments/123-x.pdf");
  });
});
