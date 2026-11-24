import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mutAddTokenHash,
  mutAnswers,
  mutApplySignature,
  mutClaimManualReminder,
  mutClaimReminders,
  mutConsumeSigningCode,
  mutReserveSigningCode,
  mutRollbackSigningCode,
  mutSetDocument,
  mutTokenSendFailed,
  mutUnclaimReminders,
  type Mut,
  type WindowInput,
} from "@/app/lib/travels-v2-invitation-mutations";
import {
  addAuxTokenHash,
  addTokenHash,
  generateInviteSecret,
  hashInviteSecret,
  issueSigningCode,
  verifyInviteSecret,
} from "@/app/lib/travels-v2-registration";
import {
  CODE_MAX_ATTEMPTS,
  MAX_AUX_TOKEN_HASHES,
  MAX_TOKEN_HASHES,
  type FamilyInvitation,
} from "@/app/lib/travels-v2-registration-types";

function baseInv(over: Partial<FamilyInvitation> = {}): FamilyInvitation {
  return {
    id: "INV00001",
    tripId: "TRIP0001",
    ine: "I1",
    studentName: "Léa DUPONT",
    email: "p@x.fr",
    createdAt: "2026-11-01T00:00:00Z",
    tokenHashes: [],
    sendCount: 0,
    answers: {},
    documents: {},
    exemptions: {},
    codeSends: [],
    ...over,
  };
}

const WIN: WindowInput = { tripStatus: "INSCRIPTIONS_OUVERTES", openedAt: "2026-11-05T00:00:00Z", deadline: "2026-12-20", todayIso: "2026-12-01" };

/**
 * Moteur de stockage en mémoire qui reproduit le contrat de `mutateInvitation` : chaque mutation est exécutée l'une
 * après l'autre sur la dernière version (comme le verrou advisory + la relecture dans la transaction).
 */
function lockedEngine(initial: FamilyInvitation) {
  let stored = structuredClone(initial);
  let chain: Promise<unknown> = Promise.resolve();
  return {
    get value() {
      return stored;
    },
    mutate<R>(fn: (inv: FamilyInvitation) => Mut<R>): Promise<{ value: R; inv: FamilyInvitation }> {
      const run = chain.then(async () => {
        await new Promise((r) => setImmediate(r)); // l'ordonnanceur peut entrelacer d'autres requêtes ici
        const cur = structuredClone(stored);
        const out = fn(cur);
        if (out.save) stored = cur;
        return { value: out.value, inv: structuredClone(cur) };
      });
      chain = run.catch(() => undefined);
      return run;
    },
  };
}

/** Ancien comportement : lecture au début, écriture complète à la fin, sans verrou (perd des mises à jour). */
function unlockedEngine(initial: FamilyInvitation) {
  let stored = structuredClone(initial);
  return {
    get value() {
      return stored;
    },
    async mutate<R>(fn: (inv: FamilyInvitation) => Mut<R>): Promise<{ value: R; inv: FamilyInvitation }> {
      const cur = structuredClone(stored);
      await new Promise((r) => setImmediate(r));
      const out = fn(cur);
      if (out.save) stored = cur;
      return { value: out.value, inv: structuredClone(cur) };
    },
  };
}

/* ─── Compteur d'essais du code ─── */

function invWithCode(code = "123456") {
  const inv = baseInv();
  const { state } = issueSigningCode(inv.id, new Date("2026-12-01T10:00:00Z"));
  void code;
  inv.code = state;
  return inv;
}

test("B2 le compteur d'essais est garanti (5 au plus) même avec 40 requêtes parallèles", async () => {
  const eng = lockedEngine(invWithCode());
  const now = new Date("2026-12-01T10:01:00Z");
  const results = await Promise.all(
    Array.from({ length: 40 }, (_, i) => eng.mutate((inv) => mutConsumeSigningCode(inv, { input: String(100000 + i), now }))),
  );
  const wrong = results.filter((r) => r.value.kind === "checked" && r.value.check.result === "wrong").length;
  const locked = results.filter((r) => r.value.kind === "checked" && r.value.check.result === "locked").length;
  const none = results.filter((r) => r.value.kind === "checked" && r.value.check.result === "none").length;
  assert.equal(wrong, CODE_MAX_ATTEMPTS - 1, "4 réponses « faux », la 5e verrouille");
  assert.equal(locked, 1);
  assert.equal(none, 40 - CODE_MAX_ATTEMPTS, "ensuite le code n'existe plus");
  assert.equal(eng.value.code, undefined);
});

