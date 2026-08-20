import assert from "node:assert/strict";
import test from "node:test";
import { defaultPosterDraft, elementsForStarter } from "./catalog";
import { exportSheetSizePt, pageSizePt } from "./poster-layout";
import { snapElementMove } from "./snap";

test("A4 portrait page size", () => {
  const p = pageSizePt("a4-portrait");
  assert.ok(p.widthPt < p.heightPt);
});

test("A3 landscape page size", () => {
  const p = pageSizePt("a3-landscape");
  assert.ok(p.widthPt > p.heightPt);
});

test("A5 portrait smaller than A4", () => {
  const a4 = pageSizePt("a4-portrait");
  const a5 = pageSizePt("a5-portrait");
  assert.ok(a5.widthPt < a4.widthPt);
  assert.ok(a5.heightPt < a4.heightPt);
});

test("A5 export is A4 4-up sheet", () => {
  const sheet = exportSheetSizePt("a5-portrait");
  assert.equal(sheet.tiles, 4);
  assert.ok(Math.abs(sheet.widthPt - 595.28) < 1);
});

test("default draft has school + partner logos", () => {
  const draft = defaultPosterDraft();
  assert.ok(draft.elements.some((e) => e.kind === "logo-school"));
  assert.ok(draft.elements.some((e) => e.kind === "logo-partner"));
  assert.ok(draft.elements.some((e) => e.kind === "title"));
});

test("partner-sides starter places partner on the right", () => {
  const els = elementsForStarter("partner-sides");
  const school = els.find((e) => e.kind === "logo-school")!;
  const partner = els.find((e) => e.kind === "logo-partner")!;
  assert.ok(partner.x > school.x);
});

test("snap aligns left edges", () => {
  const moving = {
    id: "a",
    kind: "title" as const,
    x: 0.1,
    y: 0.2,
    w: 0.2,
    h: 0.1,
  };
  const other = {
    id: "b",
    kind: "body" as const,
    x: 0.2,
    y: 0.5,
    w: 0.3,
    h: 0.1,
  };
  const snapped = snapElementMove(moving, [other], 0.205, 0.2);
  assert.ok(Math.abs(snapped.x - 0.2) < 0.001);
  assert.ok(snapped.guides.some((g) => g.orientation === "v"));
});
