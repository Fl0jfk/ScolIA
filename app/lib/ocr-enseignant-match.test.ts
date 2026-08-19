import assert from "node:assert/strict";
import test from "node:test";
import type { EnseignantConfig } from "./enseignants-types";
import { matchEnseignantFromDocument } from "./ocr-enseignant-match";
import { scanStudentsInText } from "./ocr-eleve-match";

function prof(partial: Partial<EnseignantConfig> & Pick<EnseignantConfig, "nom" | "prenom">): EnseignantConfig {
  return {
    id: partial.id || `${partial.nom}-${partial.prenom}`,
    nom: partial.nom,
    prenom: partial.prenom,
    folderName: partial.folderName || `${partial.nom} ${partial.prenom}`,
    secteur: partial.secteur || "lycee",
    email: partial.email,
    emailPro: partial.emailPro,
  };
}

const roster: EnseignantConfig[] = [
  prof({
    nom: "LEFEVRE",
    prenom: "Christelle",
    emailPro: "christelle.lefevre1@ac-normandie.fr",
    secteur: "lycee",
  }),
  prof({ nom: "MARTIN", prenom: "Paul", secteur: "college" }),
  prof({ nom: "DUPONT", prenom: "Marie", secteur: "lycee" }),
];

const CONVOCATION = `
Caen, le 17 juillet 2026
Convocation n° 3858 1 /2
ACADEMIE DE NORMANDIE
Baccalauréat général et technologique
LEFEVRE CHRISTELLE
152 RUE FERME DES CLOS
76530 MAUNY
christelle.lefevre1@ac-normandie.fr 76240 LE MESNIL ESNARD
6 RUE DE NEUVILLETTE
LGT PR LA PROVIDENCE
LEFEVRE CHRISTELLE
J'ai l'honneur de vous informer que je vous ai désigné(e) pour les opérations liées aux épreuves du baccalauréat
technologique, session 2026.
LYCEE GENERAL ET TECHNOLOGIQUE JEANNE D'ARC
`;

test("convocation bac : nom+prénom dans le texte même si l'IA n'extrait rien", () => {
  const r = matchEnseignantFromDocument({
    text: CONVOCATION,
    extractedNom: "non_trouvé",
    extractedPrenom: "non_trouvé",
    enseignants: roster,
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.enseignant?.nom, "LEFEVRE");
  assert.equal(r.reason, "email_dans_document");
});

test("convocation bac : scan nom/prénom sans email dans la liste", () => {
  const r = matchEnseignantFromDocument({
    text: CONVOCATION,
    extractedNom: "",
    extractedPrenom: "",
    enseignants: roster.map((e) => ({ ...e, email: undefined, emailPro: undefined })),
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.enseignant?.prenom, "Christelle");
  assert.equal(r.reason, "nom_prenom_dans_texte");
});

test("VILLETTE n'est pas un faux positif de NEUVILLETTE", () => {
  const hits = scanStudentsInText(CONVOCATION, [
    { nom: "VILLETTE", prenom: "Jean", folderName: "VILLETTE Jean", ine: "", classe: "TLE" },
  ]);
  assert.equal(hits.length, 0);
});

test("extrait IA non_trouvé ne bloque plus le matching", () => {
  const r = matchEnseignantFromDocument({
    text: "Courrier interne pour DUPONT Marie",
    extractedNom: "non_trouvé",
    extractedPrenom: "non_trouvé",
    enseignants: roster,
  });
  assert.equal(r.decision, "auto");
  assert.equal(r.enseignant?.nom, "DUPONT");
});
