import { test } from "node:test";
import assert from "node:assert/strict";
import { isAccountActivationPending, roleRequiresTwoFactor } from "./two-factor-policy";

test("professeur seul : MFA facultative", () => {
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["professeur"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["enseignant"] }),
    false,
  );
});

test("professeur + parent : MFA facultative", () => {
  assert.equal(
    roleRequiresTwoFactor({
      platformAdmin: false,
      orgAdmin: false,
      roles: ["professeur", "parent"],
    }),
    false,
  );
});

test("professeur + direction : MFA facultative (temporaire)", () => {
  assert.equal(
    roleRequiresTwoFactor({
      platformAdmin: false,
      orgAdmin: false,
      roles: ["professeur", "direction_lycee"],
    }),
    false,
  );
});

test("professeur orgAdmin sans rôle admin : MFA facultative", () => {
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: true, roles: ["professeur"] }),
    false,
  );
});

test("direction / administratif / compta : MFA facultative ; admin seule obligatoire", () => {
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["administratif"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["direction"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["direction_lycee"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["comptabilite"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["admin"] }),
    true,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: true, roles: ["admin"] }),
    true,
  );
});

test("surveillant / CPE / Accueil : MFA facultative", () => {
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["surveillant"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["cpe"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["accueil"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({
      platformAdmin: false,
      orgAdmin: false,
      roles: ["surveillant", "cpe"],
    }),
    false,
  );
});

test("CPE + direction : MFA facultative", () => {
  assert.equal(
    roleRequiresTwoFactor({
      platformAdmin: false,
      orgAdmin: false,
      roles: ["cpe", "direction_college"],
    }),
    false,
  );
});

test("surveillant déjà connecté sans MFA : plus en attente", () => {
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: false,
      twoFactorEnabled: false,
      platformAdmin: false,
      orgAdmin: false,
      roles: ["surveillant"],
    }),
    false,
  );
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: false,
      twoFactorEnabled: false,
      platformAdmin: false,
      orgAdmin: false,
      roles: ["cpe"],
    }),
    false,
  );
});

test("parent / élève seuls : pas de MFA obligatoire", () => {
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["parent"] }),
    false,
  );
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: ["eleve"] }),
    false,
  );
});

test("aucun rôle : MFA seulement si orgAdmin", () => {
  assert.equal(roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: false, roles: [] }), false);
  assert.equal(roleRequiresTwoFactor({ platformAdmin: false, orgAdmin: true, roles: [] }), true);
});

test("platformAdmin : MFA obligatoire", () => {
  assert.equal(
    roleRequiresTwoFactor({ platformAdmin: true, orgAdmin: false, roles: ["professeur"] }),
    true,
  );
});

test("rôle professeur_* : MFA facultative", () => {
  assert.equal(
    roleRequiresTwoFactor({
      platformAdmin: false,
      orgAdmin: false,
      roles: ["professeur_lycee"],
    }),
    false,
  );
});

test("professeur déjà connecté sans MFA : plus en attente", () => {
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: false,
      twoFactorEnabled: false,
      platformAdmin: false,
      orgAdmin: false,
      roles: ["professeur"],
    }),
    false,
  );
});

test("professeur avec MFA déjà activée : pas en attente (on la laisse)", () => {
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: false,
      twoFactorEnabled: true,
      platformAdmin: false,
      orgAdmin: false,
      roles: ["professeur"],
    }),
    false,
  );
});

test("direction / administratif sans MFA : plus en attente", () => {
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: false,
      twoFactorEnabled: false,
      platformAdmin: false,
      orgAdmin: false,
      roles: ["administratif"],
    }),
    false,
  );
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: false,
      twoFactorEnabled: false,
      platformAdmin: false,
      orgAdmin: false,
      roles: ["direction_lycee"],
    }),
    false,
  );
});

test("admin sans MFA : toujours en attente", () => {
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: false,
      twoFactorEnabled: false,
      platformAdmin: false,
      orgAdmin: true,
      roles: ["admin"],
    }),
    true,
  );
});

test("professeur qui n’a pas encore créé son mot de passe : en attente", () => {
  assert.equal(
    isAccountActivationPending({
      emailVerified: true,
      mustChangePassword: true,
      twoFactorEnabled: false,
      platformAdmin: false,
      orgAdmin: false,
      roles: ["professeur"],
    }),
    true,
  );
});
