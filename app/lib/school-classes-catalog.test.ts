import assert from "node:assert/strict";
import {
  foldSchoolClass,
  resolveSchoolClassQuery,
  schoolClassesMatch,
  spokenClassHasDivision,
} from "@/app/lib/school-classes-catalog";

const known = ["6A", "6B", "6C", "6E", "5A", "3B"];

assert.equal(foldSchoolClass("6ème A"), "6A");
assert.equal(foldSchoolClass("6eme A"), "6A");
assert.equal(foldSchoolClass("6e A"), "6A");
assert.equal(foldSchoolClass("6 A"), "6A");
assert.equal(foldSchoolClass("sixieme a"), "6A");
assert.equal(foldSchoolClass("6°A"), "6A");
assert.equal(foldSchoolClass("6ème"), "6E");
assert.equal(foldSchoolClass("6E"), "6E");

assert.equal(schoolClassesMatch("6A", "6ème A"), true);
assert.equal(schoolClassesMatch("6E", "6ème A"), false);
assert.equal(schoolClassesMatch("6A", "6ème"), false);
assert.equal(schoolClassesMatch("6E", "6ème"), true);

assert.equal(spokenClassHasDivision("6ème A"), true);
assert.equal(spokenClassHasDivision("6ème"), false);

assert.equal(resolveSchoolClassQuery("6ème A", known).match, "6A");
assert.equal(resolveSchoolClassQuery("sixieme a", known).match, "6A");
assert.deepEqual(
  resolveSchoolClassQuery("6ème", known).ambiguous.sort(),
  ["6A", "6B", "6C", "6E"].sort(),
);

console.log("school-classes fold/resolve: ok");
