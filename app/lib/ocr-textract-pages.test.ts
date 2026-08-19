import assert from "node:assert/strict";
import test from "node:test";
import {
  buildOcrResultFromPageTexts,
  lastPageLooksUnfinished,
  looksLikeNewDocumentStart,
  mergeOcrPageTexts,
  pageClearlyEndsDocument,
  remapOcrPagesToAbsolute,
} from "./ocr-textract-pages";

test("remap : index 0 du paquet = page 9 du PDF original", () => {
  const pages = remapOcrPagesToAbsolute(
    [
      { index: 0, markdown: "Bulletin A" },
      { index: 1, markdown: "Bulletin B" },
    ],
    9,
  );
  assert.equal(pages["9"], "Bulletin A");
  assert.equal(pages["10"], "Bulletin B");
});

test("merge conserve les pages déjà lues", () => {
  const merged = mergeOcrPageTexts({ "1": "p1", "2": "p2" }, { "3": "p3" }, 90);
  assert.equal(merged.pageCount, 90);
  assert.match(merged.text, /--- Page 1 ---/);
  assert.match(merged.text, /--- Page 3 ---/);
});

test("pageCount du PDF original même si une page est vide", () => {
  const r = buildOcrResultFromPageTexts({ "1": "ok" }, 90);
  assert.equal(r.pageCount, 90);
});

test("1/2 = document pas terminé, 2/2 = fin", () => {
  assert.equal(lastPageLooksUnfinished("Bulletin Dupont page 1/2"), true);
  assert.equal(pageClearlyEndsDocument("Bulletin Dupont page 2/2"), true);
  assert.equal(lastPageLooksUnfinished("Bulletin Dupont page 2/2"), false);
});

test("nouvel INE = nouveau document, même INE = suite", () => {
  const p1 = "Bulletin INE 1234567890A DUPONT Marie 1/2";
  const p2 = "Notes INE 1234567890A 2/2";
  const p3 = "Bulletin INE 9988776655C MARTIN Paul 1/2";
  assert.equal(looksLikeNewDocumentStart(p1, p2), false);
  assert.equal(looksLikeNewDocumentStart(p2, p3), true);
});
