/**
 * Audit personas « fiches de dialogue » — Postgres local uniquement.
 *
 * Lancement (PowerShell) :
 *   $env:TEST_DATABASE_URL="postgresql://scola:scola_dev_pwd@127.0.0.1:5432/scolia_migrate"
 *   $env:SCOLIA_TEST_MODE="1"
 *   npx tsx --require ./scripts/stub-server-only.cjs scripts/audit-fiches-dialogue-personas-v2.ts
 *
 * Aucun e-mail réel, aucun S3 réel, aucune connexion prod.
 */
import { randomUUID } from "node:crypto";
import { and, desc, eq, isNull } from "drizzle-orm";
import { beginTestDatabase, endTestDatabase } from "@/app/lib/test-database-harness";
import type { FdFicheStatut } from "@/db/schema-fiches-dialogue";

type Niveau = "6e" | "5e" | "4e" | "3e" | "2nde" | "1re" | "Tle";
type Starter = "conseil_dabord" | "famille_dabord";

type Obs = {
  scenario: string;
  persona: string;
  action: string;
  before: string;
  after: string;
  ok: boolean;
  detail: string;
  ux?: string;
  /** « gap » = manque produit confirmé, pas une assertion de test en échec. */
  kind?: "test" | "gap";
};

const obs: Obs[] = [];
function record(o: Obs) {
  obs.push({ kind: "test", ...o });
  const marker = o.kind === "gap" ? "-- " : o.ok ? "OK " : "KO ";
  console.log(
    `${marker}| ${o.scenario} | ${o.persona} | ${o.action} | ${o.before} -> ${o.after} | ${o.detail}`,
  );
}

function unique(prefix: string): string {
  return `${prefix}-${randomUUID().slice(0, 8)}`;
}

