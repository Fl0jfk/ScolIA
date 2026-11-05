import assert from "node:assert/strict";
import { test } from "node:test";
import {
  CHAT_MAX_ATTACHMENTS,
  chatAttachmentKind,
  chatUploadPrefix,
  sanitizeChatAttachments,
} from "@/app/lib/brain-ai/chat-attachments";

test("types acceptés", () => {
  assert.equal(chatAttachmentKind("application/pdf"), "pdf");
  assert.equal(chatAttachmentKind("IMAGE/JPEG"), "image");
  assert.equal(chatAttachmentKind("text/csv"), "text");
  assert.equal(chatAttachmentKind("application/zip"), null);
  assert.equal(chatAttachmentKind("image/svg+xml"), null);
});

test("un utilisateur ne lit que ses propres fichiers", () => {
  const mine = `${chatUploadPrefix("user-1")}/123-a.pdf`;
  const other = `${chatUploadPrefix("user-2")}/123-b.pdf`;
  const out = sanitizeChatAttachments(
    [
      { key: mine, fileName: "a.pdf", contentType: "application/pdf" },
      { key: other, fileName: "b.pdf", contentType: "application/pdf" },
      { key: "rh/dossiers/secret.pdf", fileName: "x.pdf", contentType: "application/pdf" },
      { key: `${chatUploadPrefix("user-1")}/../user-2/b.pdf`, fileName: "y.pdf", contentType: "application/pdf" },
      // Préfixe d'un autre identifiant qui commence pareil.
      { key: `${chatUploadPrefix("user-10")}/c.pdf`, fileName: "c.pdf", contentType: "application/pdf" },
    ],
    "user-1",
  );
  assert.deepEqual(
    out.map((a) => a.key),
    [mine],
  );
});

test("type non pris en charge ignoré, 10 fichiers maximum", () => {
  const prefix = chatUploadPrefix("u");
  const many = Array.from({ length: 15 }, (_, i) => ({
    key: `${prefix}/${i}.jpg`,
    fileName: `${i}.jpg`,
    contentType: "image/jpeg",
  }));
  many.unshift({ key: `${prefix}/x.exe`, fileName: "x.exe", contentType: "application/octet-stream" });
  assert.equal(sanitizeChatAttachments(many, "u").length, CHAT_MAX_ATTACHMENTS);
  assert.deepEqual(sanitizeChatAttachments("pas une liste", "u"), []);
});
