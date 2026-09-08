import assert from "node:assert/strict";
import test from "node:test";
import {
  classeMappingKey,
  guessSiteKindForClass,
  resolveMappedSiteId,
  suggestSiecleCode,
} from "./classe-site-mapping-logic";

test("classeMappingKey compacte JE et divisions rectorat", () => {
  assert.equal(classeMappingKey("JE1  MME BAYEL"), classeMappingKey("JE1MMEBAYEL"));
  assert.equal(classeMappingKey("3ème A"), classeMappingKey("3A"));
});

test("guessSiteKindForClass classe JE à l’école", () => {
  assert.equal(guessSiteKindForClass("JE1 MME BAYEL"), "ecole");
  assert.equal(guessSiteKindForClass("JE4  MME LOURDEL"), "ecole");
  assert.equal(guessSiteKindForClass("5°B"), "college");
  assert.equal(guessSiteKindForClass("2A"), "lycee");
});

test("resolveMappedSiteId privilégie le mapping manuel", () => {
  const mappings = [
    {
      classKey: classeMappingKey("JE1 MME BAYEL"),
      className: "JE1 MME BAYEL",
      siteId: "ecole",
      siecleCode: null,
    },
    {
      classKey: classeMappingKey("3A"),
      className: "3A",
      siteId: "college",
      siecleCode: "3A",
    },
  ];
  assert.equal(resolveMappedSiteId("JE1  MME  BAYEL", mappings), "ecole");
  assert.equal(resolveMappedSiteId("3ème A", mappings), "college");
  assert.equal(resolveMappedSiteId("2A", mappings), null);
});

test("suggestSiecleCode aligne un libellé élève sur le code rectorat", () => {
  const divisions = [
    { code: "3A", libelle: "3ème A" },
    { code: "6B", libelle: "6ème B" },
  ];
  assert.equal(suggestSiecleCode("3ème A", divisions), "3A");
  assert.equal(suggestSiecleCode("6B", divisions), "6B");
  assert.equal(suggestSiecleCode("JE1 MME BAYEL", divisions), null);
});
