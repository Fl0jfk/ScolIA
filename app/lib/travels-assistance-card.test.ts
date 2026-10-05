import assert from "node:assert/strict";
import test from "node:test";
import { parseTravelsModule } from "./app-config-schemas";

test("parseTravelsModule — carte d’assistance", () => {
  const parsed = parseTravelsModule({
    comptaEmails: [],
    transportProviders: [],
    assistanceCardS3Key: "tenant/settings/travels/assistance-card.pdf",
    assistanceCardFileName: "Mutuelle 2026.pdf",
  });
  assert.equal(parsed.assistanceCardS3Key, "tenant/settings/travels/assistance-card.pdf");
  assert.equal(parsed.assistanceCardFileName, "Mutuelle 2026.pdf");
});

test("parseTravelsModule — sans clé S3, pas de nom fichier", () => {
  const parsed = parseTravelsModule({
    transportProviders: [],
    assistanceCardFileName: "orphelin.pdf",
  });
  assert.equal(parsed.assistanceCardS3Key, undefined);
  assert.equal(parsed.assistanceCardFileName, undefined);
});

test("parseTravelsModule — fusion partielle conserve la carte", () => {
  const existing = parseTravelsModule({
    transportProviders: [{ name: "Bus", email: "bus@example.com" }],
    assistanceCardS3Key: "settings/travels/assistance-card.pdf",
    assistanceCardFileName: "carte.pdf",
  });
  const merged = parseTravelsModule({
    ...existing,
    pdfFooterText: "Pied de page",
  });
  assert.equal(merged.assistanceCardS3Key, "settings/travels/assistance-card.pdf");
  assert.equal(merged.pdfFooterText, "Pied de page");
});
