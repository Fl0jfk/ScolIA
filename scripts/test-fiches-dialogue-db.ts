/**
 * Smoke DB workflow fiches de dialogue.
 * Usage: npx tsx --env-file=.env.local scripts/test-fiches-dialogue-db.ts
 */
import { getDb } from "../db/index";
import { eleve, fdCampagne, fdEtape, fdFiche, fdToken } from "../db/schema";
import { eq } from "drizzle-orm";
import { createFdCampagneFromTemplate } from "../app/lib/fiches-dialogue-workflow";
import { createFdAccessToken } from "../app/lib/fiches-dialogue-tokens";
import { submitFdFamilleReponse, submitFdConseilDecision, submitFdAcceptation } from "../app/lib/fiches-dialogue-workflow";

async function main() {
  const etabId = "eee622d1-7e24-44fe-a338-7c3db03e834a";
  const db = getDb();

  const [el] = await db.select().from(eleve).where(eq(eleve.etablissementId, etabId)).limit(1);
  if (!el) throw new Error("Aucun élève de test");

  const { campagne, etapes } = await createFdCampagneFromTemplate({
    etablissementId: etabId,
    templateKey: "college_trimestriel",
    label: "Test FD smoke",
    anneeLabel: "2025-2026",
    classesCibles: ["5e"],
    delaiFamilleJours: 7,
    appelConfig: {
      enabled: true,
      dateLimite: "15 juin 2026",
      procedureHtml: "Déposer le dossier au secrétariat.",
      documentsLabels: ["Formulaire d’appel"],
    },
  });

  const first = etapes[0];
  const [fiche] = await db
    .insert(fdFiche)
    .values({
      etablissementId: etabId,
      campagneId: campagne.id,
      eleveId: el.id,
      eleveNom: el.nom,
      elevePrenom: el.prenom,
      classeActuelle: el.classe || "5e1",
      parentEmails: ["parent.alice@localhost.dev"],
      statut: "en_attente_famille",
      etapeCouranteId: first.id,
    })
    .returning();

  const token = await createFdAccessToken({
    etablissementId: etabId,
    ficheId: fiche.id,
    etapeId: first.id,
    email: "parent.alice@localhost.dev",
    purpose: "saisie",
    expiresInDays: 14,
  });

  const saisie = await submitFdFamilleReponse({
    etablissementId: etabId,
    ficheId: fiche.id,
    etapeId: first.id,
    payload: {
      values: { destination: "4e", options: ["anglais", "espagnol"] },
      comment: "Vœux test",
    },
    auteurLabel: "Parent Alice",
    signature: { name: "Parent Alice", method: "pad" },
  });
  if (!saisie.ok) throw new Error(saisie.error);

  // Avancer à la décision finale pour tester acceptation/refus rapidement
  const decisionEtape = etapes.find((e) => e.kind === "decision_finale_conseil");
  if (!decisionEtape) throw new Error("Pas d'étape décision finale");
  await db
    .update(fdFiche)
    .set({ etapeCouranteId: decisionEtape.id, statut: "en_conseil" })
    .where(eq(fdFiche.id, fiche.id));

  const conseil = await submitFdConseilDecision({
    etablissementId: etabId,
    ficheId: fiche.id,
    etapeId: decisionEtape.id,
    payload: { avis: "reserve", destinationProposee: "4e", motif: "Suivi latin" },
    signatures: [
      { role: "professeur_principal", name: "M. PP" },
      { role: "direction", name: "Mme Direction" },
    ],
  });
  if (!conseil.ok) throw new Error(conseil.error);

  const acceptEtape = etapes.find((e) => e.kind === "acceptation_famille");
  if (!acceptEtape) throw new Error("Pas d'acceptation");

  // Recharger fiche (workflow a pu changer etapeCourante)
  const [fiche2] = await db.select().from(fdFiche).where(eq(fdFiche.id, fiche.id));
  await db
    .update(fdFiche)
    .set({ etapeCouranteId: acceptEtape.id, statut: "en_attente_acceptation" })
    .where(eq(fdFiche.id, fiche.id));

  const refus = await submitFdAcceptation({
    etablissementId: etabId,
    ficheId: fiche.id,
    etapeId: acceptEtape.id,
    payload: { accepte: false, motifRefus: "On veut garder latin" },
    auteurLabel: "Parent Alice",
    signature: { name: "Parent Alice", method: "pad" },
  });
  if (!refus.ok) throw new Error(refus.error);

  const [ficheFinal] = await db.select().from(fdFiche).where(eq(fdFiche.id, fiche.id));
  const tokens = await db.select().from(fdToken).where(eq(fdToken.ficheId, fiche.id));

  console.log(
    JSON.stringify(
      {
        ok: true,
        campagneId: campagne.id,
        ficheId: fiche.id,
        statutFinal: ficheFinal?.statut,
        tokenSample: token.token.slice(0, 8) + "…",
        tokensCount: tokens.length,
        etapes: etapes.length,
        fiche2statut: fiche2?.statut,
      },
      null,
      2,
    ),
  );
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
