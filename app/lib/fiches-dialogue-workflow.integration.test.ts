/**
 * Régressions « fiches de dialogue » — intégration Postgres locale.
 *
 * Usage :
 *   TEST_DATABASE_URL=postgresql://scola:...@127.0.0.1:5432/scolia_migrate \
 *   SCOLIA_TEST_MODE=1 npx tsx --test --require ./scripts/stub-server-only.cjs \
 *   app/lib/fiches-dialogue-workflow.integration.test.ts
 *
 * Couvre les correctifs du rapport personas :
 * - envoi post-conseil (decision_envoyee) et activation campagne ;
 * - gel d’étape : seules les fiches prêtes avancent, les autres sont signalées ;
 * - résolution de conflit parental ;
 * - verrou d’étape + signature direction + avenant explicite ;
 * - Terminale avec acceptation finale ;
 * - sélection ST2S ;
 * - repli PDF base64 si S3 indisponible ;
 * - refus (`refusee`) quand l’appel est explicitement désactivé ;
 * - campagne clôturée : plus aucune soumission famille.
 */
import assert from "node:assert/strict";
import test from "node:test";
import { randomUUID } from "node:crypto";
import { and, eq } from "drizzle-orm";
import {
  eleve,
  etablissement,
  fdCampagne,
  fdFiche,
  fdPdfArchive,
  fdReponse,
  fdEtape,
  fdToken,
  type FdFicheStatut,
} from "@/db/schema";
import { beginTestDatabase, endTestDatabase } from "@/app/lib/test-database-harness";

function unique(prefix: string): string {
  return `${prefix}-${randomUUID().slice(0, 8)}`;
}

