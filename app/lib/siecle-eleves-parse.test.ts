import assert from "node:assert/strict";
import test from "node:test";
import {
  isDateSortiePassee,
  normalizeSiecleDate,
  parseSiecleElevesXmlServer,
  todayIsoLocal,
} from "./siecle-eleves-parse";

test("normalizeSiecleDate FR et ISO", () => {
  assert.equal(normalizeSiecleDate("31/08/2026"), "2026-08-31");
  assert.equal(normalizeSiecleDate("2026-08-31"), "2026-08-31");
  assert.equal(normalizeSiecleDate(""), "");
});

test("isDateSortiePassee — strictement avant aujourd'hui", () => {
  const now = new Date(2026, 8, 11); // 11 sept 2026 local
  assert.equal(todayIsoLocal(now), "2026-09-11");
  assert.equal(isDateSortiePassee("31/08/2026", now), true);
  assert.equal(isDateSortiePassee("2026-08-31", now), true);
  assert.equal(isDateSortiePassee("11/09/2026", now), false); // jour J : encore là
  assert.equal(isDateSortiePassee("12/09/2026", now), false); // futur
  assert.equal(isDateSortiePassee("", now), false);
  assert.equal(isDateSortiePassee(undefined, now), false);
});

test("parse Siècle : sortis → Externe, actifs avec CODE_REGIME canonique", () => {
  const now = new Date(2026, 8, 11);
  const xml = `<?xml version="1.0"?>
<BEE_ELEVES>
  <ELEVE ELEVE_ID="1">
    <NOM_DE_FAMILLE>SORTI</NOM_DE_FAMILLE><PRENOM>Alice</PRENOM>
    <ID_NATIONAL>AAA</ID_NATIONAL><CODE_REGIME>2</CODE_REGIME>
    <DATE_SORTIE>31/08/2026</DATE_SORTIE>
  </ELEVE>
  <ELEVE ELEVE_ID="2">
    <NOM_DE_FAMILLE>ACTIF</NOM_DE_FAMILLE><PRENOM>Bob</PRENOM>
    <ID_NATIONAL>BBB</ID_NATIONAL><CODE_REGIME>2</CODE_REGIME>
  </ELEVE>
  <ELEVE ELEVE_ID="3">
    <NOM_DE_FAMILLE>DP</NOM_DE_FAMILLE><PRENOM>Carole</PRENOM>
    <ID_NATIONAL>CCC</ID_NATIONAL><CODE_REGIME>1</CODE_REGIME>
    <DATE_SORTIE>30/06/2027</DATE_SORTIE>
  </ELEVE>
</BEE_ELEVES>`;

  const parsed = parseSiecleElevesXmlServer(xml, now);
  assert.equal(parsed.totalInFile, 3);
  assert.equal(parsed.skippedSortis, 1);
  assert.equal(parsed.total, 2);
  assert.equal(parsed.sortis.length, 1);
  assert.equal(parsed.sortis[0]?.regime, "Externe");
  assert.equal(parsed.sortis[0]?.nom, "SORTI");
  assert.equal(parsed.internesCount, 1); // Bob seulement
  assert.equal(parsed.eleves.find((e) => e.nom === "ACTIF")?.regime, "Interne");
  assert.equal(parsed.eleves.find((e) => e.nom === "DP")?.regime, "Demi-pension");
  assert.equal(parsed.withRegimeCount, 2);
  assert.equal(parsed.siecleEleveIdMap["1"], undefined);
  assert.equal(parsed.siecleEleveIdMap["2"], "BBB");
});
