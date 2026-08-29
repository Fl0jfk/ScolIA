/**
 * Test unitaire / smoke du module fiches de dialogue (PDF + templates + création campagne).
 * Usage: npx tsx scripts/test-fiches-dialogue.ts
 */
import { getFdTemplate, FD_CAMPAGNE_TEMPLATES } from "../app/lib/fiches-dialogue-templates";
import {
  buildFicheDialoguePdf,
  sectionsFromAcceptation,
  sectionsFromConseil,
  sectionsFromFamilleReponse,
} from "../app/lib/fiches-dialogue-pdf";

async function main() {
  if (FD_CAMPAGNE_TEMPLATES.length < 3) {
    throw new Error("Templates manquants");
  }
  const college = getFdTemplate("college_trimestriel");
  const lycee = getFdTemplate("lycee_semestriel");
  if (!college || !lycee) throw new Error("Templates collège/lycée absents");
  if (college.calendrierMode !== "trimestre") throw new Error("Collège doit être trimestriel");
  if (lycee.calendrierMode !== "semestre") throw new Error("Lycée doit être semestriel");
  if (!college.etapes.some((e) => e.kind === "acceptation_famille")) {
    throw new Error("Acceptation famille manquante");
  }
  if (!college.etapes.some((e) => e.kind === "appel" && e.optionnelle)) {
    throw new Error("Appel optionnel manquant");
  }

  const catalogue = college.catalogue;
  const famille = sectionsFromFamilleReponse(catalogue, {
    values: { destination: "5e", options: ["anglais", "latin"], commentaire_famille: "OK" },
  });
  const conseil = sectionsFromConseil(catalogue, {
    avis: "favorable",
    destinationProposee: "5e",
    commentaire: "Accord",
  });
  const accept = sectionsFromAcceptation(
    { accepte: false, motifRefus: "On maintient latin" },
    { enabled: true, dateLimite: "15 juin 2026" },
  );

  const pdf = await buildFicheDialoguePdf({
    title: "Document final",
    campagneLabel: "Test",
    anneeLabel: "2025-2026",
    eleveNom: "DUPONT",
    elevePrenom: "Alice",
    classeActuelle: "6e1",
    etapeLabel: "Acceptation",
    sections: [...famille, ...conseil, ...accept],
    signatures: [
      { role: "Famille", name: "Parent Dupont" },
      { role: "Direction", name: "Mme Directrice" },
    ],
  });

  if (pdf.byteLength < 500) throw new Error("PDF trop petit");
  console.log(
    JSON.stringify(
      {
        ok: true,
        templates: FD_CAMPAGNE_TEMPLATES.map((t) => t.key),
        pdfBytes: pdf.byteLength,
        sections: famille.length + conseil.length + accept.length,
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
