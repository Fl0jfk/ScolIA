import type { DocumentTemplateId, DocumentTemplateMeta } from "@/app/lib/document-templates/types";

export const DOCUMENT_TEMPLATES: DocumentTemplateMeta[] = [
  {
    id: "certificat-scolarite",
    label: "Certificat de scolarité",
    description:
      "Attestation moderne brandée établissement — à remettre aux familles (PDF déjà rempli).",
    fields: [
      {
        key: "prenom",
        label: "Prénom de l’élève",
        type: "text",
        required: true,
        fromEleve: "prenom",
      },
      {
        key: "nom",
        label: "Nom de l’élève",
        type: "text",
        required: true,
        fromEleve: "nom",
      },
      {
        key: "classe",
        label: "Classe",
        type: "text",
        required: true,
        fromEleve: "classe",
      },
      {
        key: "anneeScolaire",
        label: "Année scolaire",
        type: "text",
        required: true,
        placeholder: "2025-2026",
      },
      {
        key: "dateDocument",
        label: "Date du document",
        type: "date",
        required: true,
      },
      {
        key: "ville",
        label: "Fait à (ville)",
        type: "text",
        required: true,
      },
      {
        key: "signataire",
        label: "Nom du signataire",
        type: "text",
        required: true,
        placeholder: "Le / La directeur·rice",
      },
      {
        key: "qualite",
        label: "Qualité du signataire",
        type: "text",
        required: true,
        placeholder: "Directeur / Directrice",
      },
    ],
  },
  {
    id: "fiche-inscription",
    label: "Fiche d’inscription",
    description:
      "Dossier d’inscription rempli depuis Scola — PDF prêt à archiver ou joindre (zéro papier secrétariat).",
    fields: [
      {
        key: "prenom",
        label: "Prénom de l’enfant",
        type: "text",
        required: true,
        fromEleve: "prenom",
      },
      {
        key: "nom",
        label: "Nom de l’enfant",
        type: "text",
        required: true,
        fromEleve: "nom",
      },
      {
        key: "dateNaissance",
        label: "Date de naissance",
        type: "date",
        required: true,
      },
      {
        key: "classeDemandee",
        label: "Classe demandée",
        type: "text",
        required: true,
        fromEleve: "classe",
      },
      {
        key: "anneeScolaire",
        label: "Année scolaire",
        type: "text",
        required: true,
        placeholder: "2025-2026",
      },
      {
        key: "resp1Nom",
        label: "Responsable 1 — nom",
        type: "text",
        required: true,
      },
      {
        key: "resp1Email",
        label: "Responsable 1 — e-mail",
        type: "text",
        required: true,
      },
      {
        key: "resp1Tel",
        label: "Responsable 1 — téléphone",
        type: "text",
        required: true,
      },
      {
        key: "resp2Nom",
        label: "Responsable 2 — nom",
        type: "text",
      },
      {
        key: "resp2Email",
        label: "Responsable 2 — e-mail",
        type: "text",
      },
      {
        key: "resp2Tel",
        label: "Responsable 2 — téléphone",
        type: "text",
      },
      {
        key: "adresse",
        label: "Adresse familiale",
        type: "textarea",
        required: true,
      },
      {
        key: "allergies",
        label: "Allergies / précautions",
        type: "textarea",
        placeholder: "Néant si aucune",
        aiAssist: true,
      },
      {
        key: "droitImage",
        label: "Droit à l’image accepté",
        type: "checkbox",
      },
      {
        key: "notes",
        label: "Notes secrétariat",
        type: "textarea",
        aiAssist: true,
      },
    ],
  },
  {
    id: "autorisation-sortie",
    label: "Autorisation de sortie",
    description:
      "Autorisation parentale pour une sortie / voyage — PDF rempli, DOCX ou PDF à trous pour ÉcoleDirecte.",
    fields: [
      {
        key: "prenom",
        label: "Prénom de l’élève",
        type: "text",
        required: true,
        fromEleve: "prenom",
      },
      {
        key: "nom",
        label: "Nom de l’élève",
        type: "text",
        required: true,
        fromEleve: "nom",
      },
      {
        key: "classe",
        label: "Classe",
        type: "text",
        required: true,
        fromEleve: "classe",
      },
      {
        key: "sortieTitre",
        label: "Intitulé de la sortie",
        type: "text",
        required: true,
        placeholder: "Visite musée / séjour ski…",
      },
      {
        key: "lieu",
        label: "Lieu",
        type: "text",
        required: true,
      },
      {
        key: "dateDebut",
        label: "Date de début",
        type: "date",
        required: true,
      },
      {
        key: "dateFin",
        label: "Date de fin",
        type: "date",
        required: true,
      },
      {
        key: "horaireDepart",
        label: "Horaire de départ",
        type: "text",
        placeholder: "08:00",
      },
      {
        key: "horaireRetour",
        label: "Horaire de retour",
        type: "text",
        placeholder: "17:30",
      },
      {
        key: "respNom",
        label: "Responsable légal — nom",
        type: "text",
        required: true,
      },
      {
        key: "respTel",
        label: "Responsable légal — téléphone",
        type: "text",
        required: true,
      },
      {
        key: "urgenceTel",
        label: "Téléphone d’urgence",
        type: "text",
      },
      {
        key: "autorise",
        label: "J’autorise mon enfant à participer",
        type: "checkbox",
      },
      {
        key: "soins",
        label: "J’autorise les soins d’urgence",
        type: "checkbox",
      },
      {
        key: "notes",
        label: "Informations utiles (santé, précautions…)",
        type: "textarea",
        aiAssist: true,
      },
      {
        key: "dateDocument",
        label: "Date de signature",
        type: "date",
        required: true,
      },
    ],
  },
  {
    id: "courrier-families",
    label: "Courrier aux familles",
    description:
      "Courrier administratif générique — retouche en Word ou envoi PDF brandé.",
    fields: [
      {
        key: "objet",
        label: "Objet",
        type: "text",
        required: true,
        placeholder: "Information aux familles",
        aiAssist: true,
      },
      {
        key: "destinataire",
        label: "Destinataire",
        type: "text",
        required: true,
        placeholder: "Aux familles des élèves de…",
      },
      {
        key: "corps",
        label: "Corps du courrier",
        type: "textarea",
        required: true,
        aiAssist: true,
        placeholder: "Madame, Monsieur,\n\n…",
      },
      {
        key: "dateDocument",
        label: "Date du courrier",
        type: "date",
        required: true,
      },
      {
        key: "ville",
        label: "Fait à (ville)",
        type: "text",
        required: true,
      },
      {
        key: "signataire",
        label: "Nom du signataire",
        type: "text",
        required: true,
      },
      {
        key: "qualite",
        label: "Qualité du signataire",
        type: "text",
        required: true,
        placeholder: "Directeur / Directrice",
      },
    ],
  },
];

export function getTemplateMeta(id: string): DocumentTemplateMeta | undefined {
  return DOCUMENT_TEMPLATES.find((t) => t.id === id);
}

export function isDocumentTemplateId(id: string): id is DocumentTemplateId {
  return DOCUMENT_TEMPLATES.some((t) => t.id === id);
}

/** Année scolaire FR courante (sept. → août). */
export function defaultAnneeScolaire(now = new Date()): string {
  const y = now.getFullYear();
  const m = now.getMonth(); // 0 = jan
  return m >= 8 ? `${y}-${y + 1}` : `${y - 1}-${y}`;
}

export function isDocumentOutputFormat(v: string): v is import("./types").DocumentOutputFormat {
  return v === "pdf" || v === "docx" || v === "fillable-pdf";
}
