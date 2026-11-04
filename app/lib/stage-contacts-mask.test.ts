import { test } from "node:test";
import assert from "node:assert/strict";
import {
  buildPublicContacts,
  maskPhoneNumber,
  resolveCreateParentEmails,
} from "@/app/lib/stage-contacts-mask";
import {
  buildSiblingStageSummary,
  sortSiblingsCurrentFirst,
} from "@/app/lib/stage-siblings-summary";

test("maskPhoneNumber : 2 premiers et 2 derniers chiffres", () => {
  assert.equal(maskPhoneNumber("06 12 34 56 78"), "06 •• •• •• 78");
  assert.equal(maskPhoneNumber("+33612345678"), "06 •• •• •• 78");
  assert.equal(maskPhoneNumber(""), "");
  assert.equal(maskPhoneNumber(null), "");
  assert.ok(!maskPhoneNumber("0612345678").includes("1234"));
});

test("contacts : seule l'adresse vérifiée (casse/espaces) est en clair et modifiable", () => {
  const c = buildPublicContacts({
    parent1Email: "  Jean.Dupont@Gmail.com ",
    parent1Phone: "0612345678",
    parent2Email: "marie@orange.fr",
    parent2Phone: "0698765432",
    verifiedEmails: ["jean.dupont@gmail.com"],
  });
  assert.equal(c.length, 2);
  assert.equal(c[0]!.editable, true);
  assert.equal(c[0]!.email, "Jean.Dupont@Gmail.com");
  assert.equal(c[1]!.editable, false);
  assert.equal(c[1]!.email, undefined);
  assert.ok(!JSON.stringify(c[1]).includes("marie@orange.fr"));
  assert.ok(!JSON.stringify(c).includes("0612345678"));
  assert.ok(c[1]!.emailMasked.includes("***"));
});

test("contacts : preuve sans verifiedEmails → rien de modifiable ni en clair", () => {
  const c = buildPublicContacts({ parent1Email: "a@b.fr", parent1Phone: "0612345678" });
  assert.equal(c[0]!.editable, false);
  assert.equal(c[0]!.email, undefined);
  assert.equal(buildPublicContacts({ parent1Email: "a@b.fr", verifiedEmails: [] })[0]!.editable, false);
});

test("create : modification acceptée seulement si le mail du registre est vérifié et le format valide", () => {
  const v = ["p1@x.fr"];
  assert.deepEqual(
    resolveCreateParentEmails({ registryParent1: "P1@x.fr", requestedParent1: "new@x.fr", verifiedEmails: v }),
    { parent1: "new@x.fr", parent2: undefined },
  );
  // slot 2 non vérifié : valeur reçue ignorée
  assert.equal(
    resolveCreateParentEmails({ registryParent2: "p2@x.fr", requestedParent2: "evil@x.fr", verifiedEmails: v }).parent2,
    "p2@x.fr",
  );
  // format invalide : on garde le registre
  assert.equal(
    resolveCreateParentEmails({ registryParent1: "p1@x.fr", requestedParent1: "pas-un-mail", verifiedEmails: v }).parent1,
    "p1@x.fr",
  );
  // slot vide au registre : pas d'ajout
  assert.equal(
    resolveCreateParentEmails({ registryParent2: "", requestedParent2: "add@x.fr", verifiedEmails: v }).parent2,
    undefined,
  );
  // preuve ancienne
  assert.equal(
    resolveCreateParentEmails({ registryParent1: "p1@x.fr", requestedParent1: "new@x.fr" }).parent1,
    "p1@x.fr",
  );
});