test("fiches de dialogue — régressions workflow (intégration)", async (t) => {
  if (!beginTestDatabase(t)) return;

  const { getDb, closeDb } = await import("@/db/index");
  const wf = await import("@/app/lib/fiches-dialogue-workflow");
  const templates = await import("@/app/lib/fiches-dialogue-templates");
  const filing = await import("@/app/lib/fiches-dialogue-filing");

  const db = getDb();
  const etablissementId = await (async () => {
    const [row] = await db
      .insert(etablissement)
      .values({ slug: unique("fd-test"), name: "Établissement test fiches" })
      .returning();
    return row.id;
  })();

  async function createEleve(label: string): Promise<string> {
    const [row] = await db
      .insert(eleve)
      .values({
        etablissementId,
        sourceKey: unique(`test:${label}`),
        nom: `NOM-${label}`,
        prenom: `Prénom-${label}`,
        folderName: unique(`dossier-${label}`),
        classe: "5e A",
      })
      .returning();
    return row.id;
  }

  async function makeCampagne(
    niveau: "5e" | "1re" | "Tle",
    starterMode: "conseil_dabord" | "famille_dabord",
    templateKey?: string,
  ) {
    return wf.createFdCampagneFromTemplate({
      etablissementId,
      niveauActuel: niveau,
      starterMode,
      templateKey,
      label: `Campagne test ${niveau}`,
      anneeLabel: "2025-2026",
    });
  }

  async function insertFiche(params: {
    campagneId: string;
    etapeId: string;
    statut: FdFicheStatut;
    label: string;
    eleveId?: string;
  }) {
    const eleveId = params.eleveId ?? (await createEleve(params.label));
    const [fiche] = await db
      .insert(fdFiche)
      .values({
        etablissementId,
        campagneId: params.campagneId,
        eleveId,
        eleveNom: `NOM-${params.label}`,
        elevePrenom: `Prénom-${params.label}`,
        classeActuelle: "5e A",
        parentEmails: ["parent1@example.test", "parent2@example.test"],
        statut: params.statut,
        etapeCouranteId: params.etapeId,
      })
      .returning();
    return fiche;
  }

  const ppSignature = {
    role: "professeur_principal" as const,
    name: "PP Test",
    method: "saved",
  };
  const directionSignature = {
    role: "direction" as const,
    name: "Direction Test",
    method: "saved",
  };

  try {
    await t.test("SMTP absent : un échec d’envoi n’est jamais compté comme réussi", async () => {
      const { campagne, etapes } = await makeCampagne("5e", "famille_dabord");
      const choix = etapes.find((e) => e.kind === "choix_definitifs");
      assert.ok(choix, "étape choix_definitifs attendue");
      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: choix.id,
        statut: "decision_envoyee",
        label: "post-conseil",
      });

      const res = await wf.sendFdCampagneEtapeToFamilles({
        etablissementId,
        campagneId: campagne.id,
        onlyMissing: true,
      });
      assert.equal(res.sent, 0, "aucun e-mail réellement parti");
      assert.equal(res.failed, 1, "l’échec SMTP doit être compté");
      assert.match(res.errors.join(" "), /SMTP|délivré/i);

      const fresh = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(
        fresh?.statut,
        "decision_envoyee",
        "la fiche ne bascule pas en attente famille sans envoi",
      );
      assert.equal(fresh?.lastSentAt, null, "lastSentAt ne doit pas mentir");

      const tokenRows = await db
        .select()
        .from(fdToken)
        .where(and(eq(fdToken.ficheId, fiche.id), eq(fdToken.purpose, "saisie")));
      assert.ok(
        tokenRows.some((row) => !row.revokedAt),
        "le lien reste disponible pour un renvoi après rétablissement du SMTP",
      );

      const camp = await wf.getFdCampagne(etablissementId, campagne.id);
      assert.equal(
        camp?.statut,
        "brouillon",
        "une campagne sans aucun envoi réussi ne s’active pas toute seule",
      );
    });

    await t.test("gel vers le conseil : seules les fiches prêtes avancent", async () => {
      const { campagne, etapes } = await makeCampagne("5e", "famille_dabord");
      const saisie = etapes[0];
      const conseil = etapes[1];
      assert.equal(saisie.kind, "saisie_famille");
      assert.equal(conseil.kind, "conseil");

      const fReady = await insertFiche({
        campagneId: campagne.id,
        etapeId: saisie.id,
        statut: "saisie_recue",
        label: "pret",
      });
      const fNoReply = await insertFiche({
        campagneId: campagne.id,
        etapeId: saisie.id,
        statut: "en_attente_famille",
        label: "sans-reponse",
      });
      const fConflit = await insertFiche({
        campagneId: campagne.id,
        etapeId: saisie.id,
        statut: "en_conflit",
        label: "conflit",
      });

      const first = await wf.advanceCampagneToConseil({
        etablissementId,
        campagneId: campagne.id,
        fromEtapeId: saisie.id,
      });
      assert.equal(first.advanced, 1);
      assert.equal(first.skipped.length, 2);
      assert.ok(first.skipped.some((s) => s.ficheId === fNoReply.id));
      assert.ok(first.skipped.some((s) => s.ficheId === fConflit.id));

      const ready = await wf.getFdFiche(etablissementId, fReady.id);
      assert.equal(ready?.statut, "en_conseil");
      assert.equal(ready?.etapeCouranteId, conseil.id);

      const noReply = await wf.getFdFiche(etablissementId, fNoReply.id);
      assert.equal(noReply?.statut, "en_attente_famille");
      assert.equal(noReply?.etapeCouranteId, saisie.id);

      const conflict = await wf.getFdFiche(etablissementId, fConflit.id);
      assert.equal(conflict?.statut, "en_conflit");
      assert.equal(conflict?.etapeCouranteId, saisie.id);

      const resolved = await wf.resolveFdConflit({
        etablissementId,
        ficheId: fConflit.id,
        resolution: "avis_conseil",
        note: "Le conseil retient sa proposition.",
      });
      assert.equal(resolved.ok, true);

      const afterResolve = await wf.getFdFiche(etablissementId, fConflit.id);
      assert.equal(afterResolve?.statut, "saisie_recue");
      assert.equal(afterResolve?.parentAccord?.resolution, "avis_conseil");

      const second = await wf.advanceCampagneToConseil({
        etablissementId,
        campagneId: campagne.id,
        fromEtapeId: saisie.id,
      });
      assert.equal(second.advanced, 1, "la fiche déconflitée devient avançable");
      assert.equal(second.skipped.length, 1, "la non-réponse reste signalée");

      const resolvedFiche = await wf.getFdFiche(etablissementId, fConflit.id);
      assert.equal(resolvedFiche?.statut, "en_conseil");

      // Le non-répondant reste sur l’étape figée : l’admin peut la dégeler pour le traiter.
      const unfrozen = await wf.unfreezeFdEtape({
        etablissementId,
        etapeId: saisie.id,
      });
      assert.equal(unfrozen.gelee, false);

      const lateResponse = await wf.submitFdFamilleReponse({
        etablissementId,
        ficheId: fNoReply.id,
        etapeId: saisie.id,
        payload: { values: { destination: "maintien" } },
        auteurLabel: "Famille test",
      });
      assert.equal(lateResponse.ok, true, lateResponse.ok ? "" : lateResponse.error);

      const lateFiche = await wf.getFdFiche(etablissementId, fNoReply.id);
      assert.equal(lateFiche?.statut, "saisie_recue");

      const third = await wf.advanceCampagneToConseil({
        etablissementId,
        campagneId: campagne.id,
        fromEtapeId: saisie.id,
      });
      assert.equal(third.advanced, 1, "la réponse tardive peut être avancée après dégel");
      const lateAdvanced = await wf.getFdFiche(etablissementId, fNoReply.id);
      assert.equal(lateAdvanced?.statut, "en_conseil");
    });

    await t.test("Terminale : étape d’acceptation, verrou d’étape et avenant explicite", async () => {
      const { campagne, etapes } = await makeCampagne("Tle", "conseil_dabord");
      assert.equal(campagne.starterMode, "famille_dabord", "Tle force famille d’abord");
      const kinds = etapes.map((e) => e.kind);
      assert.deepEqual(kinds, [
        "saisie_famille",
        "conseil",
        "choix_definitifs",
        "conseil",
        "acceptation_famille",
        "appel",
      ]);

      const [saisie, conseil1, choix, conseil2, acceptation] = etapes;
      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: saisie.id,
        statut: "saisie_recue",
        label: "terminale",
      });

      const wrongStep = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil1.id,
        payload: { avis: "favorable" },
        signatures: [ppSignature, directionSignature],
        publish: true,
      });
      assert.equal(wrongStep.ok, false, "une étape non courante est refusée");
      assert.match(wrongStep.ok ? "" : wrongStep.error, /courante|avenant/i);

      const advanced = await wf.advanceCampagneToConseil({
        etablissementId,
        campagneId: campagne.id,
        fromEtapeId: saisie.id,
      });
      assert.equal(advanced.advanced, 1);

      const withoutDirection = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil1.id,
        payload: { avis: "favorable" },
        signatures: [ppSignature],
        publish: true,
      });
      assert.equal(withoutDirection.ok, false, "publication sans direction refusée");
      assert.match(withoutDirection.ok ? "" : withoutDirection.error, /direction/i);

      const firstPublish = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil1.id,
        payload: { avis: "favorable" },
        signatures: [ppSignature, directionSignature],
        publish: true,
      });
      assert.equal(firstPublish.ok, true, firstPublish.ok ? "" : firstPublish.error);

      const afterFirst = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(afterFirst?.etapeCouranteId, choix.id);
      assert.equal(
        afterFirst?.statut,
        "decision_envoyee",
        "sans SMTP, la fiche reste à envoyer au lieu d’afficher une fausse attente famille",
      );
      assert.equal(afterFirst?.lastSentAt, null);

      // La famille répond puis la fiche arrive au 2e conseil.
      await db
        .update(fdFiche)
        .set({ etapeCouranteId: conseil2.id, statut: "saisie_recue" })
        .where(eq(fdFiche.id, fiche.id));

      const secondPublish = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil2.id,
        payload: { avis: "favorable" },
        signatures: [ppSignature, directionSignature],
        publish: true,
      });
      assert.equal(secondPublish.ok, true, secondPublish.ok ? "" : secondPublish.error);

      const afterSecond = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(afterSecond?.etapeCouranteId, acceptation.id);
      assert.equal(afterSecond?.statut, "en_attente_acceptation");

      const acceptationTokens = await db
        .select()
        .from(fdToken)
        .where(and(eq(fdToken.ficheId, fiche.id), eq(fdToken.purpose, "acceptation")));
      assert.ok(
        acceptationTokens.some((row) => !row.revokedAt),
        "un jeton d’acceptation actif doit exister après publication",
      );

      const notCurrent = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil2.id,
        payload: { avis: "reserve" },
        signatures: [ppSignature, directionSignature],
        publish: true,
      });
      assert.equal(notCurrent.ok, false, "rectification sans avenant refusée");

      const avenant = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil2.id,
        payload: { avis: "favorable", motif: "Erreur de saisie sur la destination." },
        signatures: [ppSignature, directionSignature],
        publish: true,
        avenant: true,
      });
      assert.equal(avenant.ok, true, avenant.ok ? "" : avenant.error);

      const [conseilRow] = await db
        .select()
        .from(fdReponse)
        .where(
          and(
            eq(fdReponse.ficheId, fiche.id),
            eq(fdReponse.etapeId, conseil2.id),
            eq(fdReponse.auteurRole, "conseil"),
          ),
        )
        .limit(1);
      const conseilPayload = conseilRow?.payload as
        | { avenant?: boolean; avenantMotif?: string }
        | undefined;
      assert.equal(conseilPayload?.avenant, true);
      assert.equal(conseilPayload?.avenantMotif, "Erreur de saisie sur la destination.");

      const accepted = await wf.submitFdAcceptation({
        etablissementId,
        ficheId: fiche.id,
        etapeId: acceptation.id,
        payload: { accepte: true },
        auteurLabel: "Famille test",
        signature: { name: "Famille test", method: "pad" },
      });
      assert.equal(accepted.ok, true, accepted.ok ? "" : accepted.error);

      const afterAccept = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(afterAccept?.statut, "acceptee");
      assert.ok(afterAccept?.closedAt, "la fiche acceptée est clôturée");

      const allTokens = await db.select().from(fdToken).where(eq(fdToken.ficheId, fiche.id));
      assert.ok(allTokens.length > 0);
      assert.ok(
        allTokens.every((row) => row.revokedAt),
        "les jetons sont révoqués après clôture",
      );

      const afterCloseNoAvenant = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil2.id,
        payload: { avis: "reserve" },
        signatures: [ppSignature, directionSignature],
        publish: true,
      });
      assert.equal(
        afterCloseNoAvenant.ok,
        false,
        "une fiche acceptée ne se rouvre pas sans avenant explicite",
      );

      const afterCloseAvenant = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil2.id,
        payload: { avis: "favorable", motif: "Rectification après acceptation." },
        signatures: [ppSignature, directionSignature],
        publish: true,
        avenant: true,
      });
      assert.equal(
        afterCloseAvenant.ok,
        true,
        afterCloseAvenant.ok ? "" : afterCloseAvenant.error,
      );
      const reopened = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(reopened?.statut, "en_attente_acceptation");
      assert.equal(reopened?.etapeCouranteId, acceptation.id);

      const noAvenantOnCurrent = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil2.id,
        payload: { avis: "reserve" },
        signatures: [ppSignature, directionSignature],
        publish: true,
      });
      assert.equal(noAvenantOnCurrent.ok, false, "l’étape courante prévaut");
    });

    await t.test("Terminale legacy sans étape d’acceptation : clôture au dernier conseil", async () => {
      const { campagne, etapes } = await makeCampagne("Tle", "famille_dabord");
      const conseil2 = etapes.find(
        (e) => e.kind === "conseil" && e.ordre === 4,
      );
      assert.ok(conseil2, "2e conseil de Terminale attendu");
      const acceptation = etapes.find((e) => e.kind === "acceptation_famille");
      const appel = etapes.find((e) => e.kind === "appel");
      assert.ok(acceptation);
      assert.ok(appel);

      // Simule une campagne Terminale créée avant l’ajout de l’acceptation.
      await db.delete(fdEtape).where(eq(fdEtape.id, acceptation.id));
      await db.delete(fdEtape).where(eq(fdEtape.id, appel.id));

      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: conseil2.id,
        statut: "saisie_recue",
        label: "legacy-tle",
      });

      const res = await wf.submitFdConseilDecision({
        etablissementId,
        ficheId: fiche.id,
        etapeId: conseil2.id,
        payload: { avis: "favorable" },
        signatures: [ppSignature, directionSignature],
        publish: true,
      });
      assert.equal(res.ok, true, res.ok ? "" : res.error);

      const fresh = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(fresh?.statut, "cloturee");
      assert.ok(fresh?.closedAt, "la fiche est clôturée faute d’étape suivante");
    });

    await t.test("1re ST2S : templateKey respecté (pas de Terminale générale)", async () => {
      const preset = templates.presetForNiveau(
        "1re",
        "famille_dabord",
        "lycee_1ere_st2s",
      );
      assert.equal(preset.templateKey, "lycee_1ere_st2s");
      const ids = preset.catalogue.destinations.map((d) => d.id);
      assert.ok(ids.includes("tle_st2s"));
      assert.ok(!ids.includes("tle_g"), "la série ST2S ne doit pas proposer tle_g");

      const college = templates.presetForNiveau("5e", "conseil_dabord");
      assert.equal(
        college.catalogue.voiesOuverture?.[0]?.niveauActuel,
        "5e",
        "voiesOuverture.niveauActuel doit refléter le niveau réel",
      );

      const { campagne } = await makeCampagne("1re", "famille_dabord", "lycee_1ere_st2s");
      assert.equal(campagne.templateKey, "lycee_1ere_st2s");
      const campagneIds = campagne.catalogue.destinations.map((d) => d.id);
      assert.ok(campagneIds.includes("tle_st2s"));
      assert.ok(!campagneIds.includes("tle_g"));
    });

    await t.test("PDF : repli base64 quand S3 est indisponible", async () => {
      const eleveId = await createEleve("pdf");
      const { campagne, etapes } = await makeCampagne("5e", "famille_dabord");
      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: etapes[0].id,
        statut: "saisie_recue",
        label: "pdf",
        eleveId,
      });

      const pdfBytes = new TextEncoder().encode("%PDF-1.4 contenu de test");
      const filed = await filing.fileFicheDialoguePdfToDossier({
        etablissementId,
        ficheId: fiche.id,
        eleveId,
        etapeId: etapes[0].id,
        kind: "test_document",
        title: "Document test",
        pdfBytes,
        anneeLabel: "2025-2026",
      });

      const [archive] = await db
        .select()
        .from(fdPdfArchive)
        .where(eq(fdPdfArchive.id, filed.archiveId))
        .limit(1);
      assert.ok(archive?.pdfBase64, "le PDF doit être conservé en base si S3 échoue");
      assert.equal(
        Buffer.from(archive.pdfBase64 ?? "", "base64").toString("utf8"),
        "%PDF-1.4 contenu de test",
      );
      assert.equal(archive?.s3Key ?? null, null);
    });

    await t.test("appel : dossier complétable, décision obligatoire, clôture et PDF", async () => {
      const { campagne, etapes } = await makeCampagne("Tle", "famille_dabord");
      const appelEtape = etapes.find((e) => e.kind === "appel");
      assert.ok(appelEtape, "étape appel attendue");

      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: appelEtape.id,
        statut: "en_appel",
        label: "appel",
      });

      const [token] = await db
        .insert(fdToken)
        .values({
          etablissementId,
          ficheId: fiche.id,
          etapeId: appelEtape.id,
          token: unique("jeton-appel"),
          purpose: "saisie",
        })
        .returning();

      const firstSave = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        updatedBy: "Direction test",
        patch: {
          commissionLabel: "Commission académique d’appel",
          saisineDate: "2026-06-10",
          documents: [{ label: "Bulletins", statut: "attendu" }],
        },
      });
      assert.equal(firstSave.ok, true, firstSave.ok ? "" : firstSave.error);

      // Les pièces arrivent plus tard : un correctif partiel ne doit rien effacer.
      const laterSave = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        updatedBy: "Direction test",
        patch: {
          informations: "Pièces complémentaires reçues le 12 juin.",
        },
      });
      assert.equal(laterSave.ok, true, laterSave.ok ? "" : laterSave.error);
      assert.equal(laterSave.appel.commissionLabel, "Commission académique d’appel");
      assert.equal(laterSave.appel.saisineDate, "2026-06-10");
      assert.equal(laterSave.appel.documents?.length, 1);
      assert.equal(laterSave.appel.documents?.[0]?.statut, "attendu");

      const dated = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        patch: { commissionDate: "2026-06-15" },
      });
      assert.equal(dated.ok, true, dated.ok ? "" : dated.error);

      const cleared = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        patch: { commissionDate: "" },
      });
      assert.equal(cleared.ok, true, cleared.ok ? "" : cleared.error);
      assert.equal(cleared.appel.commissionDate, undefined, "une date erronée peut être effacée");

      const stillOpen = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(stillOpen?.statut, "en_appel", "l’enregistrement partiel ne clôture pas");

      const noDecision = await wf.closeFdAppel({
        etablissementId,
        ficheId: fiche.id,
        patch: { motif: "Motif sans décision." },
      });
      assert.equal(noDecision.ok, false);
      assert.match(noDecision.ok ? "" : noDecision.error, /décision/i);

      const noMotif = await wf.closeFdAppel({
        etablissementId,
        ficheId: fiche.id,
        patch: { decision: "maintien" },
      });
      assert.equal(noMotif.ok, false);
      assert.match(noMotif.ok ? "" : noMotif.error, /motif/i);

      const draftDecision = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        patch: { decision: "reformation" },
      });
      assert.equal(draftDecision.ok, true, draftDecision.ok ? "" : draftDecision.error);

      const afterPartial = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        patch: { informations: "Nouvelle pièce attendue." },
      });
      assert.equal(afterPartial.ok, true, afterPartial.ok ? "" : afterPartial.error);
      assert.equal(
        afterPartial.appel.decision,
        "reformation",
        "un patch partiel ne perd pas la décision déjà saisie",
      );

      const closed = await wf.closeFdAppel({
        etablissementId,
        ficheId: fiche.id,
        updatedBy: "Direction test",
        patch: {
          decision: "maintien",
          motif: "La commission confirme la décision du conseil.",
          documents: [
            { label: "Bulletins", statut: "recu", receivedAt: "2026-06-12" },
          ],
        },
      });
      assert.equal(closed.ok, true, closed.ok ? "" : closed.error);

      const afterClose = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(afterClose?.statut, "cloturee");
      assert.ok(afterClose?.closedAt, "la fiche est clôturée");
      assert.equal(afterClose?.appel?.decision, "maintien");
      assert.ok(afterClose?.appel?.closedAt, "l’appel porte sa date de clôture");
      assert.equal(afterClose?.appel?.documents?.[0]?.statut, "recu");

      const [archive] = await db
        .select()
        .from(fdPdfArchive)
        .where(
          and(
            eq(fdPdfArchive.ficheId, fiche.id),
            eq(fdPdfArchive.kind, "appel_decision"),
          ),
        )
        .limit(1);
      assert.ok(archive, "un PDF de décision d’appel est archivé");

      const [revoked] = await db
        .select()
        .from(fdToken)
        .where(eq(fdToken.id, token.id))
        .limit(1);
      assert.ok(revoked?.revokedAt, "les jetons de la fiche sont révoqués après clôture");

      const afterCloseSave = await wf.saveFdAppelDossier({
        etablissementId,
        ficheId: fiche.id,
        patch: { informations: "Trop tard." },
      });
      assert.equal(afterCloseSave.ok, false, "un appel clôturé ne se modifie plus");

      // Deuxième issue possible : la commission réforme la décision du conseil.
      const reformed = await insertFiche({
        campagneId: campagne.id,
        etapeId: appelEtape.id,
        statut: "en_appel",
        label: "appel-reformation",
      });
      const reformClose = await wf.closeFdAppel({
        etablissementId,
        ficheId: reformed.id,
        patch: {
          decision: "reformation",
          motif: "La commission retient la demande de redoublement de la famille.",
          destinationProposee: "Maintien en 5e",
        },
      });
      assert.equal(reformClose.ok, true, reformClose.ok ? "" : reformClose.error);
      const afterReform = await wf.getFdFiche(etablissementId, reformed.id);
      assert.equal(afterReform?.statut, "cloturee");
      assert.equal(afterReform?.appel?.decision, "reformation");
      assert.equal(afterReform?.appel?.destinationProposee, "Maintien en 5e");
    });

    await t.test("dossier famille staff et vue élève en lecture seule", async () => {
      const [student] = await db
        .insert(eleve)
        .values({
          etablissementId,
          sourceKey: unique("test:eleve-view"),
          nom: "NOM-ELEVE-VIEW",
          prenom: "Prénom-ELEVE-VIEW",
          folderName: unique("dossier-eleve-view"),
          classe: "5e A",
          email: "eleve.view@example.test",
        })
        .returning();
      const { campagne, etapes } = await makeCampagne("5e", "famille_dabord");
      const saisie = etapes[0];
      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: saisie.id,
        statut: "en_attente_famille",
        label: "dossier-famille",
        eleveId: student.id,
      });

      const reponse = await wf.submitFdFamilleReponse({
        etablissementId,
        ficheId: fiche.id,
        etapeId: saisie.id,
        payload: {
          values: { destination: campagne.catalogue.destinations[0]?.id || "maintien" },
          comment: "Nous souhaitons une poursuite en 4e.",
        },
        auteurLabel: "Parent test",
        signature: {
          name: "Parent test",
          email: "parent1@example.test",
          method: "saved",
        },
      });
      assert.equal(reponse.ok, true, reponse.ok ? "" : reponse.error);

      const dossier = await wf.getFdFicheDossier(etablissementId, fiche.id);
      assert.ok(dossier, "dossier staff attendu");
      assert.equal(dossier.fiche.parentEmails.length, 2);
      assert.ok(dossier.reponses.some((r) => r.auteurRole === "famille"));
      assert.ok(
        dossier.reponses.some((r) => r.sections.some((s) => s.lines.length > 0)),
        "les réponses doivent être lisibles, pas seulement un JSON brut",
      );
      assert.ok(dossier.signatures.some((s) => s.signerEmail === "parent1@example.test"));
      assert.ok(
        dossier.tokens.every((t) => !("token" in t) && !("secureCode" in t)),
        "le dossier staff ne divulgue ni jeton ni code",
      );

      const found = await wf.findFdEleveForUser({
        etablissementId,
        email: "ELEVE.VIEW@example.test",
      });
      assert.equal(found?.id, student.id, "l’élève est retrouvé par e-mail, insensible à la casse");
      const notFound = await wf.findFdEleveForUser({
        etablissementId,
        email: "autre.eleve@example.test",
      });
      assert.equal(notFound, null, "aucun accès croisé entre élèves");

      const views = await wf.listFdEleveFiches(etablissementId, student.id);
      const view = views.find((v) => v.id === fiche.id);
      assert.ok(view, "la fiche de l’élève est visible");
      assert.ok(view.reponses.some((r) => r.auteurRole === "famille"));
      assert.ok(
        !("parentEmails" in view) && !("tokens" in view),
        "la vue élève ne contient ni contacts ni jetons",
      );
    });

    await t.test("campagne clôturée : soumission famille refusée", async () => {
      const { campagne, etapes } = await makeCampagne("5e", "famille_dabord");
      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: etapes[0].id,
        statut: "en_attente_famille",
        label: "cloturee",
      });

      const closedCampagne = await wf.updateFdCampagne(etablissementId, campagne.id, {
        statut: "cloturee",
      });
      assert.ok(closedCampagne.closedAt, "la clôture de campagne est horodatée");

      const res = await wf.submitFdFamilleReponse({
        etablissementId,
        ficheId: fiche.id,
        etapeId: etapes[0].id,
        payload: { values: { destination: "maintien" } },
        auteurLabel: "Famille test",
      });
      assert.equal(res.ok, false);
      assert.match(res.ok ? "" : res.error, /clôturée/i);

      const fresh = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(fresh?.statut, "en_attente_famille", "aucun effet de bord");
    });

    await t.test("refus : refusee atteignable quand l’appel est explicitement désactivé", async () => {
      const { campagne, etapes } = await makeCampagne("Tle", "famille_dabord");
      const acceptation = etapes.find((e) => e.kind === "acceptation_famille");
      assert.ok(acceptation);
      const fiche = await insertFiche({
        campagneId: campagne.id,
        etapeId: acceptation.id,
        statut: "en_attente_acceptation",
        label: "refus",
      });

      await db
        .update(fdCampagne)
        .set({
          statut: "active",
          appelConfig: {
            enabled: false,
            contactPpLabel: "Vie scolaire",
          },
        })
        .where(eq(fdCampagne.id, campagne.id));

      const res = await wf.submitFdAcceptation({
        etablissementId,
        ficheId: fiche.id,
        etapeId: acceptation.id,
        payload: { accepte: false, motifRefus: "Désaccord motivé" },
        auteurLabel: "Famille test",
        signature: { name: "Famille test", method: "pad" },
      });
      assert.equal(res.ok, true, res.ok ? "" : res.error);

      const fresh = await wf.getFdFiche(etablissementId, fiche.id);
      assert.equal(fresh?.statut, "refusee");
      assert.ok(fresh?.refusedAt);
      assert.ok(fresh?.closedAt);
    });
  } finally {
    await endTestDatabase(closeDb);
  }
});
