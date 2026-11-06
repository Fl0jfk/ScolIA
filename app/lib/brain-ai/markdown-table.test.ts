import assert from "node:assert/strict";
import { test } from "node:test";
import { messageTables, splitMessageSegments } from "@/app/lib/brain-ai/markdown-table";

const answer = [
  "Voici les 3 élèves du collège avec un PAP :",
  "",
  "| Nom | Prénom | Classe |",
  "|-----|--------|:------:|",
  "| MARTIN | Léa | 6A |",
  "| **DURAND** | Tom | 4B |",
  "| PETIT | Inès |",
  "",
  "Vous pouvez l’exporter en Excel avec le bouton ci-dessous.",
].join("\n");

test("texte et tableau séparés", () => {
  const segs = splitMessageSegments(answer);
  assert.deepEqual(
    segs.map((s) => s.type),
    ["text", "table", "text"],
  );
  const table = messageTables(answer)[0]!;
  assert.deepEqual(table.headers, ["Nom", "Prénom", "Classe"]);
  assert.deepEqual(table.rows[1], ["DURAND", "Tom", "4B"]);
  // Ligne incomplète complétée à la largeur de l'en-tête.
  assert.deepEqual(table.rows[2], ["PETIT", "Inès", ""]);
});

test("texte sans tableau inchangé", () => {
  const text = "Bonjour,\nAucun tableau | ici.";
  assert.deepEqual(splitMessageSegments(text), [{ type: "text", text }]);
  assert.equal(messageTables(text).length, 0);
});
