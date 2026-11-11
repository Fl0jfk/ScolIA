import assert from "node:assert/strict";
import test from "node:test";
import { isSafePublicHttpsUrl } from "@/app/lib/ssrf-guard";

test("NJ-10 : refuse adresses internes, IP, http et identifiants", () => {
  for (const u of [
    "http://bucket.s3.fr-par.scw.cloud/x.pdf",
    "https://169.254.169.254/latest/meta-data",
    "https://127.0.0.1/x",
    "https://[::1]/x",
    "https://localhost/x",
    "https://metadata/x",
    "https://svc.internal/x",
    "https://user:pw@example.com/x",
    "file:///etc/passwd",
    "n'importe quoi",
  ]) {
    assert.equal(isSafePublicHttpsUrl(u), false, u);
  }
  assert.equal(isSafePublicHttpsUrl("https://bucket.s3.fr-par.scw.cloud/devis.pdf"), true);
});
