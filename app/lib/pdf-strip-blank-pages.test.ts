import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { PDFDocument, StandardFonts, rgb } from "pdf-lib";
import { stripBlankPagesFromPdfBytes } from "./pdf-strip-blank-pages";

async function buildSamplePdf(): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  // Page 1 : contenu
  const p1 = doc.addPage([595, 842]);
  const font = await doc.embedFont(StandardFonts.Helvetica);
  p1.drawText("PAP — page avec contenu", {
    x: 50,
    y: 750,
    size: 18,
    font,
    color: rgb(0, 0, 0),
  });
  p1.drawRectangle({
    x: 50,
    y: 600,
    width: 200,
    height: 80,
    borderColor: rgb(0, 0, 0),
    borderWidth: 2,
  });
  // Page 2 : blanche
  doc.addPage([595, 842]);
  // Page 3 : contenu
  const p3 = doc.addPage([595, 842]);
  p3.drawText("Derniere page utile", {
    x: 50,
    y: 700,
    size: 16,
    font,
    color: rgb(0.1, 0.1, 0.1),
  });
  return doc.save();
}

describe("stripBlankPagesFromPdfBytes", () => {
  it("retire la page blanche et conserve les pages avec encre", async () => {
    const input = await buildSamplePdf();
    const result = await stripBlankPagesFromPdfBytes(input);
    assert.equal(result.pageCountBefore, 3);
    assert.equal(result.changed, true);
    assert.deepEqual(result.removedPageNumbers, [2]);
    assert.equal(result.pageCountAfter, 2);

    const out = await PDFDocument.load(result.bytes);
    assert.equal(out.getPageCount(), 2);
  });
});