test("B2 sans verrou, le compteur serait contourné (contre-épreuve du test)", async () => {
  const eng = unlockedEngine(invWithCode());
  const now = new Date("2026-12-01T10:01:00Z");
  const results = await Promise.all(
    Array.from({ length: 40 }, (_, i) => eng.mutate((inv) => mutConsumeSigningCode(inv, { input: String(100000 + i), now }))),
  );
  const wrongOrLocked = results.filter((r) => r.value.kind === "checked" && (r.value.check.result === "wrong" || r.value.check.result === "locked")).length;
  assert.ok(wrongOrLocked > CODE_MAX_ATTEMPTS, "l'ancien schéma laisse passer plus de 5 essais");
});

test("B2 un code juste n'est utilisable qu'une fois, même en parallèle", async () => {
  const inv = baseInv();
  const issued = issueSigningCode(inv.id, new Date("2026-12-01T10:00:00Z"));
  inv.code = issued.state;
  const eng = lockedEngine(inv);
  const now = new Date("2026-12-01T10:01:00Z");
  const results = await Promise.all(
    Array.from({ length: 6 }, () => eng.mutate((cur) => mutConsumeSigningCode(cur, { input: issued.code, now }))),
  );
  const ok = results.filter((r) => r.value.kind === "checked" && r.value.check.result === "ok").length;
  assert.equal(ok, 1);
});

test("B2 la garde refuse sans consommer d'essai (réponses modifiées, déjà signé)", async () => {
  const eng = lockedEngine(invWithCode());
  const now = new Date("2026-12-01T10:01:00Z");
  const r = await eng.mutate((inv) => mutConsumeSigningCode(inv, { input: "000000", now, guard: () => "Vos réponses viennent de changer" }));
  assert.equal(r.value.kind, "guard");
  assert.equal(eng.value.code?.attempts, 0);
});

test("B2 envoi de code : 30 s et 5 par heure tiennent sous requêtes parallèles ; échec d'e-mail = réservation défaite", async () => {
  const eng = lockedEngine(baseInv({ answers: { participates: "oui" } }));
  const now = new Date("2026-12-01T10:00:00Z");
  const results = await Promise.all(
    Array.from({ length: 10 }, () => eng.mutate((inv) => mutReserveSigningCode(inv, { now, objection: false, window: WIN }))),
  );
  const okCount = results.filter((r) => r.value.ok).length;
  assert.equal(okCount, 1, "une seule réservation dans les 30 s");
  const first = results.find((r) => r.value.ok)!.value;
  assert.ok(first.ok);
  if (first.ok) {
    const back = await eng.mutate((inv) => mutRollbackSigningCode(inv, first.reservation));
    assert.equal(back.value, true);
    assert.equal(eng.value.code, undefined);
    assert.equal(eng.value.codeSends.length, 0);
  }
});

test("B2 envoi de code : refusé sans réponse « participe » ou page fermée", async () => {
  const eng = lockedEngine(baseInv());
  const now = new Date("2026-12-01T10:00:00Z");
  const noAnswer = await eng.mutate((inv) => mutReserveSigningCode(inv, { now, objection: false, window: WIN }));
  assert.equal(noAnswer.value.ok, false);
  const closed = await eng.mutate((inv) => mutReserveSigningCode(inv, { now, objection: false, window: { ...WIN, tripStatus: "VALIDE" } }));
  assert.equal(closed.value.ok, false);
});

/* ─── Fusion champ par champ ─── */

test("B2 deux enregistrements parallèles de champs différents sont tous les deux conservés", async () => {
  const eng = lockedEngine(baseInv());
  const apply = (input: unknown) =>
    eng.mutate((inv) => mutAnswers(inv, { input, picnicDayKeys: ["2027-03-20", "2027-03-21"], nowIso: "2026-12-01T10:00:00Z", window: WIN }));
  await Promise.all([
    apply({ participates: "oui" }),
    apply({ phone: "06 12 34 56 78" }),
    apply({ meals: { "2027-03-20": { wants: "oui", type: "vegetarien" } } }),
    apply({ meals: { "2027-03-21": { wants: "non" } } }),
  ]);
  const a = eng.value.answers;
  assert.equal(a.participates, "oui");
  assert.equal(a.phone, "06 12 34 56 78");
  assert.deepEqual(a.meals, { "2027-03-20": { wants: "oui", type: "vegetarien" }, "2027-03-21": { wants: "non" } });
});

test("B2 sans verrou, une mise à jour est perdue (contre-épreuve)", async () => {
  const eng = unlockedEngine(baseInv());
  const apply = (input: unknown) =>
    eng.mutate((inv) => mutAnswers(inv, { input, picnicDayKeys: [], nowIso: "2026-12-01T10:00:00Z", window: WIN }));
  await Promise.all([apply({ participates: "oui" }), apply({ phone: "06 12 34 56 78" })]);
  const a = eng.value.answers;
  assert.ok(!(a.participates === "oui" && a.phone === "06 12 34 56 78"), "l'ancien schéma écrase l'un des deux champs");
});

