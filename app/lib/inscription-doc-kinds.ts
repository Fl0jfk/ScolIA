/** Types de pièces d’inscription (classification OCR / IA). */

export const INSCRIPTION_DOC_KINDS = [
  "fiche_inscription",
  "bulletin",
  "releve_notes",
  "piece_identite",
  "livret_famille",
  "justificatif_domicile",
  "photo_identite",
  "attestation_assurance",
  "certificat_scolarite",
  "certificat_radiation",
  "vaccinations",
  "pap",
  "pai",
  "pps",
  "gevasco",
  "jugement",
  "autre",
] as const;

export type InscriptionDocKind = (typeof INSCRIPTION_DOC_KINDS)[number];

export const INSCRIPTION_DOC_KIND_LABELS: Record<InscriptionDocKind, string> = {
  fiche_inscription: "Fiche d'inscription",
  bulletin: "Bulletin scolaire",
  releve_notes: "Relevé de notes",
  piece_identite: "Pièce d'identité",
  livret_famille: "Livret de famille",
  justificatif_domicile: "Justificatif de domicile",
  photo_identite: "Photo d'identité",
  attestation_assurance: "Attestation d'assurance",
  certificat_scolarite: "Certificat de scolarité",
  certificat_radiation: "Certificat de radiation",
  vaccinations: "Vaccinations / carnet de santé",
  pap: "PAP",
  pai: "PAI",
  pps: "PPS",
  gevasco: "GEVASCO",
  jugement: "Jugement / autorité parentale",
  autre: "Document d'inscription",
};

export function isInscriptionDocKind(v: string): v is InscriptionDocKind {
  return (INSCRIPTION_DOC_KINDS as readonly string[]).includes(v);
}

export function inscriptionDocKindLabel(kind: InscriptionDocKind): string {
  return INSCRIPTION_DOC_KIND_LABELS[kind];
}

/** Titre dossier : identité élève déjà connue + nature du document (OCR). */
export function buildInscriptionDocumentTitle(opts: {
  nom: string;
  prenom: string;
  kind: InscriptionDocKind;
  /** Précision OCR (ex. « Bulletin 2e semestre 2024-2025 »), sans nom/prénom. */
  detail?: string | null;
}): string {
  const nom = opts.nom.trim().toUpperCase() || "ELEVE";
  const prenom = opts.prenom.trim() || "";
  const identity = prenom ? `${nom} ${prenom}` : nom;
  const detail = (opts.detail || "").trim();
  const kindLabel = inscriptionDocKindLabel(opts.kind);
  const nature =
    detail &&
    !/^document$/i.test(detail) &&
    !/^fichier$/i.test(detail) &&
    !/^pdf$/i.test(detail)
      ? detail
      : kindLabel;
  return `${identity} — ${nature}`.slice(0, 200);
}

/**
 * Mappe le couple type / origine renvoyé par l’IA (même vocabulaire que
 * `analyzeDocMatchEleve`) vers un kind d’inscription.
 */
export function inscriptionKindFromAiType(opts: {
  type?: string | null;
  titre?: string | null;
  detail?: string | null;
  origine?: string | null;
  fileName?: string | null;
}): InscriptionDocKind {
  const type = String(opts.type || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
  const blob = `${opts.type || ""} ${opts.titre || ""} ${opts.detail || ""}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  const origine = String(opts.origine || "")
    .trim()
    .toLowerCase();

  if (isInscriptionDocKind(type.replace(/[\s-]+/g, "_"))) {
    return type.replace(/[\s-]+/g, "_") as InscriptionDocKind;
  }

  if (/carte\s*d[' ]?identit|piece\s*d[' ]?identit|\bcni\b|passeport|titre\s*de\s*sejour/.test(blob)) {
    return "piece_identite";
  }
  if (/bulletin/.test(blob)) return "bulletin";
  if (/releve/.test(blob) || /notes/.test(type)) return "releve_notes";
  if (/livret/.test(blob)) return "livret_famille";
  if (/domicile|quittance|facture/.test(blob)) return "justificatif_domicile";
  if (/photo/.test(blob)) return "photo_identite";
  if (/assurance|mutuelle|responsabilite/.test(blob)) return "attestation_assurance";
  if (/radiation|exeat/.test(blob)) return "certificat_radiation";
  if (/fiche|inscription|formulaire/.test(blob)) return "fiche_inscription";
  if (/\bpap\b/.test(blob)) return "pap";
  if (/\bpai\b/.test(blob)) return "pai";
  if (/\bpps\b/.test(blob)) return "pps";
  if (/gevasco/.test(blob)) return "gevasco";
  if (/jugement|garde|autorite/.test(blob)) return "jugement";
  if (/vaccin|sante|carnet/.test(blob)) return "vaccinations";

  // « Certificat » : comme l’OCR dossiers — interne = scolarité, sinon on lit le titre.
  if (/certificat/.test(blob) || /attestation/.test(type)) {
    if (/scolarit/.test(blob) || origine === "interne") return "certificat_scolarite";
    if (/radiation|exeat/.test(blob)) return "certificat_radiation";
    if (origine === "externe" && /identit|cni|passeport/.test(blob)) return "piece_identite";
    if (/scolarit/.test(blob)) return "certificat_scolarite";
  }

  if (opts.fileName) return guessInscriptionKindFromFileName(opts.fileName);
  return "autre";
}

/** Heuristique nom de fichier si l’OCR / l’IA est indisponible. */
export function guessInscriptionKindFromFileName(fileName: string): InscriptionDocKind {
  const n = fileName
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  if (/fiche|inscription|dossier.?inscription|formulaire/.test(n)) return "fiche_inscription";
  if (/bulletin/.test(n)) return "bulletin";
  if (/releve|notes/.test(n)) return "releve_notes";
  if (/cni|passeport|identite|id.?card/.test(n)) return "piece_identite";
  if (/livret.?famille|livret_famille/.test(n)) return "livret_famille";
  if (/domicile|edf|facture|quittance/.test(n)) return "justificatif_domicile";
  if (/photo|portrait|id.?photo/.test(n)) return "photo_identite";
  if (/assurance|mutuelle|responsabilite.?civile|rc\b/.test(n)) return "attestation_assurance";
  if (/scolarite|scolarit/.test(n)) return "certificat_scolarite";
  if (/radiation|exeat/.test(n)) return "certificat_radiation";
  if (/vaccin|sante|carnet/.test(n)) return "vaccinations";
  if (/\bpap\b/.test(n)) return "pap";
  if (/\bpai\b/.test(n)) return "pai";
  if (/\bpps\b/.test(n)) return "pps";
  if (/gevasco/.test(n)) return "gevasco";
  if (/jugement|garde|autorite.?parentale/.test(n)) return "jugement";
  return "autre";
}
