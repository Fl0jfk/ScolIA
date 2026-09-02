import assert from "node:assert/strict";
import { personMatchesSearchQuery } from "@/app/lib/person-name-search";

const margot = { nom: "Simon", prenom: "Margot" };

assert.equal(personMatchesSearchQuery(margot, "Margot"), true);
assert.equal(personMatchesSearchQuery(margot, "margot s"), true);
assert.equal(personMatchesSearchQuery(margot, "Margot Si"), true);
assert.equal(personMatchesSearchQuery(margot, "Margot Simon"), true);
assert.equal(personMatchesSearchQuery(margot, "Simon Margot"), true);
assert.equal(personMatchesSearchQuery(margot, "Simon"), true);
assert.equal(personMatchesSearchQuery(margot, "Dupont"), false);

console.log("person-name-search ok");
