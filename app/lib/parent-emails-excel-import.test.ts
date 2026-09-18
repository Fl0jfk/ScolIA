import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  groupParentEmailLines,
  type ParentEmailExcelLine,
} from "@/app/lib/parent-emails-excel-import";

describe("groupParentEmailLines", () => {
  it("fusionne les lignes miroir (mêmes mails, rôles inversés)", () => {
    const lines: ParentEmailExcelLine[] = [
      {
        nom: "ROBICHON",
        prenom: "Léna",
        emails: ["a@x.fr", "b@x.fr"],
      },
      {
        nom: "ROBICHON",
        prenom: "Léna",
        emails: ["b@x.fr", "a@x.fr"],
      },
    ];
    const g = groupParentEmailLines(lines);
    assert.equal(g.length, 1);
    assert.equal(g[0]!.foyers.length, 1);
    assert.deepEqual(g[0]!.allEmails.sort(), ["a@x.fr", "b@x.fr"]);
  });

  it("garde deux foyers quand les jeux d’e-mails diffèrent", () => {
    const lines: ParentEmailExcelLine[] = [
      { nom: "BEGUIN", prenom: "Juliette", emails: ["fpbeguin@wanadoo.fr"] },
      { nom: "BEGUIN", prenom: "Juliette", emails: ["marguerie.helene@gmail.com"] },
    ];
    const g = groupParentEmailLines(lines);
    assert.equal(g.length, 1);
    assert.equal(g[0]!.foyers.length, 2);
    assert.equal(g[0]!.allEmails.length, 2);
  });
});