test("stageSummary : périodes choisies avec entreprise, aucun jeton, conventions inactives écartées", () => {
  const s = buildSiblingStageSummary({
    availablePeriods: [
      { id: "p1", label: "Stage 1", periodStart: "2027-01-10", periodEnd: "2027-01-14", used: true, conventionId: "c1" },
      { id: "p2", label: "Stage 2", periodStart: "2027-03-01", periodEnd: "2027-03-05", used: false },
    ],
    conventions: [
      { id: "c1", status: "signed", statusLabel: "Convention signée", companyName: "ACME", studentAccessToken: "SECRET" } as never,
      { id: "c2", status: "cancelled", companyName: "X" },
    ],
  });
  assert.equal(s.periods[0]!.chosen, true);
  assert.equal(s.periods[0]!.companyName, "ACME");
  assert.equal(s.periods[1]!.chosen, false);
  assert.equal(s.conventions.length, 1);
  assert.ok(!JSON.stringify(s).includes("SECRET"));
});

test("tri : enfant courant d'abord, puis prénom", () => {
  const l = sortSiblingsCurrentFirst([
    { firstName: "Zoé", lastName: "A", current: false },
    { firstName: "Éva", lastName: "A", current: false },
    { firstName: "Max", lastName: "A", current: true },
  ]);
  assert.deepEqual(l.map((x) => x.firstName), ["Max", "Éva", "Zoé"]);
});

import { maskEmailAddress } from "@/app/lib/stage-identity-otp";
import {
  buildEmergencyContacts,
  isStrictEmailFormat,
  redactConventionForPublicStudent,
  restoreStoredParentContacts,
} from "@/app/lib/stage-contacts-mask";
import { filterStageSiblings } from "@/app/lib/stage-siblings-filter";
import type { StageConvention } from "@/app/lib/stage-types";

test("maskEmailAddress : au moins la moitié masquée", () => {
  const m = maskEmailAddress("jeandup@x.fr");
  assert.ok(!m.includes("jeandup"));
  assert.ok(m.startsWith("j***"));
  const local = maskEmailAddress("jean.dupont@gmail.com").split("@")[0]!;
  assert.ok(local.replace("***", "").length <= Math.floor("jean.dupont".length / 2));
});

test("maskPhoneNumber : +33 converti, moins de 8 chiffres → 2 derniers seulement", () => {
  assert.equal(maskPhoneNumber("+33 6 12 34 56 78"), "06 •• •• •• 78");
  assert.equal(maskPhoneNumber("0033612345678"), "06 •• •• •• 78");
  assert.equal(maskPhoneNumber("123456"), "•• •• 56");
});

test("validation stricte : refuse * et •", () => {
  assert.equal(isStrictEmailFormat("a.b@x.fr"), true);
  assert.equal(isStrictEmailFormat("j***p@x***.fr"), false);
  assert.equal(isStrictEmailFormat("a•b@x.fr"), false);
  assert.equal(
    resolveCreateParentEmails({ registryParent1: "p@x.fr", requestedParent1: "j***p@x.fr", verifiedEmails: ["p@x.fr"] }).parent1,
    "p@x.fr",
  );
});

test("redactConventionForPublicStudent masque les contacts parents et retire les secrets", () => {
  const conv = {
    student: { firstName: "A", lastName: "B", className: "3e", level: "3e", email: "eleve@x.fr", parent1Email: "papa@x.fr", parent2Email: "maman@x.fr", parentEmail: "papa@x.fr" },
    parentSignerEmail: "papa@x.fr",
    parent2SignerEmail: "maman@x.fr",
    parentEmailVerification: { email: "papa@x.fr", code: "123456", sentAt: "" },
    company: { name: "ACME", tutorEmail: "tuteur@acme.fr" },
    signatures: [
      { id: "1", role: "parent", signEmail: "papa@x.fr", signToken: "T", signConfirmCode: "999" },
      { id: "2", role: "tuteur_entreprise", signEmail: "tuteur@acme.fr", signToken: "T2" },
    ],
  } as unknown as StageConvention;
  const out = redactConventionForPublicStudent(conv);
  const json = JSON.stringify(out);
  for (const secret of ["papa@x.fr", "maman@x.fr", "eleve@x.fr", "123456", "999", '"T"', "T2"]) {
    assert.ok(!json.includes(secret), secret);
  }
  assert.equal(out.signatures[1]!.signEmail, "tuteur@acme.fr");
  assert.equal(conv.student.parent1Email, "papa@x.fr"); // entrée non mutée
});

