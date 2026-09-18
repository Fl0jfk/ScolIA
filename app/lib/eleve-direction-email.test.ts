import assert from "node:assert/strict";
import { test } from "node:test";

import {
  isEstablishmentDirectionEmail,
  sanitizeElevePersonalEmail,
} from "@/app/lib/eleve-direction-email";

test("détecte les mails CE / RNE collège et lycée", () => {
  assert.equal(isEstablishmentDirectionEmail("ce.0762565a@ac-normandie.fr"), true);
  assert.equal(isEstablishmentDirectionEmail("0762565A@ac-normandie.fr"), true);
  assert.equal(isEstablishmentDirectionEmail("ce.0761713z@ac-normandie.fr"), true);
  assert.equal(isEstablishmentDirectionEmail("0762041f@ac-normandie.fr"), true);
});

test("laisse passer un vrai mail élève", () => {
  assert.equal(isEstablishmentDirectionEmail("paul.vasseur@gmail.com"), false);
  assert.equal(sanitizeElevePersonalEmail("paul.vasseur@gmail.com"), "paul.vasseur@gmail.com");
});

test("sanitize vide les mails direction", () => {
  assert.equal(sanitizeElevePersonalEmail("ce.0762565a@ac-normandie.fr"), undefined);
  assert.equal(sanitizeElevePersonalEmail("  CE.0761713Z@AC-NORMANDIE.FR "), undefined);
  assert.equal(sanitizeElevePersonalEmail(""), undefined);
});
