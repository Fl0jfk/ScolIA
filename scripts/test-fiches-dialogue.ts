/**
 * Test unitaire / smoke du module fiches de dialogue (PDF + presets niveau).
 * Usage: npx tsx scripts/test-fiches-dialogue.ts
 */
import {
  getFdTemplate,
  FD_CAMPAGNE_TEMPLATES,
  presetForNiveau,
  FD_NIVEAUX,
} from "../app/lib/fiches-dialogue-templates";
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
  if (!getFdTemplate("college_3e")) throw new Error("college_3e manquant");

  const college = presetForNiveau("6e", "conseil_dabord");
  const lycee = presetForNiveau("2nde", "famille_dabord");
  if (college.calendrierMode !== "trimestre") throw new Error("Collège doit être trimestriel");
  if (lycee.calendrierMode !== "semestre") throw new Error("Lycée doit être semestriel");
  if (college.etapes[0]?.kind !== "conseil") {
    throw new Error("6e conseil_dabord doit commencer par conseil");
  }
  if (lycee.etapes[0]?.kind !== "saisie_famille") {
    throw new Error("2nde famille_dabord doit commencer par saisie famille");
  }
  for (const n of FD_NIVEAUX) {
    const p = presetForNiveau(n, "conseil_dabord");
    if (!p.catalogue.destinations.length && n !== "Tle") {
      // Tle uses checkboxes without classic destinations list content ok
    }
    if (!p.etapes.length) throw new Error(`Pas d'étapes pour ${n}`);
  }

  const catalogue = college.catalogue;
  const famille = sectionsFromFamilleReponse(catalogue, {
    values: { destination: "5e", options: ["lv1_anglais", "latin"] },
  });
  const conseil = sectionsFromConseil(catalogue, {
    avis: "favorable",
    destinationProposee: "5e",
    optionsProposees: ["lv1_anglais"],
    commentaire: "Accord",
  });
  const accept = sectionsFromAcceptation(
    { accepte: false, motifRefus: "On maintient latin" },
    { enabled: true, dateLimite: "15 juin 2026" },
  );

  if (!famille[0]?.checks?.length) throw new Error("Cases famille attendues");

  const pdf = await buildFicheDialoguePdf({
    title: "Apres la classe de 6eme",
    campagneLabel: "Test",
    anneeLabel: "2025-2026",
    eleveNom: "DUPONT",
    elevePrenom: "Alice",
    classeActuelle: "6e1",
    etapeLabel: "Acceptation",
    identity: {
      dateNaissance: "2013-05-12",
      ine: "1234567890A",
      mef: "100100",
      lva: "LV1 Anglais",
      lvb: "LV2 Espagnol",
    },
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
        niveaux: FD_NIVEAUX,
        templates: FD_CAMPAGNE_TEMPLATES.map((t) => t.key),
        pdfBytes: pdf.byteLength,
        familleChecks: famille[0].checks?.length ?? 0,
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