test("restoreStoredParentContacts ignore les valeurs du client", () => {
  const stored = { student: { parent1Email: "papa@x.fr", email: "e@x.fr" }, parentSignerEmail: "papa@x.fr" } as unknown as StageConvention;
  const sent = { student: { parent1Email: "p***", email: "evil@x.fr", parent2Email: "new@x.fr" }, parentSignerEmail: "p***", parent2SignerEmail: "new@x.fr" } as unknown as StageConvention;
  const out = restoreStoredParentContacts(sent, stored);
  assert.equal(out.student.parent1Email, "papa@x.fr");
  assert.equal(out.student.email, "e@x.fr");
  assert.equal(out.student.parent2Email, undefined);
  assert.equal(out.parent2SignerEmail, undefined);
});

test("contacts d'urgence : tuteur seulement, parent 2 absent, téléphones vides", () => {
  const p1 = { email: "papa@x.fr", phone: "0612345678" };
  assert.equal(buildEmergencyContacts({ role: "parent", parent1: p1 }), null);
  assert.equal(buildEmergencyContacts({ role: "rh_entreprise", parent1: p1 }), null);
  assert.equal(buildEmergencyContacts({ role: "professeur_referent", parent1: p1 }), null);
  const r = buildEmergencyContacts({
    role: "tuteur_entreprise",
    parent1: p1,
    parent2: { email: "", phone: " " },
    school: { phone: "01 02 03 04 05" },
  })!;
  assert.equal(r.parents.length, 1);
  assert.deepEqual(r.parents[0], { label: "Responsable 1", phone: "0612345678", email: "papa@x.fr" });
  assert.deepEqual(r.school, { phone: "01 02 03 04 05" });
  const only = buildEmergencyContacts({ role: "tuteur_entreprise", parent1: { email: "a@b.fr", phone: "" } })!;
  assert.deepEqual(only.parents, [{ label: "Responsable 1", email: "a@b.fr" }]);
  assert.equal(only.school, undefined);
});

test("école : TPS, PS A, CE2 B exclus ; PSY et 3eA conservés", () => {
  const mk = (id: string, classe: string) =>
    ({ id, ine: id, nom: "N", prenom: "P", folderName: "", classe, status: "inscrit", dateNaissance: "2015-01-01", parentEmail: "p@x.fr" }) as never;
  const res = filterStageSiblings({
    eleves: [mk("1", "TPS"), mk("2", "PS A"), mk("3", "CE2 B"), mk("4", "3eA"), mk("5", "Maternelle")],
    verifiedEmails: ["p@x.fr"],
    isClassEligible: () => true,
  });
  assert.deepEqual(res.map((s) => s.id), ["4"]);
});

test("stageSummary : companyName « — » ignoré", () => {
  const s = buildSiblingStageSummary({
    availablePeriods: [{ id: "p", label: "S", periodStart: "", periodEnd: "", used: true, conventionId: "c" }],
    conventions: [{ id: "c", status: "draft", companyName: "—" }],
  });
  assert.equal(s.periods[0]!.companyName, undefined);
  assert.equal(s.conventions[0]!.companyName, undefined);
});

import { disambiguateMaskedLabels } from "@/app/lib/stage-contacts-mask";

test("masques identiques distingués sans révéler plus de caractères", () => {
  const a = maskEmailAddress("paul@gmail.com");
  const b = maskEmailAddress("pauline@gmail.com");
  const out = disambiguateMaskedLabels([a, b, maskEmailAddress("autre@orange.fr")]);
  if (a === b) {
    assert.equal(out[0], `${a} (adresse 1)`);
    assert.equal(out[1], `${b} (adresse 2)`);
  }
  assert.equal(new Set(out).size, 3);
  assert.deepEqual(disambiguateMaskedLabels(["x", "x", "y"]), ["x (adresse 1)", "x (adresse 2)", "y"]);
});

