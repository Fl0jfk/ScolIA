import assert from "node:assert/strict";
import { test } from "node:test";
import * as XLSX from "xlsx";
import { PDFDocument } from "pdf-lib";
import { FileRenderError, normalizeSpec, renderFile, safeBaseName } from "@/app/lib/brain-ai/file-render";

const accents = "Élève Zoé, Noël à Pâques – œuvre « garçon » 12 € ’ ç ù ï";
const blocks = [
  { type: "heading", text: accents, level: 1 },
  { type: "paragraph", text: `${accents}\nDeuxième ligne` },
  { type: "bullets", items: ["un", accents] },
  { type: "table", headers: ["Nom", "Total"], rows: [["MARTIN", "=SOMME(A1:A2)"], ["Léa", "+1"]] },
];

for (const format of ["docx", "xlsx", "pdf"] as const) {
  test(`${format} : buffer valide avec accents`, async () => {
    const buf = await renderFile(normalizeSpec({ format, title: accents, blocks }));
    assert.ok(buf.length > 200);
    assert.equal(buf.subarray(0, format === "pdf" ? 4 : 2).toString(), format === "pdf" ? "%PDF" : "PK");
  });
}

test("pdf : tableau long sur plusieurs pages", async () => {
  const rows = Array.from({ length: 300 }, (_, i) => [`Nom ${i}`, "Éléonore", "6A"]);
  const buf = await renderFile(
    normalizeSpec({ format: "pdf", title: "Liste", blocks: [{ type: "table", headers: ["Nom", "Prénom", "Classe"], rows }] }),
  );
  const pdf = await PDFDocument.load(buf);
  assert.ok(pdf.getPageCount() >= 5);
});

test("pdf : caractères non gérés ne font pas planter", async () => {
  const buf = await renderFile(
    normalizeSpec({ format: "pdf", title: "x", blocks: [{ type: "paragraph", text: "Emoji 😀 chinois 中 flèche → ok" }] }),
  );
  assert.equal(buf.subarray(0, 4).toString(), "%PDF");
});

test("xlsx : =SOMME reste du texte, une feuille par tableau, nom <= 31", async () => {
  const buf = await renderFile(
    normalizeSpec({
      format: "xlsx",
      title: "t",
      blocks: [
        { type: "heading", text: "Un titre de feuille beaucoup trop long pour Excel [x]" },
        ...blocks.filter((b) => b.type === "table"),
        { type: "table", headers: ["a"], rows: [["@cmd"], ["-2"]] },
      ],
    }),
  );
  const wb = XLSX.read(buf, { type: "buffer" });
  assert.equal(wb.SheetNames.length, 2);
  assert.ok(wb.SheetNames.every((n) => n.length <= 31 && !/[[\]]/.test(n)));
  const c = wb.Sheets[wb.SheetNames[0]!]!["B2"]!;
  assert.equal(c.t, "s");
  assert.equal(c.v, "=SOMME(A1:A2)");
  assert.equal(c.f, undefined);
});

test("limites refusées proprement", () => {
  assert.throws(() => normalizeSpec({ format: "pdf", title: "", blocks: [] }), FileRenderError);
  assert.throws(() => normalizeSpec({ format: "zip", title: "", blocks: [{ type: "paragraph", text: "a" }] }), FileRenderError);
  assert.throws(
    () => normalizeSpec({ format: "pdf", title: "", blocks: Array.from({ length: 201 }, () => ({ type: "paragraph", text: "a" })) }),
    FileRenderError,
  );
  assert.throws(
    () => normalizeSpec({ format: "xlsx", title: "", blocks: [{ type: "table", headers: ["a"], rows: Array.from({ length: 2001 }, () => ["x"]) }] }),
    FileRenderError,
  );
  assert.throws(
    () => normalizeSpec({ format: "docx", title: "", blocks: [{ type: "paragraph", text: "a".repeat(20_001) }] }),
    FileRenderError,
  );
});

test("nettoyage : contrôles retirés, nom de fichier sûr", () => {
  const s = normalizeSpec({ format: "docx", title: "a\u0000b", blocks: [{ type: "paragraph", text: "x\u0007y" }] });
  assert.equal(s.title, "ab");
  assert.equal((s.blocks[0] as { text: string }).text, "xy");
  assert.equal(safeBaseName("Café/../liste élèves.pdf"), "Cafe_liste_eleves");
  assert.equal(safeBaseName(""), "document");
});
