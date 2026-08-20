import assert from "node:assert/strict";
import test from "node:test";
import { defaultPosterDraft } from "./catalog";
import { computePosterLayout, pageSizePt } from "./layout";

test("A4 portrait page size", () => {
  const p = pageSizePt("a4-portrait");
  assert.ok(p.widthPt < p.heightPt);
});

test("A3 landscape page size", () => {
  const p = pageSizePt("a3-landscape");
  assert.ok(p.widthPt > p.heightPt);
});

test("logos-top place school logo and title", () => {
  const draft = defaultPosterDraft();
  const layout = computePosterLayout(draft);
  assert.ok(layout.boxes.logoSchool.w > 0);
  assert.ok(layout.boxes.title.y > layout.boxes.logoSchool.y);
  assert.equal(layout.boxes.logoPartner, null);
});

test("partner logo appears when key set", () => {
  const draft = defaultPosterDraft();
  draft.partnerLogoKey = "posters/assets/partner.png";
  const layout = computePosterLayout(draft);
  assert.ok(layout.boxes.logoPartner);
  assert.ok(layout.boxes.logoPartner!.x > layout.boxes.logoSchool.x);
});

test("V2 titleOffsetY shifts title", () => {
  const draft = defaultPosterDraft();
  const base = computePosterLayout(draft);
  draft.offsets = { ...draft.offsets, titleOffsetY: 0.05 };
  const shifted = computePosterLayout(draft);
  assert.ok(shifted.boxes.title.y > base.boxes.title.y);
});

test("photo-full uses higher title band", () => {
  const draft = defaultPosterDraft();
  draft.layoutPreset = "photo-full";
  const layout = computePosterLayout(draft);
  assert.ok(layout.boxes.title.y > 0.3);
});

test("datePlace block respects toggle", () => {
  const draft = defaultPosterDraft();
  draft.dateLabel = "Mars 2026";
  draft.blocks.showDatePlace = false;
  assert.equal(computePosterLayout(draft).boxes.datePlace, null);
  draft.blocks.showDatePlace = true;
  assert.ok(computePosterLayout(draft).boxes.datePlace);
});