test("école : CM1A, CE2B, CP1 exclus ; CPGE, PSY conservés", () => {
  const mk = (id: string, classe: string) =>
    ({ id, ine: id, nom: "N", prenom: "P", folderName: "", classe, status: "inscrit", dateNaissance: "2015-01-01", parentEmail: "p@x.fr" }) as never;
  const res = filterStageSiblings({
    eleves: [mk("1", "CM1A"), mk("2", "CE2B"), mk("3", "CP1"), mk("4", "CPGE"), mk("5", "PSY")],
    verifiedEmails: ["p@x.fr"],
    isClassEligible: () => true,
  });
  assert.deepEqual(res.map((s) => s.id).sort(), ["4", "5"]);
});

test("redact : champs internes retirés, motif de refus conservé", () => {
  const conv = {
    student: { firstName: "A", lastName: "B", className: "3e", level: "3e" },
    teacherReferent: { name: "Prof", email: "prof@x.fr", userId: "u1" },
    discussion: { messages: [{ text: "secret interne" }] },
    history: [{ at: "", by: "staff@x.fr", action: "X" }],
    oneDriveFiling: { folderPath: "/secret" },
    eleveDossierFiling: { s3Key: "k/secret" },
    uploadedPdf: { s3Key: "k/pdf", fileName: "c.pdf", uploadedAt: "" },
    ocrMeta: { extractedAt: "", matchedEleveIne: "INE123" },
    adminReview: { at: "t", by: "uid", byName: "Mme Admin", approved: false, note: "À corriger" },
    status: "admin_rejected",
    signatures: [],
  } as unknown as StageConvention;
  const out = redactConventionForPublicStudent(conv);
  const json = JSON.stringify(out);
  for (const secret of ["prof@x.fr", "secret interne", "staff@x.fr", "/secret", "k/pdf", "INE123", "Mme Admin", "uid"]) {
    assert.ok(!json.includes(secret), secret);
  }
  assert.equal(out.adminReview?.note, "À corriger");
  assert.equal(out.teacherReferent.name, "Prof");
  assert.equal(out.uploadedPdf?.fileName, "c.pdf");
});

test("non-régression écriture : normalizeConventionInput puis restoreStoredParentContacts", async () => {
  let normalize: ((raw: unknown, base?: StageConvention) => StageConvention) | undefined;
  try {
    ({ normalizeConventionInput: normalize } = await import("@/app/lib/stage-workflow"));
  } catch {
    normalize = undefined;
  }
  if (!normalize) {
    return; // stage-workflow non importable sans DB : couvert par le test restoreStoredParentContacts ci-dessus
  }
  const stored = normalize({
    student: { firstName: "A", lastName: "B", className: "3e", level: "3e", parent1Email: "papa@x.fr", parent2Email: "maman@x.fr", email: "e@x.fr" },
    parentSignerEmail: "papa@x.fr",
    parent2SignerEmail: "maman@x.fr",
  });
  stored.signatures = [{ id: "s1", role: "parent", label: "P", status: "en_attente" }];
  const sent = redactConventionForPublicStudent(stored);
  sent.signatures = [];
  const next = restoreStoredParentContacts(normalize(sent, stored), stored);
  assert.equal(next.student.parent1Email, "papa@x.fr");
  assert.equal(next.student.parent2Email, "maman@x.fr");
  assert.equal(next.student.email, "e@x.fr");
  assert.equal(next.parentSignerEmail, "papa@x.fr");
  assert.equal(next.parent2SignerEmail, "maman@x.fr");
  assert.equal(next.signatures.length, 1);
});