test("B2 une réponse tardive après signature ou page fermée est refusée", async () => {
  const signed = lockedEngine(baseInv({ signature: { decision: "oui", signedAt: "t", signerName: "A" } }));
  const r = await signed.mutate((inv) => mutAnswers(inv, { input: { participates: "non" }, picnicDayKeys: [], nowIso: "t", window: WIN }));
  assert.equal(r.value.ok, false);
  assert.equal(signed.value.answers.participates, undefined);
});

/* ─── Signature, pièces ─── */

test("B2 une seule signature par invitation, même en parallèle", async () => {
  const eng = lockedEngine(baseInv());
  const results = await Promise.all(
    [1, 2, 3].map((n) =>
      eng.mutate((inv) => mutApplySignature(inv, { signature: { decision: "oui", signedAt: `t${n}`, signerName: `P${n}` } }, WIN)),
    ),
  );
  assert.equal(results.filter((r) => r.value.ok).length, 1);
});

test("B2 le dépôt d'une pièce ne touche pas aux réponses écrites entre-temps", async () => {
  const eng = lockedEngine(baseInv());
  await Promise.all([
    eng.mutate((inv) => mutAnswers(inv, { input: { participates: "oui" }, picnicDayKeys: [], nowIso: "t", window: WIN })),
    eng.mutate((inv) =>
      mutSetDocument(inv, "d1", { fileName: "a.pdf", key: "k1", size: 10, contentType: "application/pdf", uploadedAt: "t" }, () => true),
    ),
  ]);
  assert.equal(eng.value.answers.participates, "oui");
  assert.equal(eng.value.documents.d1?.key, "k1");
});

/* ─── Liens (M5) ─── */

test("M5 les liens auxiliaires (relances, copies) n'évincent jamais un lien principal", () => {
  const main: string[] = [];
  const secrets: string[] = [];
  let hashes: string[] = [];
  for (let i = 0; i < MAX_TOKEN_HASHES; i++) {
    const s = generateInviteSecret();
    secrets.push(s);
    hashes = addTokenHash(hashes, hashInviteSecret(s));
    main.push(s);
  }
  let aux: string[] = [];
  for (let i = 0; i < MAX_AUX_TOKEN_HASHES + 4; i++) aux = addAuxTokenHash(aux, hashInviteSecret(generateInviteSecret()));
  assert.equal(aux.length, MAX_AUX_TOKEN_HASHES);
  for (const s of main) assert.ok(verifyInviteSecret(hashes, s), "tous les liens principaux restent valables");
  assert.equal(MAX_TOKEN_HASHES, 10, "10 emplacements pour les liens principaux");
});

test("M5 mutAddTokenHash : un lien auxiliaire va dans son propre emplacement ; un échec d'envoi retire l'empreinte", async () => {
  const eng = lockedEngine(baseInv({ tokenHashes: ["h-main"] }));
  await eng.mutate((inv) => mutAddTokenHash(inv, "h-aux", true));
  assert.deepEqual(eng.value.tokenHashes, ["h-main"]);
  assert.deepEqual(eng.value.auxTokenHashes, ["h-aux"]);
  await eng.mutate((inv) => mutTokenSendFailed(inv, "h-aux", "SMTP"));
  assert.deepEqual(eng.value.auxTokenHashes, []);
  assert.equal(eng.value.lastSendError, "SMTP");
});

/* ─── Relances : réservation atomique (M6) ─── */

test("M6 une relance n'est réservée qu'une fois, même si deux serveurs se croisent", async () => {
  const eng = lockedEngine(baseInv());
  const results = await Promise.all(
    [1, 2, 3, 4].map(() => eng.mutate((inv) => mutClaimReminders(inv, ["deadline_j1"], "2026-12-19"))),
  );
  assert.equal(results.filter((r) => r.value.claimed).length, 1);
  const claim = results.find((r) => r.value.claimed)!.value;
  await eng.mutate((inv) => mutUnclaimReminders(inv, ["deadline_j1"], "2026-12-19", claim.before));
  assert.equal(eng.value.reminders?.deadline_j1, undefined, "e-mail non parti : la réservation est défaite");
  const again = await eng.mutate((inv) => mutClaimReminders(inv, ["deadline_j1"], "2026-12-19"));
  assert.equal(again.value.claimed, true);
});

test("M6 la relance manuelle est limitée à une par jour sous verrou", async () => {
  const eng = lockedEngine(baseInv());
  const results = await Promise.all([1, 2, 3].map(() => eng.mutate((inv) => mutClaimManualReminder(inv, "2026-12-19"))));
  assert.equal(results.filter((r) => r.value.claimed).length, 1);
});
