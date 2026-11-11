import assert from "node:assert/strict";
import test from "node:test";
import { parsePosterDraft } from "@/app/lib/posters/parse-draft";

test("NJ-39 : une clé d'image hors posters/assets est écartée", () => {
  const d = parsePosterDraft({
    partnerLogoKey: "eleves/dossiers/bulletin.pdf",
    backgroundImageKey: "posters/assets/background/1-a.png",
    elements: [{ id: "x", kind: "image", x: 0.1, y: 0.1, w: 0.2, h: 0.2, imageKey: "../secret/x.png" }],
  });
  assert.equal(d.partnerLogoKey, null);
  assert.equal(d.backgroundImageKey, "posters/assets/background/1-a.png");
  assert.ok(d.elements.every((e) => !e.imageKey || e.imageKey.startsWith("posters/assets/")));
});