async function main() {
  const fakeTest = {
    skip(reason: string) {
      throw new Error(reason);
    },
  } as unknown as import("node:test").TestContext;
  if (!beginTestDatabase(fakeTest)) {
    throw new Error("TEST_DATABASE_URL absente ou refusée");
  }

  const { getDb, closeDb } = await import("@/db/index");
  const wf = await import("@/app/lib/fiches-dialogue-workflow");
  const tokens = await import("@/app/lib/fiches-dialogue-tokens");
  const access = await import("@/app/lib/fiches-dialogue-access");
  const schema = await import("@/db/schema");
  const db = getDb();

  const etablissementId = await (async () => {
    const [row] = await db
      .insert(schema.etablissement)
      .values({ slug: unique("fd-audit"), name: "Établissement audit personas" })
      .returning();
    return row.id;
  })();

  const P1 = "parent1.audit@example.test";
  const P2 = "parent2.audit@example.test";
  const PP_SIG = { role: "professeur_principal" as const, name: "Mme Prof Principale", method: "saved" };
  const DIR_SIG = { role: "direction" as const, name: "M. Le Directeur", method: "saved" };

  async function createEleve(label: string, classe: string) {
    const [row] = await db
      .insert(schema.eleve)
      .values({
        etablissementId,
        sourceKey: unique(`audit:${label}`),
        nom: `NOM-${label}`,
        prenom: `Prenom-${label}`,
        folderName: unique(`dossier-${label}`),
        classe,
        dateNaissance: "2012-05-10",
      })
      .returning();
    return row.id;
  }

  async function makeCampagne(
    niveau: Niveau,
    starter: Starter,
    opts?: { templateKey?: string; appelEnabled?: boolean; closesAt?: Date },
  ) {
    const created = await wf.createFdCampagneFromTemplate({
      etablissementId,
      niveauActuel: niveau,
      starterMode: starter,
      templateKey: opts?.templateKey,
      label: `Audit ${niveau} ${starter} ${unique("")}`,
      anneeLabel: "2025-2026",
      delaiFamilleJours: 7,
      contactPpLabel: "École Directe",
      appelConfig: {
        enabled: true,
        dateLimite: "2026-06-30",
        procedureHtml: "<p>Écrire au secrétariat pour constituer le dossier.</p>",
        documentsLabels: ["Lettre de motivation", "Bulletins"],
        contactPpLabel: "École Directe",
      },
      etapesDates: opts?.closesAt
        ? createdEtapesDates(opts.closesAt)
        : undefined,
    });
    await wf.updateFdCampagne(etablissementId, created.campagne.id, { statut: "active" });
    if (opts?.appelEnabled === false) {
      const [camp] = await db
        .select()
        .from(schema.fdCampagne)
        .where(eq(schema.fdCampagne.id, created.campagne.id));
      await db
        .update(schema.fdCampagne)
        .set({ appelConfig: { ...camp.appelConfig, enabled: false } })
        .where(eq(schema.fdCampagne.id, created.campagne.id));
    }
    return created;
  }

  function createdEtapesDates(closesAt: Date) {
    return [1, 2, 3, 4, 5, 6].map((ordre) => ({ ordre, closesAt }));
  }

  async function makeFiche(
    campagneId: string,
    etapeId: string,
    statut: FdFicheStatut,
    opts?: { parents?: string[]; label?: string; classe?: string },
  ) {
    const label = opts?.label ?? unique("eleve");
    const classe = opts?.classe ?? "5e A";
    const eleveId = await createEleve(label, classe);
    const [fiche] = await db
      .insert(schema.fdFiche)
      .values({
        etablissementId,
        campagneId,
        eleveId,
        eleveNom: `NOM-${label}`,
        elevePrenom: `Prenom-${label}`,
        classeActuelle: classe,
        eleveDateNaissance: "2012-05-10",
        optionsActuelles: [],
        parentEmails: opts?.parents ?? [P1],
        statut,
        etapeCouranteId: etapeId,
      })
      .returning();
    return fiche;
  }

  async function statutOf(ficheId: string) {
    const f = await wf.getFdFiche(etablissementId, ficheId);
    return f?.statut ?? "introuvable";
  }

  async function etapeOf(ficheId: string) {
    const f = await wf.getFdFiche(etablissementId, ficheId);
    return f?.etapeCouranteId ?? null;
  }

  async function familyReply(
    ficheId: string,
    etapeId: string,
    email: string,
    values: Record<string, string | string[] | boolean | null>,
    comment?: string,
  ) {
    return wf.submitFdFamilleReponse({
      etablissementId,
      ficheId,
      etapeId,
      payload: { values, comment },
      auteurLabel: "Parent audit",
      signature: { name: "Parent audit", email, method: "saved" },
    });
  }

  async function publishConseil(
    ficheId: string,
    etapeId: string,
    payload: { avis: "favorable" | "reserve" | "defavorable" | "autre"; destinationProposee?: string; motif?: string; commentaire?: string },
    opts?: { publish?: boolean; avenant?: boolean; signatures?: Array<{ role: "professeur_principal" | "direction"; name: string; method?: string }> },
  ) {
    return wf.submitFdConseilDecision({
      etablissementId,
      ficheId,
      etapeId,
      payload,
      signatures: opts?.signatures ?? [PP_SIG, DIR_SIG],
      publish: opts?.publish ?? true,
      avenant: opts?.avenant,
    });
  }

  async function latestToken(ficheId: string, purpose?: string) {
    const conditions = [eq(schema.fdToken.ficheId, ficheId), isNull(schema.fdToken.revokedAt)];
    if (purpose) conditions.push(eq(schema.fdToken.purpose, purpose));
    const [row] = await db
      .select()
      .from(schema.fdToken)
      .where(and(...conditions))
      .orderBy(desc(schema.fdToken.createdAt))
      .limit(1);
    return row ?? null;
  }

  async function activeTokens(ficheId: string, purpose: string) {
    return db
      .select()
      .from(schema.fdToken)
      .where(
        and(
          eq(schema.fdToken.ficheId, ficheId),
          eq(schema.fdToken.purpose, purpose),
          isNull(schema.fdToken.revokedAt),
        ),
      )
      .orderBy(desc(schema.fdToken.createdAt));
  }

  async function confirmParent2(ficheId: string, decision: "confirme" | "contredit", motif?: string) {
    const token = await latestToken(ficheId, "accord_parent2");
    if (!token?.email) return { ok: false as const, error: "jeton 2e parent absent" };
    return wf.submitFdParentAccord({
      etablissementId,
      ficheId,
      email: token.email,
      decision,
      motif,
      conflictValues: decision === "contredit" ? { destination: "5e" } : undefined,
    });
  }

  try {
    // ---------------------------------------------------------------------
    // S1 — Parcours nominal collège : famille d'abord, 1 parent, acceptation
    // ---------------------------------------------------------------------
    {
      const s = "S1 nominal collège";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const [saisie, conseil, choix, decisionFinale, acceptation, appel] = etapes;
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s1" });

      const send = await wf.sendFdCampagneEtapeToFamilles({
        etablissementId,
        campagneId: campagne.id,
      });
      const afterSend = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Secrétariat", action: "Envoyer le lien initial",
        before: "a_envoyer", after: await statutOf(fiche.id),
        ok: send.sent === 0 && send.failed === 1 && afterSend?.lastSentAt === null,
        detail: `sent=${send.sent}, failed=${send.failed}, lastSentAt=${afterSend?.lastSentAt ? "renseigné" : "null"}`,
        ux: "SMTP absent : l’échec est désormais compté (failed=1), la fiche reste « à envoyer » et la date d’envoi n’est pas falsifiée.",
      });

      const t1 = await latestToken(fiche.id, "saisie");
      if (!t1) throw new Error("S1 : jeton saisie absent");
      const ctx = await wf.getFdPublicContext(t1.token);
      record({
        scenario: s, persona: "Parent 1", action: "Ouvrir le lien",
        before: "en_attente_famille", after: ctx ? `${ctx.etape.kind} / open=${ctx.campagneOpen}` : "contexte null",
        ok: Boolean(ctx && ctx.etape.id === saisie.id && ctx.campagneOpen),
        detail: `etape=${ctx?.etape.label ?? "—"}`,
      });

      const r1 = await familyReply(fiche.id, saisie.id, P1, { destination: "5e" }, "Nous souhaitons le passage en 5e.");
      record({
        scenario: s, persona: "Parent 1", action: "Déposer les vœux",
        before: "en_attente_famille", after: await statutOf(fiche.id), ok: r1.ok,
        detail: r1.ok ? "réponse enregistrée" : r1.error,
      });

      const adv1 = await wf.advanceCampagneToConseil({
        etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id,
      });
      record({
        scenario: s, persona: "Direction", action: "Geler et transmettre au conseil",
        before: "saisie_recue", after: await statutOf(fiche.id), ok: adv1.advanced === 1,
        detail: `advanced=${adv1.advanced}, skipped=${adv1.skipped.length}`,
      });

      const draft = await publishConseil(fiche.id, conseil.id, { avis: "reserve", destinationProposee: "5e", motif: "Avis favorable sous réserve." }, { publish: false, signatures: [PP_SIG] });
      record({
        scenario: s, persona: "Professeur principal", action: "Rédiger l’avis (brouillon)",
        before: "en_conseil", after: await statutOf(fiche.id), ok: draft.ok && (await statutOf(fiche.id)) === "en_attente_direction",
        detail: draft.ok ? "brouillon PP enregistré" : draft.error,
      });

      const pub1 = await publishConseil(fiche.id, conseil.id, { avis: "favorable", destinationProposee: "5e" });
      record({
        scenario: s, persona: "Direction", action: "Publier l’avis provisoire",
        before: "en_attente_direction", after: await statutOf(fiche.id), ok: pub1.ok,
        detail: pub1.ok ? "avis publié + envoi auto de l’étape suivante" : pub1.error,
      });

      const r2 = await familyReply(fiche.id, choix.id, P1, { destination: "5e" }, "Nous confirmons.");
      record({
        scenario: s, persona: "Parent 1", action: "Confirmer au 2e semestre",
        before: "en_attente_famille", after: await statutOf(fiche.id), ok: r2.ok,
        detail: r2.ok ? "choix définitifs enregistrés" : r2.error,
      });

      const adv2 = await wf.advanceCampagneToConseil({
        etablissementId, campagneId: campagne.id, fromEtapeId: choix.id,
      });
      record({
        scenario: s, persona: "Direction", action: "Transmettre la décision finale au conseil",
        before: "saisie_recue", after: await statutOf(fiche.id), ok: adv2.advanced === 1,
        detail: `advanced=${adv2.advanced}, skipped=${adv2.skipped.length}`,
      });

      const pub2 = await publishConseil(fiche.id, decisionFinale.id, { avis: "favorable", destinationProposee: "5e" });
      record({
        scenario: s, persona: "Direction", action: "Publier la décision définitive",
        before: "en_conseil", after: await statutOf(fiche.id), ok: pub2.ok,
        detail: pub2.ok ? "décision finale publiée + demande d’acceptation" : pub2.error,
      });

      const acc = await wf.submitFdAcceptation({
        etablissementId,
        ficheId: fiche.id,
        etapeId: acceptation.id,
        payload: { accepte: true },
        auteurLabel: "Parent audit",
        signature: { name: "Parent audit", email: P1, method: "saved" },
      });
      const fFinal = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Parent 1", action: "Accepter la décision",
        before: "en_attente_acceptation", after: `${fFinal?.statut} / closed=${Boolean(fFinal?.closedAt)}`, ok: acc.ok && fFinal?.statut === "acceptee",
        detail: acc.ok ? `étape=${appel ? "appel disponible" : "—"}` : acc.error,
      });

      const archives = await db.select().from(schema.fdPdfArchive).where(eq(schema.fdPdfArchive.ficheId, fiche.id));
      record({
        scenario: s, persona: "Admin", action: "Vérifier les archives PDF",
        before: "—", after: `${archives.length} archive(s)`, ok: archives.length >= 4,
        detail: archives.map((a) => `${a.kind}(${a.pdfBase64 ? "db" : a.s3Key ? "s3" : "none"})`).join(", "),
      });
    }

    // ---------------------------------------------------------------------
    // S2 — Changement d'avis avant le gel (1 parent)
    // ---------------------------------------------------------------------
    {
      const s = "S2 changement d’avis";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s2" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      const t1 = await latestToken(fiche.id, "saisie");
      if (!t1) throw new Error("S2 : jeton absent");

      const first = await familyReply(fiche.id, saisie.id, P1, { destination: "maintien" }, "Nous demandons le maintien.");
      const firstStatut = await statutOf(fiche.id);
      record({
        scenario: s, persona: "Parent 1", action: "Déposer un premier vœu (maintien)",
        before: "en_attente_famille", after: firstStatut, ok: first.ok && firstStatut === "saisie_recue",
        detail: first.ok ? "maintien enregistré" : first.error,
      });

      const ctxAfter = await wf.getFdPublicContext(t1.token);
      record({
        scenario: s, persona: "Parent 1", action: "Réutiliser le lien initial pour changer d’avis",
        before: "saisie_recue", after: ctxAfter ? "contexte encore valide" : "lien invalide",
        ok: ctxAfter !== null && ctxAfter.fiche.statut === "saisie_recue",
        detail: ctxAfter
          ? "le lien initial reste valable jusqu’à la clôture de l’étape : correction possible"
          : "le jeton de saisie a été révoqué après le dépôt",
        ux: "Le parent peut rectifier ses vœux depuis son e-mail initial, sans redemander un code, jusqu’à la date de clôture.",
      });

      const resend = await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      record({
        scenario: s, persona: "Secrétariat", action: "Cliquer « Envoyer aux familles (manquants) »",
        before: "saisie_recue", after: `${resend.sent} envoi(s)`, ok: resend.sent === 0,
        detail: "les fiches saisie_recue sont ignorées : aucun renvoi n’est nécessaire, le lien initial reste ouvert",
        ux: "Le secrétariat n’a pas besoin de renvoyer un lien de correction : le parent garde la main jusqu’à la clôture.",
      });

      const newToken = await tokens.createFdAccessToken({
        etablissementId,
        ficheId: fiche.id,
        etapeId: saisie.id,
        email: P1,
        purpose: "saisie",
        expiresInMinutes: 30,
      });
      const corrected = await familyReply(fiche.id, saisie.id, P1, { destination: "5e" }, "Finalement, passage en 5e.");
      const rows = await db
        .select()
        .from(schema.fdReponse)
        .where(and(eq(schema.fdReponse.ficheId, fiche.id), eq(schema.fdReponse.etapeId, saisie.id), eq(schema.fdReponse.auteurRole, "famille")));
      record({
        scenario: s, persona: "Parent 1", action: "Corriger de secours via un nouveau code (OTP simulé)",
        before: "saisie_recue", after: await statutOf(fiche.id), ok: corrected.ok && rows.length === 1,
        detail: `réponses famille=${rows.length}, payload=${JSON.stringify(rows[0]?.payload ?? null)}`,
        ux: "Si le lien est perdu, l’identification + code reste un filet de sécurité.",
      });
    }

    // ---------------------------------------------------------------------
    // S3 — Deux parents : accord confirmé
    // ---------------------------------------------------------------------
    {
      const s = "S3 deux parents — accord";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1, P2], label: "s3" });
      const ficheP2First = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1, P2], label: "s3b" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      const first = await familyReply(fiche.id, saisie.id, P1, { destination: "5e" });
      const t2 = await latestToken(fiche.id, "accord_parent2");
      record({
        scenario: s, persona: "Parent 1", action: "Déposer la réponse",
        before: "en_attente_famille", after: await statutOf(fiche.id), ok: first.ok && (await statutOf(fiche.id)) === "en_attente_accord_parent2",
        detail: first.ok ? "2e parent notifié" : first.error,
      });
      record({
        scenario: s, persona: "Parent 2", action: "Recevoir le lien de confirmation",
        before: "en_attente_accord_parent2", after: t2 ? `jeton pour ${t2.email}` : "aucun jeton",
        ok: Boolean(t2?.email === P2),
        detail: "un seul jeton d’accord est créé, pour la première adresse différente du déposant",
      });
      const confirm = await confirmParent2(fiche.id, "confirme");
      const f = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Parent 2", action: "Confirmer l’accord",
        before: "en_attente_accord_parent2", after: `${f?.statut} / ${f?.parentAccord?.statutAutre}`,
        ok: confirm.ok && f?.statut === "saisie_recue" && f?.parentAccord?.statutAutre === "confirme",
        detail: confirm.ok ? "accord confirmé" : confirm.error,
      });

      const firstP2 = await familyReply(ficheP2First.id, saisie.id, P2, { destination: "5e" });
      const t2b = await latestToken(ficheP2First.id, "accord_parent2");
      record({
        scenario: s, persona: "Parent 2", action: "Déposer en premier (rôle inversé)",
        before: "en_attente_famille", after: t2b ? `confirmation demandée à ${t2b.email}` : "aucun jeton",
        ok: firstP2.ok && t2b?.email === P1,
        detail: "le dépôt est attribué au parent 2 ; la confirmation part bien vers le parent 1",
      });
      const confirmP1 = await wf.submitFdParentAccord({
        etablissementId, ficheId: ficheP2First.id, email: P1, decision: "confirme",
      });
      const f2 = await wf.getFdFiche(etablissementId, ficheP2First.id);
      record({
        scenario: s, persona: "Parent 1", action: "Confirmer la réponse du parent 2",
        before: "en_attente_accord_parent2", after: `${f2?.statut} / ${f2?.parentAccord?.statutAutre}`,
        ok: confirmP1.ok && f2?.statut === "saisie_recue" && f2?.parentAccord?.statutAutre === "confirme",
        detail: confirmP1.ok ? "rôle inversé validé" : confirmP1.error,
      });
    }

    // ---------------------------------------------------------------------
    // S4 — Deux parents : désaccord, deux modes de résolution
    // ---------------------------------------------------------------------
    {
      const s = "S4 conflit parents";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const f1 = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1, P2], label: "s4a" });
      const f2 = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1, P2], label: "s4b" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      await familyReply(f1.id, saisie.id, P1, { destination: "maintien" });
      await familyReply(f2.id, saisie.id, P1, { destination: "maintien" });
      const c1 = await confirmParent2(f1.id, "contredit", "Pas d’accord avec le maintien.");
      const c2 = await confirmParent2(f2.id, "contredit", "Pas d’accord avec le maintien.");
      record({
        scenario: s, persona: "Parent 2", action: "Contredire la réponse",
        before: "en_attente_accord_parent2", after: `${await statutOf(f1.id)} / ${await statutOf(f2.id)}`,
        ok: c1.ok && c2.ok && (await statutOf(f1.id)) === "en_conflit",
        detail: "les deux fiches passent en conflit",
        ux: "Alerte staff (PP/direction) et information des deux parents tentées ; sans SMTP, l’échec est best-effort et ne bloque pas le conflit.",
      });
      const conflictDossier = await wf.getFdFicheDossier(etablissementId, f1.id);
      record({
        scenario: s, persona: "Professeur principal / direction",
        action: "Lire le désaccord dans le dossier famille",
        before: "en_conflit",
        after: `réponses=${conflictDossier?.reponses.length ?? 0}, motif=${conflictDossier?.fiche.parentAccord?.conflictMotif ? "présent" : "absent"}`,
        ok: Boolean(
          conflictDossier &&
            conflictDossier.reponses.some((r) => r.auteurRole === "famille") &&
            conflictDossier.fiche.parentAccord?.conflictMotif,
        ),
        detail: "le dossier staff expose la réponse déposée et le motif du désaccord, pas seulement un statut",
      });
      const frozen = await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id });
      record({
        scenario: s, persona: "Direction", action: "Geler malgré les conflits",
        before: "en_conflit", after: `advanced=${frozen.advanced}, skipped=${frozen.skipped.length}`,
        ok: frozen.advanced === 0 && frozen.skipped.length === 2,
        detail: frozen.skipped.map((x) => `${x.eleve}: ${x.reason}`).join(" | "),
      });
      const r1 = await wf.resolveFdConflit({ etablissementId, ficheId: f1.id, resolution: "maintien_famille", note: "On conserve la demande de la famille." });
      const r2 = await wf.resolveFdConflit({ etablissementId, ficheId: f2.id, resolution: "avis_conseil", note: "Le conseil tranche." });
      const f1After = await wf.getFdFiche(etablissementId, f1.id);
      const f2After = await wf.getFdFiche(etablissementId, f2.id);
      record({
        scenario: s, persona: "Direction", action: "Résoudre les conflits",
        before: "en_conflit", after: `${f1After?.statut}/${f1After?.parentAccord?.resolution} ; ${f2After?.statut}/${f2After?.parentAccord?.resolution}`,
        ok: r1.ok && r2.ok && f1After?.statut === "saisie_recue" && f2After?.statut === "saisie_recue",
        detail: "maintien_famille et avis_conseil tracés dans parentAccord",
      });
      const adv = await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id });
      record({
        scenario: s, persona: "Direction", action: "Regeler après résolution",
        before: "saisie_recue", after: `advanced=${adv.advanced}, skipped=${adv.skipped.length}`,
        ok: adv.advanced === 2,
        detail: "les deux fiches avancent une fois le conflit résolu",
      });
    }

    // ---------------------------------------------------------------------
    // S5 — Accord tacite du 2e parent
    // ---------------------------------------------------------------------
    {
      const s = "S5 accord tacite";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1, P2], label: "s5" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      await familyReply(fiche.id, saisie.id, P1, { destination: "5e" });
      const t2 = await latestToken(fiche.id, "accord_parent2");
      const f = await wf.getFdFiche(etablissementId, fiche.id);
      await db
        .update(schema.fdFiche)
        .set({
          parentAccord: {
            ...(f?.parentAccord ?? {}),
            accordDeadlineAt: new Date(Date.now() - 60_000).toISOString(),
          },
        })
        .where(eq(schema.fdFiche.id, fiche.id));
      const applied = await wf.applyFdTacitParentAccordIfDue(etablissementId, fiche.id);
      const after = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Système", action: "Appliquer l’accord tacite échu",
        before: "en_attente_accord_parent2", after: `${after?.statut} / ${after?.parentAccord?.statutAutre}`,
        ok: applied && after?.statut === "saisie_recue" && after?.parentAccord?.statutAutre === "tacite",
        detail: "l’accord tacite est bien appliqué",
        ux: "Les deux parents sont informés de l’accord tacite (envoi best-effort : sans SMTP, l’accord reste acquis et l’horodatage de notification n’est pas falsifié).",
      });
      const ctxAfter = t2 ? await wf.getFdPublicContext(t2.token) : null;
      record({
        scenario: s, persona: "Parent 2", action: "Cliquer sur son lien après l’échéance",
        before: "saisie_recue (tacite)", after: ctxAfter ? "contexte encore valide" : "lien invalide",
        ok: Boolean(ctxAfter && ctxAfter.fiche.parentAccord?.statutAutre === "tacite"),
        detail: ctxAfter
          ? `le lien affiche l’accord tacite déjà enregistré (${ctxAfter.fiche.parentAccord?.statutAutre})`
          : "le jeton du 2e parent a été révoqué par l’accord tacite",
        ux: "Le parent lit « accord tacite enregistré » au lieu de « lien invalide » : il sait que son silence a été pris en compte.",
      });
      const late = await wf.submitFdParentAccord({
        etablissementId,
        ficheId: fiche.id,
        email: P2,
        decision: "confirme",
      });
      record({
        scenario: s, persona: "Parent 2", action: "Confirmer après l’échéance (si le lien était encore valide)",
        before: "saisie_recue", after: await statutOf(fiche.id), ok: late.ok,
        detail: late.ok ? "action idempotente : l’accord tacite est déjà enregistré" : `refusé : ${late.error}`,
        ux: "Une confirmation tardive ne rouvre pas le dossier et ne produit pas d’erreur anxiogène.",
      });
    }

    // ---------------------------------------------------------------------
    // S6 — Refus final + appel (redoublement demandé, refusé)
    // ---------------------------------------------------------------------
    {
      const s = "S6 refus + appel";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const [saisie, conseil, choix, decisionFinale, acceptation, appel] = etapes;
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s6" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      await familyReply(fiche.id, saisie.id, P1, { destination: "maintien" }, "Nous demandons le redoublement.");
      await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id });
      await publishConseil(fiche.id, conseil.id, { avis: "reserve", destinationProposee: "5e", motif: "Le conseil propose le passage." });
      await familyReply(fiche.id, choix.id, P1, { destination: "maintien" }, "Nous maintenons notre demande de redoublement.");
      await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: choix.id });
      const pub = await publishConseil(fiche.id, decisionFinale.id, {
        avis: "defavorable",
        destinationProposee: "5e",
        motif: "Le conseil ne retient pas le redoublement et propose le passage en 5e.",
      });
      record({
        scenario: s, persona: "Direction", action: "Publier un avis défavorable au redoublement",
        before: "en_conseil", after: await statutOf(fiche.id), ok: pub.ok,
        detail: "décision finale publiée",
      });
      const refus = await wf.submitFdAcceptation({
        etablissementId, ficheId: fiche.id, etapeId: acceptation.id,
        payload: { accepte: false, motifRefus: "Nous refusons le passage et demandons le redoublement." },
        auteurLabel: "Parent audit",
        signature: { name: "Parent audit", email: P1, method: "saved" },
      });
      const afterRefus = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Parent 1", action: "Refuser la décision et demander l’appel",
        before: "en_attente_acceptation", after: `${afterRefus?.statut} / etape=${afterRefus?.etapeCouranteId === appel.id ? "appel" : "autre"}`,
        ok: refus.ok && afterRefus?.statut === "en_appel",
        detail: refus.ok ? "procédure d’appel déclenchée" : refus.error,
        ux: "L’e-mail d’appel et l’alerte staff sont prévus ; la direction dispose désormais d’un dossier d’appel pour statuer.",
      });
      const retry = await wf.submitFdAcceptation({
        etablissementId, ficheId: fiche.id, etapeId: acceptation.id,
        payload: { accepte: true }, auteurLabel: "Parent audit",
      });
      record({
        scenario: s, persona: "Parent 1", action: "Revenir en arrière après l’appel",
        before: "en_appel", after: await statutOf(fiche.id), ok: !retry.ok,
        detail: retry.ok ? "accepté (inattendu)" : `refusé : ${retry.error}`,
      });
      const saved = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        updatedBy: "Direction audit",
        patch: {
          commissionLabel: "Commission académique d’appel",
          saisineDate: "2026-06-10",
          documents: [
            { label: "Lettre de motivation", statut: "attendu" },
            { label: "Bulletins", statut: "recu", reference: "dossier-2026" },
          ],
          informations: "Les pièces complémentaires seront transmises par la commission.",
        },
      });
      const afterSave = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Direction", action: "Compléter le dossier d’appel (documents attendus/reçus)",
        before: "en_appel", after: `${afterSave?.statut} / ${afterSave?.appel?.documents?.length ?? 0} document(s)`,
        ok: saved.ok && afterSave?.statut === "en_appel" && afterSave?.appel?.documents?.length === 2,
        detail: saved.ok
          ? "dossier enregistré sans clôturer : les informations peuvent être ajoutées plus tard"
          : saved.error,
        ux: "La commission est extérieure : l’établissement peut préparer le dossier avant de connaître la décision.",
      });

      const lateDoc = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        updatedBy: "Direction audit",
        patch: {
          informations: "La commission a demandé un bulletin complémentaire.",
          documents: [
            { label: "Lettre de motivation", statut: "recu" },
            { label: "Bulletins", statut: "recu", reference: "dossier-2026" },
            { label: "Avis du médecin scolaire", statut: "attendu" },
          ],
        },
      });
      const afterLateDoc = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Direction", action: "Ajouter un document reçu plus tard",
        before: "en_appel", after: `${afterLateDoc?.appel?.documents?.length ?? 0} document(s)`,
        ok:
          lateDoc.ok &&
          afterLateDoc?.statut === "en_appel" &&
          afterLateDoc?.appel?.documents?.length === 3,
        detail: lateDoc.ok
          ? "le dossier reste ouvert et accepte de nouvelles pièces"
          : lateDoc.error,
        ux: "Les informations et documents peuvent arriver après l’ouverture de l’appel sans bloquer la saisie.",
      });

      const closed = await wf.closeFdAppel({
        etablissementId,
        ficheId: fiche.id,
        updatedBy: "Direction audit",
        patch: {
          commissionDate: "2026-06-20",
          receivedAt: "2026-06-22",
          decision: "maintien",
          motif: "La commission confirme la décision du conseil de classe.",
          documents: [
            { label: "Lettre de motivation", statut: "recu" },
            { label: "Bulletins", statut: "recu", reference: "dossier-2026" },
            { label: "Avis du médecin scolaire", statut: "recu", receivedAt: "2026-06-18" },
          ],
        },
      });
      const afterClose = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Direction", action: "Clôturer l’appel après décision extérieure",
        before: "en_appel", after: `${afterClose?.statut} / ${afterClose?.appel?.decision ?? "—"}`,
        ok:
          closed.ok &&
          afterClose?.statut === "cloturee" &&
          afterClose?.appel?.decision === "maintien" &&
          Boolean(afterClose?.closedAt),
        detail: closed.ok
          ? `décision enregistrée ; notification parents=${closed.sent ? "envoyée" : "SMTP indisponible"}`
          : closed.error,
        ux: "La fiche sort enfin de l’appel : décision, motif, dates, documents et PDF sont tracés.",
      });
      const appealArchive = await db
        .select()
        .from(schema.fdPdfArchive)
        .where(and(eq(schema.fdPdfArchive.ficheId, fiche.id), eq(schema.fdPdfArchive.kind, "appel_decision")));
      record({
        scenario: s, persona: "Admin", action: "Retrouver le PDF de décision d’appel",
        before: "—", after: `${appealArchive.length} archive(s) d’appel`,
        ok: appealArchive.length >= 1,
        detail: appealArchive.map((a) => a.title).join(", "),
      });
      const finalArchive = await db
        .select()
        .from(schema.fdPdfArchive)
        .where(and(eq(schema.fdPdfArchive.ficheId, fiche.id), eq(schema.fdPdfArchive.kind, "document_final")));
      record({
        scenario: s, persona: "Admin", action: "Vérifier le document final d’appel",
        before: "—", after: `${finalArchive.length} document(s) final(aux)`,
        ok: finalArchive.length >= 1,
        detail: finalArchive.map((a) => a.title).join(", "),
      });
    }

    // ---------------------------------------------------------------------
    // S7 — Avenant après acceptation
    // ---------------------------------------------------------------------
    {
      const s = "S7 avenant après acceptation";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const [saisie, conseil, choix, decisionFinale, acceptation] = etapes;
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s7" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      await familyReply(fiche.id, saisie.id, P1, { destination: "5e" });
      await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id });
      await publishConseil(fiche.id, conseil.id, { avis: "favorable", destinationProposee: "5e" });
      await familyReply(fiche.id, choix.id, P1, { destination: "5e" });
      await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: choix.id });
      await publishConseil(fiche.id, decisionFinale.id, { avis: "favorable", destinationProposee: "5e" });
      await wf.submitFdAcceptation({
        etablissementId, ficheId: fiche.id, etapeId: acceptation.id,
        payload: { accepte: true }, auteurLabel: "Parent audit",
        signature: { name: "Parent audit", email: P1, method: "saved" },
      });
      const beforeAvenant = await statutOf(fiche.id);
      const avenant = await publishConseil(fiche.id, decisionFinale.id, {
        avis: "favorable", destinationProposee: "autre_parcours", motif: "Erreur matérielle de destination.",
      }, { avenant: true });
      const afterAvenant = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Direction", action: "Publier un avenant après acceptation",
        before: beforeAvenant, after: `${afterAvenant?.statut} / etape=${afterAvenant?.etapeCouranteId === acceptation.id ? "acceptation" : "autre"}`,
        ok: avenant.ok && afterAvenant?.statut === "en_attente_acceptation",
        detail: avenant.ok ? "avenant publié, nouvelle acceptation demandée" : avenant.error,
        ux: "Le formulaire d’avenant s’ouvre vide : l’admin doit ressaisir la décision, sans voir l’avis précédent.",
      });
      const accept2 = await wf.submitFdAcceptation({
        etablissementId, ficheId: fiche.id, etapeId: acceptation.id,
        payload: { accepte: true }, auteurLabel: "Parent audit",
        signature: { name: "Parent audit", email: P1, method: "saved" },
      });
      record({
        scenario: s, persona: "Parent 1", action: "Accepter l’avenant",
        before: "en_attente_acceptation", after: await statutOf(fiche.id), ok: accept2.ok,
        detail: accept2.ok ? "nouvelle décision acceptée" : accept2.error,
      });
      const avenantArchives = await db
        .select()
        .from(schema.fdPdfArchive)
        .where(eq(schema.fdPdfArchive.ficheId, fiche.id));
      record({
        scenario: s, persona: "Admin", action: "Retrouver l’avenant dans les archives",
        before: "—", after: `${avenantArchives.length} archive(s)`,
        ok: avenantArchives.some((a) => a.kind.includes("avenant")),
        detail: avenantArchives.map((a) => a.kind).join(", "),
      });
    }

    // ---------------------------------------------------------------------
    // S8 — Non-répondant : gel, dégel, réponse tardive
    // ---------------------------------------------------------------------
    {
      const s = "S8 non-répondant";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const [saisie, conseil] = etapes;
      const ready = await makeFiche(campagne.id, saisie.id, "saisie_recue", { parents: [P1], label: "s8ready" });
      const late = await makeFiche(campagne.id, saisie.id, "en_attente_famille", { parents: [P1], label: "s8late" });
      const first = await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id });
      record({
        scenario: s, persona: "Direction", action: "Geler avec un non-répondant",
        before: "1 prête + 1 sans réponse", after: `${await statutOf(ready.id)} / ${await statutOf(late.id)}`,
        ok: first.advanced === 1 && first.skipped.length === 1,
        detail: `advanced=${first.advanced}, skipped=${first.skipped[0]?.eleve}: ${first.skipped[0]?.reason}`,
      });
      await wf.unfreezeFdEtape({ etablissementId, campagneId: campagne.id, etapeId: saisie.id });
      const reply = await familyReply(late.id, saisie.id, P1, { destination: "5e" });
      const second = await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id });
      record({
        scenario: s, persona: "Direction", action: "Dégeler, relancer, regeler",
        before: "étape gelée", after: `${await statutOf(late.id)} (advanced=${second.advanced})`,
        ok: reply.ok && second.advanced === 1,
        detail: reply.ok ? "réponse tardive acceptée puis avancée" : reply.error,
      });
    }

    // ---------------------------------------------------------------------
    // S9 — Campagne clôturée
    // ---------------------------------------------------------------------
    {
      const s = "S9 campagne clôturée";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s9" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      const token = await latestToken(fiche.id, "saisie");
      const closed = await wf.updateFdCampagne(etablissementId, campagne.id, { statut: "cloturee" });
      const ctx = token ? await wf.getFdPublicContext(token.token) : null;
      const reply = await familyReply(fiche.id, saisie.id, P1, { destination: "5e" });
      const send = await wf.sendFdFicheToFamille({ etablissementId, ficheId: fiche.id });
      record({
        scenario: s, persona: "Parent 1", action: "Ouvrir un ancien lien après clôture",
        before: "en_attente_famille", after: `campagneOpen=${ctx?.campagneOpen ?? "null"} / ${await statutOf(fiche.id)}`,
        ok:
          Boolean(ctx && ctx.campagneOpen === false) &&
          !reply.ok &&
          !send.ok &&
          Boolean(closed.closedAt),
        detail: `réponse=${reply.ok ? "acceptée (inattendu)" : reply.error} ; envoi=${send.ok ? "accepté" : send.error} ; closedAt=${closed.closedAt ? "renseigné" : "absent"}`,
        ux: "La clôture est horodatée et coupe les liens, les réponses et les relances ; l’UI campagne propose désormais clôturer puis archiver.",
      });
    }

    // ---------------------------------------------------------------------
    // S10 — Envoi : SMTP absent, aucun e-mail parent
    // ---------------------------------------------------------------------
    {
      const s = "S10 envois";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s10" });
      const direct = await wf.sendFdFicheToFamille({ etablissementId, ficheId: fiche.id });
      const afterDirect = await wf.getFdFiche(etablissementId, fiche.id);
      record({
        scenario: s, persona: "Système", action: "Envoi unitaire sans SMTP",
        before: "a_envoyer", after: `${await statutOf(fiche.id)} / sent=${direct.ok ? direct.sent : "erreur"}`,
        ok:
          direct.ok &&
          direct.sent === false &&
          afterDirect?.statut === "a_envoyer" &&
          afterDirect?.lastSentAt === null,
        detail: "l’e-mail n’est pas parti : statut et lastSentAt restent inchangés (pas de faux positif)",
        ux: "L’admin voit un échec réel et peut relancer ; le lien personnel reste disponible pour un renvoi.",
      });
      const sansEmail = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [], label: "s10b" });
      const batch = await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id, reminder: false });
      const camp = await wf.getFdCampagne(etablissementId, campagne.id);
      record({
        scenario: s, persona: "Secrétariat", action: "Envoyer à une fiche sans e-mail parent",
        before: "a_envoyer", after: `${await statutOf(sansEmail.id)} / campagne=${camp?.statut}`,
        ok: batch.failed >= 1,
        detail: `sent=${batch.sent}, failed=${batch.failed}, errors=${batch.errors.join(" | ")}`,
        ux: "La campagne peut être activée alors qu’aucun e-mail n’est parti pour la fiche sans contact.",
      });
    }

    // ---------------------------------------------------------------------
    // S11 — Terminale : intentions + deux conseils + acceptation
    // ---------------------------------------------------------------------
    {
      const s = "S11 Terminale";
      const { campagne, etapes } = await makeCampagne("Tle", "conseil_dabord");
      const [saisie, conseil1, choix, conseil2, acceptation] = etapes;
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s11", classe: "Tle A" });
      record({
        scenario: s, persona: "Admin", action: "Créer une campagne Tle en mode conseil d’abord",
        before: "—", after: `starter=${campagne.starterMode} / étapes=${etapes.length}`,
        ok: campagne.starterMode === "famille_dabord" && etapes.some((e) => e.kind === "acceptation_famille"),
        detail: "le mode est normalisé en famille_dabord et une acceptation est présente",
      });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      await familyReply(fiche.id, saisie.id, P1, { destination: "tle_st2s" }, "Intention : Terminale ST2S.");
      await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: saisie.id });
      await publishConseil(fiche.id, conseil1.id, { avis: "favorable", destinationProposee: "tle_st2s" });
      await familyReply(fiche.id, choix.id, P1, { destination: "tle_st2s" });
      await wf.advanceCampagneToConseil({ etablissementId, campagneId: campagne.id, fromEtapeId: choix.id });
      await publishConseil(fiche.id, conseil2.id, { avis: "favorable", destinationProposee: "tle_st2s" });
      const acc = await wf.submitFdAcceptation({
        etablissementId, ficheId: fiche.id, etapeId: acceptation.id,
        payload: { accepte: true }, auteurLabel: "Parent audit",
        signature: { name: "Parent audit", email: P1, method: "saved" },
      });
      record({
        scenario: s, persona: "Parent 1", action: "Accepter la décision finale",
        before: "en_attente_acceptation", after: await statutOf(fiche.id), ok: acc.ok && (await statutOf(fiche.id)) === "acceptee",
        detail: acc.ok ? "Terminale clôturée" : acc.error,
      });
    }

    // ---------------------------------------------------------------------
    // S12 — 1re ST2S
    // ---------------------------------------------------------------------
    {
      const s = "S12 1re ST2S";
      const { campagne, etapes } = await makeCampagne("1re", "famille_dabord", { templateKey: "lycee_1ere_st2s" });
      const [saisie] = etapes;
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1], label: "s12", classe: "1re ST2S" });
      const dests = campagne.catalogue.destinations.map((d) => d.id);
      record({
        scenario: s, persona: "Admin", action: "Vérifier le catalogue ST2S",
        before: "—", after: dests.join(", "),
        ok: dests.includes("tle_st2s"),
        detail: `template=${campagne.templateKey}`,
      });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      const reply = await familyReply(fiche.id, saisie.id, P1, { destination: "tle_st2s" });
      record({
        scenario: s, persona: "Parent 1", action: "Choisir la Terminale ST2S",
        before: "en_attente_famille", after: await statutOf(fiche.id), ok: reply.ok,
        detail: reply.ok ? "vœu ST2S enregistré" : reply.error,
      });
    }

    // ---------------------------------------------------------------------
    // S13 — Qui peut publier ? (preuve par le workflow + lecture route)
    // ---------------------------------------------------------------------
    {
      const s = "S13 signatures";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const conseil = etapes[1];
      const fiche = await makeFiche(campagne.id, conseil.id, "saisie_recue", { parents: [P1], label: "s13" });
      const dirOnly = await publishConseil(fiche.id, conseil.id, { avis: "favorable", destinationProposee: "5e" }, { signatures: [DIR_SIG] });
      record({
        scenario: s, persona: "Direction", action: "Publier avec la seule signature direction",
        before: "saisie_recue", after: await statutOf(fiche.id), ok: dirOnly.ok,
        detail: "le workflow n’exige pas la signature du PP, seulement celle de la direction",
        ux: "Un avis peut être publié sans signature PP alors que le libellé de l’étape annonce « signatures PP puis direction ».",
      });
      const ppCanPublish = access.canPublishFichesDialogue(["professeur"], {});
      const dirCanPublish = access.canPublishFichesDialogue(["direction"], {});
      record({
        scenario: s, persona: "Professeur principal",
        action: "Tenter de publier un avis avec le rôle professeur",
        before: "—",
        after: `professeur=${ppCanPublish ? "autorisé" : "refusé"} ; direction=${dirCanPublish ? "autorisée" : "refusée"}`,
        ok: !ppCanPublish && dirCanPublish,
        detail: "la garde serveur canPublishFichesDialogue réserve la publication à la direction/admin",
        ux: "Un PP peut préparer un brouillon mais ne peut plus auto-valider en tapant « Direction ».",
      });
    }

    // ---------------------------------------------------------------------
    // S14 — Élève : aucun rôle d’acteur
    // ---------------------------------------------------------------------
    {
      const s = "S14 élève";
      const roles = await db
        .selectDistinct({ role: schema.fdReponse.auteurRole })
        .from(schema.fdReponse)
        .where(eq(schema.fdReponse.etablissementId, etablissementId));
      const purposes = await db
        .selectDistinct({ purpose: schema.fdToken.purpose })
        .from(schema.fdToken)
        .where(eq(schema.fdToken.etablissementId, etablissementId));
      record({
        scenario: s, persona: "Élève",
        action: "Chercher un rôle décideur, une signature ou un jeton élève",
        before: "—", after: `roles=${roles.map((r) => r.role).join(", ")} ; purposes=${purposes.map((p) => p.purpose).join(", ")}`,
        ok:
          !roles.some((r) => r.role === "eleve") &&
          !purposes.some((p) => p.purpose.includes("eleve")),
        detail: "aucun rôle « eleve » ni jeton élève : l’élève ne décide pas et ne signe pas",
        ux: "L’élève est le sujet de la fiche mais n’est jamais acteur : ses responsables légaux décident, l’établissement arbitre.",
      });

      /**
       * Le seul écran élève est un visu en lecture seule, rattaché par e-mail /
       * sourceKey / INE. On vérifie aussi qu’une autre adresse ne voit rien.
       */
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", {
        parents: [P1],
        label: "s14-eleve",
      });
      await db
        .update(schema.eleve)
        .set({
          email: "eleve.audit@example.test",
          sourceKey: unique("audit:eleve-auth"),
        })
        .where(eq(schema.eleve.id, fiche.eleveId));
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      await familyReply(fiche.id, saisie.id, P1, { destination: "maintien" }, "Avis de la famille.");
      const found = await wf.findFdEleveForUser({
        etablissementId,
        email: "ELEVE.AUDIT@EXAMPLE.TEST",
      });
      const views = await wf.listFdEleveFiches(etablissementId, fiche.eleveId);
      const view = views.find((v) => v.id === fiche.id);
      const stranger = await wf.findFdEleveForUser({
        etablissementId,
        email: "autre.eleve.audit@example.test",
      });
      record({
        scenario: s, persona: "Élève",
        action: "Consulter sa fiche de dialogue",
        before: "en_attente_famille", after: `fiches=${views.length}, réponse famille=${view?.reponses.some((r) => r.auteurRole === "famille") ?? false}`,
        ok: Boolean(
          found?.id === fiche.eleveId &&
            view &&
            view.reponses.some((r) => r.auteurRole === "famille"),
        ),
        detail: "l’élève est rattaché par e-mail (insensible à la casse) et voit les réponses lisibles",
        ux: "Écran /eleve/fiches-dialogue en lecture seule : aucune action, aucun contact, aucun jeton.",
      });
      record({
        scenario: s, persona: "Élève non rattaché",
        action: "Tenter d’ouvrir le dossier d’un autre élève",
        before: "—", after: stranger ? "élève trouvé (fuite)" : "aucun rattachement",
        ok: stranger === null,
        detail: "une adresse sans correspondance ne donne accès à aucune fiche",
        ux: "L’écran affiche un message clair invitant à contacter la vie scolaire si le rattachement manque.",
      });
    }

    // ---------------------------------------------------------------------
    // S15 — Admin : voit-il la réponse famille avant de décider ?
    // ---------------------------------------------------------------------
    {
      const s = "S15 admin aveugle";
      const { campagne, etapes } = await makeCampagne("6e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await makeFiche(campagne.id, saisie.id, "a_envoyer", { parents: [P1, P2], label: "s15" });
      await wf.sendFdCampagneEtapeToFamilles({ etablissementId, campagneId: campagne.id });
      const saisieTokens = await activeTokens(fiche.id, "saisie");
      const tokenEmails = Array.from(
        new Set(saisieTokens.map((t) => t.email).filter((e): e is string => Boolean(e))),
      );
      await familyReply(fiche.id, saisie.id, P1, { destination: "maintien" }, "Nous demandons le redoublement.");
      const dossier = await wf.getFdFicheDossier(etablissementId, fiche.id);
      const famille = dossier?.reponses.find((r) => r.auteurRole === "famille");
      const texteFamille = famille
        ? [
            ...famille.sections.flatMap((section) => section.lines),
            ...famille.sections.flatMap((section) =>
              (section.checks ?? []).map((check) => check.label),
            ),
          ].join(" ")
        : "";
      const destination =
        famille && "values" in famille.payload
          ? famille.payload.values?.destination
          : undefined;
      const noSecrets = (dossier?.tokens ?? []).every(
        (t) => !("token" in t) && !("secureCode" in t),
      );
      record({
        scenario: s, persona: "Professeur principal / direction",
        action: "Ouvrir le dossier famille pour préparer l’avis du conseil",
        before: "saisie_recue",
        after: `réponses=${dossier?.reponses.length ?? 0}, vœu=${String(destination)}, commentaire=${texteFamille.includes("redoublement") ? "lisible" : "absent"}, secrets=${noSecrets ? "absents" : "exposés"}`,
        ok: Boolean(
          dossier &&
            famille &&
            destination === "maintien" &&
            texteFamille.includes("Nous demandons le redoublement.") &&
            noSecrets,
        ),
        detail:
          "GET /api/fiches-dialogue/fiches/[id] renvoie le vœu, le commentaire, les signatures et les documents, sans jeton ni code",
        ux: "Le PP voit la demande de la famille avant de décider ; le formulaire conseil est prérempli avec le dernier vœu (ou la dernière proposition en avenant).",
      });
      record({
        scenario: s, persona: "Parent 1 / Parent 2",
        action: "Recevoir le lien initial",
        before: "en_attente_famille",
        after: `${saisieTokens.length} jeton(s) saisi pour ${tokenEmails.length} adresse(s)`,
        ok: saisieTokens.length === 2 && tokenEmails.length === 2,
        detail: `jetons personnels : ${tokenEmails.join(" · ")}`,
        ux: "Chaque responsable reçoit son propre lien et son propre code : le dépôt est attribué à la bonne adresse et l’autre parent reçoit la demande de confirmation.",
      });
    }

    // ---------------------------------------------------------------------
    // Synthèse
    // ---------------------------------------------------------------------
    const byScenario = new Map<string, { ok: number; ko: number; gap: number }>();
    for (const o of obs) {
      const s = byScenario.get(o.scenario) ?? { ok: 0, ko: 0, gap: 0 };
      if (o.kind === "gap") s.gap += 1;
      else if (o.ok) s.ok += 1;
      else s.ko += 1;
      byScenario.set(o.scenario, s);
    }
    console.log("\n=== SYNTHÈSE ===");
    for (const [s, c] of byScenario) {
      console.log(`${s}: ${c.ok} OK / ${c.ko} KO / ${c.gap} manque(s)`);
    }
    const tests = obs.filter((o) => o.kind !== "gap");
    const gaps = obs.filter((o) => o.kind === "gap");
    console.log(
      `TESTS: ${tests.filter((o) => o.ok).length} OK / ${tests.filter((o) => !o.ok).length} KO — MANQUES CONFIRMÉS: ${gaps.length}`,
    );

    const fs = await import("node:fs/promises");
    await fs.writeFile(
      "docs/rapport-fiches-dialogue-personas-v2.json",
      `${JSON.stringify({ generatedAt: new Date().toISOString(), database: "scolia_migrate", observations: obs }, null, 2)}\n`,
      "utf8",
    );
    console.log("Écrit : docs/rapport-fiches-dialogue-personas-v2.json");
  } finally {
    // Nettoyage : la suppression de l'établissement cascade sur campagnes / fiches / réponses.
    try {
      await db.delete(schema.etablissement).where(eq(schema.etablissement.id, etablissementId));
    } catch {
      /* best-effort */
    }
    await endTestDatabase(closeDb);
  }
}

main().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
