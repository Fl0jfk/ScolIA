import assert from "node:assert/strict";
import test from "node:test";
import { isRiskyUpload } from "@/app/lib/upload-guard";

test("NJ-38 : contenus actifs refusés, documents courants admis", () => {
  assert.equal(isRiskyUpload("x.svg", "image/svg+xml"), true);
  assert.equal(isRiskyUpload("page.html", "application/octet-stream"), true);
  assert.equal(isRiskyUpload("a.pdf", "text/html"), true);
  assert.equal(isRiskyUpload("devis.pdf", "application/pdf"), false);
  assert.equal(isRiskyUpload("photo.jpg", "image/jpeg"), false);
});
