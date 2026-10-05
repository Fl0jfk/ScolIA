/**
 * Tests garde-fou labo (sans I/O réseau).
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { isLabDatabaseUrl } from "../assert-lab-database.mjs";

describe("isLabDatabaseUrl", () => {
  it("accepte localhost", () => {
    assert.equal(isLabDatabaseUrl("postgresql://scola:x@127.0.0.1:5432/scola"), true);
    assert.equal(isLabDatabaseUrl("postgresql://scola:x@localhost:5432/scola"), true);
  });

  it("accepte host ou db contenant lab", () => {
    assert.equal(
      isLabDatabaseUrl("postgresql://u:p@rdb-lab.fr-par.scw.cloud:5432/scolia"),
      true,
    );
    assert.equal(
      isLabDatabaseUrl("postgresql://u:p@rdb.fr-par.scw.cloud:5432/scolia_lab"),
      true,
    );
  });

  it("refuse une URL prod sans lab", () => {
    assert.equal(
      isLabDatabaseUrl("postgresql://u:p@rdb.fr-par.scw.cloud:5432/scolia"),
      false,
    );
  });
});
