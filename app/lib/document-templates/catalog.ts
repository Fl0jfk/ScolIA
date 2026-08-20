import type {
  DocumentOutputFormat,
  DocumentPlaceholderDef,
  DocumentTemplateId,
  DocumentTemplateMeta,
} from "@/app/lib/document-templates/types";

/** Token Word visible : remplacé ensuite dans Charlemagne / Pronote / Word. */
export function placeholderToken(key: string): string {
  return `$$${key}$$`;
}

/** Légende globale affichée dans l’UI Documents. */
export const DOCUMENT_PLACEHOLDERS: DocumentPlaceholderDef[] = [
  { token: "$$prenom$$", label: "Prénom de l’élève" },
  { token: "$$nom$$", label: "Nom de l’élève" },
  { token: "$$classe$$", label: "Classe" },
  { token: "$$annee$$", label: "Année scolaire" },
  { token: "$$date$$", label: "Date du document" },
  { token: "$$ville$$", label: "Ville" },
  { token: "$$responsable$$", label: "Responsable légal" },
  { token: "$$adresse$$", label: "Adresse familiale" },
  { token: "$$sortie$$", label: "Intitulé de la sortie" },
  { token: "$$objet$$", label: "Objet du courrier" },
  { token: "$$corps$$", label: "Corps du courrier" },
];

export const DOCUMENT_OUTPUT_FORMATS: DocumentOutputFormat[] = ["fillable-pdf", "docx"];

export const DOCUMENT_TEMPLATES: DocumentTemplateMeta[] = [
  {
    id: "certificat-scolarite",
    label: "Certificat de scolarité",
    description:
      "Modèle vierge brandé — PDF à trous ou Word avec $$prenom$$, $$nom$$, $$classe$$… pour publipostage.",
    formats: ["fillable-pdf", "docx"],
    fields: [
      { key: "prenom", label: "Prénom de l’élève", type: "text" },
      { key: "nom", label: "Nom de l’élève", type: "text" },
      { key: "classe", label: "Classe", type: "text" },
      { key: "annee", label: "Année scolaire", type: "text" },
      { key: "date", label: "Date du document", type: "date" },
      { key: "ville", label: "Fait à (ville)", type: "text" },
      { key: "signataire", label: "Nom du signataire", type: "text" },
      { key: "qualite", label: "Qualité du signataire", type: "text" },
    ],
  },
  {
    id: "fiche-inscription",
    label: "Fiche d’inscription",
    description:
      "Fiches par niveau (collège / lycée) — PDF Adobe remplissable brandé (logo, nom, couleur).",
    formats: ["fillable-pdf"],
    fields: [
      { key: "prenom", label: "Prénom de l’enfant", type: "text" },
      { key: "nom", label: "Nom de l’enfant", type: "text" },
      { key: "dateNaissance", label: "Date de naissance", type: "date" },
      { key: "classe", label: "Classe demandée", type: "text" },
      { key: "annee", label: "Année scolaire", type: "text" },
      { key: "responsable", label: "Responsable 1 — nom", type: "text" },
      { key: "resp1Email", label: "Responsable 1 — e-mail", type: "text" },
      { key: "resp1Tel", label: "Responsable 1 — téléphone", type: "text" },
      { key: "adresse", label: "Adresse familiale", type: "textarea" },
      { key: "allergies", label: "Allergies / précautions", type: "textarea" },
      { key: "droitImage", label: "Droit à l’image", type: "checkbox" },
      { key: "notes", label: "Notes secrétariat", type: "textarea" },
    ],
  },
  {
    id: "autorisation-sortie",
    label: "Autorisation de sortie",
    description:
      "Autorisation parentale vierge — PDF remplissable ou Word avec $$sortie$$, $$prenom$$…",
    formats: ["fillable-pdf", "docx"],
    fields: [
      { key: "prenom", label: "Prénom de l’élève", type: "text" },
      { key: "nom", label: "Nom de l’élève", type: "text" },
      { key: "classe", label: "Classe", type: "text" },
      { key: "sortie", label: "Intitulé de la sortie", type: "text" },
      { key: "lieu", label: "Lieu", type: "text" },
      { key: "dateDebut", label: "Date de début", type: "date" },
      { key: "dateFin", label: "Date de fin", type: "date" },
      { key: "horaireDepart", label: "Horaire de départ", type: "text" },
      { key: "horaireRetour", label: "Horaire de retour", type: "text" },
      { key: "responsable", label: "Responsable légal — nom", type: "text" },
      { key: "respTel", label: "Responsable légal — téléphone", type: "text" },
      { key: "urgenceTel", label: "Téléphone d’urgence", type: "text" },
      { key: "autorise", label: "Autorisation de participation", type: "checkbox" },
      { key: "soins", label: "Autorisation de soins", type: "checkbox" },
      { key: "notes", label: "Informations utiles", type: "textarea" },
      { key: "date", label: "Date de signature", type: "date" },
    ],
  },
  {
    id: "courrier-families",
    label: "Courrier aux familles",
    description:
      "Courrier type brandé — Word avec $$objet$$, $$corps$$, $$destinataire$$ pour publipostage.",
    formats: ["docx"],
    fields: [
      { key: "objet", label: "Objet", type: "text" },
      { key: "destinataire", label: "Destinataire", type: "text" },
      { key: "corps", label: "Corps du courrier", type: "textarea" },
      { key: "date", label: "Date du courrier", type: "date" },
      { key: "ville", label: "Fait à (ville)", type: "text" },
      { key: "signataire", label: "Nom du signataire", type: "text" },
      { key: "qualite", label: "Qualité du signataire", type: "text" },
    ],
  },
];

export function getTemplateMeta(id: string): DocumentTemplateMeta | undefined {
  return DOCUMENT_TEMPLATES.find((t) => t.id === id);
}

export function isDocumentTemplateId(id: string): id is DocumentTemplateId {
  return DOCUMENT_TEMPLATES.some((t) => t.id === id);
}

export function isDocumentOutputFormat(v: string): v is DocumentOutputFormat {
  return v === "docx" || v === "fillable-pdf";
}

/** Année scolaire FR courante (sept. → août) — info UI uniquement. */
export function defaultAnneeScolaire(now = new Date()): string {
  const y = now.getFullYear();
  const m = now.getMonth();
  return m >= 8 ? `${y}-${y + 1}` : `${y - 1}-${y}`;
}
