export type PhotoCopieStatus = "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "PRETE";

/** Couleur : validation direction. Noir & blanc : envoi direct au service impressions. */
export type PhotoCopieTypeImpression = "COULEUR" | "NOIR_BLANC";

export type PhotoCopieEtablissement = string;

/** Impression sur une face ou les deux. */
export type PhotoCopieFaces = "RECTO" | "RECTO_VERSO";

/** Finitions cumulables (cases à cocher). */
export type PhotoCopieFinition = "PLASTIFICATION" | "AGRAFAGE" | "PERFORATION" | "RELIURE";

/** Ordre d'affichage des finitions dans le formulaire. */
export const PHOTOCOPIE_FINITIONS: readonly { value: PhotoCopieFinition; label: string }[] = [
  { value: "PLASTIFICATION", label: "Plastification" },
  { value: "AGRAFAGE", label: "Agrafage" },
  { value: "PERFORATION", label: "Perforation" },
  { value: "RELIURE", label: "Reliure" },
];

/** Valeur inconnue ou absente → `RECTO` (impression simple). */
export function normalizePhotoCopieFaces(value: unknown): PhotoCopieFaces {
  const raw = String(value || "")
    .trim()
    .toUpperCase()
    .replace(/[\s-]+/g, "_");
  return raw === "RECTO_VERSO" ? "RECTO_VERSO" : "RECTO";
}

export function photoCopieFacesLabel(faces: PhotoCopieFaces): string {
  return faces === "RECTO_VERSO" ? "Recto verso" : "Recto";
}

/** Garde uniquement les finitions connues, sans doublon, dans l'ordre du formulaire. */
export function normalizePhotoCopieFinitions(value: unknown): PhotoCopieFinition[] {
  if (!Array.isArray(value)) return [];
  const wanted = new Set(value.map((v) => String(v || "").trim().toUpperCase()));
  return PHOTOCOPIE_FINITIONS.map((f) => f.value).filter((v) => wanted.has(v));
}

/**
 * Résumé lisible des options (« Recto verso · Plastification, Agrafage »).
 * Chaîne vide pour les anciennes demandes sans options.
 */
export function photoCopieOptionsLabel(record: {
  faces?: PhotoCopieFaces | string | null;
  finitions?: PhotoCopieFinition[] | string[] | null;
}): string {
  const parts: string[] = [];
  if (record.faces) parts.push(photoCopieFacesLabel(normalizePhotoCopieFaces(record.faces)));
  const finitions = normalizePhotoCopieFinitions(record.finitions);
  if (finitions.length > 0) {
    parts.push(
      finitions
        .map((v) => PHOTOCOPIE_FINITIONS.find((f) => f.value === v)?.label ?? v)
        .join(", "),
    );
  }
  return parts.join(" · ");
}

export type PhotoCopieActor = {
  userId: string;
  name: string;
  email?: string;
};

/** Nombre max de PDF joints à une même demande. */
export const PHOTOCOPIES_MAX_DOCUMENTS = 5;

/** Pièce jointe PDF d'une demande de photocopies. */
export type PhotoCopieDocument = {
  key: string;
  fileName: string;
  contentType?: string;
};

/** Nom lisible : prénom + nom, jamais l’e-mail si un vrai nom existe. */
export function photocopiePersonLabel(opts: {
  storedName?: string | null;
  firstName?: string | null;
  lastName?: string | null;
  email?: string | null;
}): string {
  const first =
    opts.firstName && !String(opts.firstName).includes("@") ? String(opts.firstName).trim() : "";
  const last =
    opts.lastName && !String(opts.lastName).includes("@") ? String(opts.lastName).trim() : "";
  const fromParts = `${first} ${last}`.trim();
  if (fromParts) return fromParts;
  const stored = String(opts.storedName ?? "").trim();
  if (stored && !stored.includes("@")) return stored;
  const email = String(opts.email ?? "").trim();
  return stored || email || "Utilisateur";
}

export type PhotoCopieRecord = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: PhotoCopieStatus;
  /**
   * Absent sur les anciennes demandes (= couleur, circuit direction).
   * `NOIR_BLANC` : créée directement en `ACCEPTEE` (file impressions, sans direction).
   */
  typeImpression?: PhotoCopieTypeImpression;
  createdBy: PhotoCopieActor & { email: string };
  submittedBy?: PhotoCopieActor & { roles?: string[] };
  etablissement: PhotoCopieEtablissement;
  motif: string;
  classesOuMatiere: string;
  nombrePhotocopies: number;
  /** Absent sur les anciennes demandes (non précisé). */
  faces?: PhotoCopieFaces;
  /** Finitions demandées (plastification, agrafage…). Absent ou vide = aucune. */
  finitions?: PhotoCopieFinition[];
  /** @deprecated Préférer `documents` — conservé pour compat lectures anciennes. */
  documentKey?: string;
  /** @deprecated Préférer `documents`. */
  documentFileName?: string;
  /** @deprecated Préférer `documents`. */
  documentContentType?: string;
  /** PDF joints (1 à {@link PHOTOCOPIES_MAX_DOCUMENTS}). */
  documents?: PhotoCopieDocument[];
  decidedBy?: PhotoCopieActor;
  decidedAt?: string;
  directionNote?: string;
  readyAt?: string;
  readyBy?: string;
  /** Demandeur a ouvert le module après « prête » → signal dashboard acquitté. */
  readySeenAt?: string;
};

/**
 * Liste normalisée des PDF d'une demande (tableau `documents` ou champs singuliers legacy).
 */
export function getPhotocopieDocuments(record: {
  documents?: PhotoCopieDocument[] | null;
  documentKey?: string | null;
  documentFileName?: string | null;
  documentContentType?: string | null;
}): PhotoCopieDocument[] {
  const fromArray = Array.isArray(record.documents)
    ? record.documents
        .map((d) => ({
          key: String(d?.key || "").trim(),
          fileName: String(d?.fileName || "").trim(),
          contentType: String(d?.contentType || "application/pdf").trim() || "application/pdf",
        }))
        .filter((d) => d.key && d.fileName)
    : [];
  if (fromArray.length > 0) return fromArray.slice(0, PHOTOCOPIES_MAX_DOCUMENTS);

  const key = String(record.documentKey || "").trim();
  const fileName = String(record.documentFileName || "").trim();
  if (!key || !fileName) return [];
  return [
    {
      key,
      fileName,
      contentType: String(record.documentContentType || "application/pdf").trim() || "application/pdf",
    },
  ];
}

export function hasPhotocopieDocuments(record: {
  documents?: PhotoCopieDocument[] | null;
  documentKey?: string | null;
  documentFileName?: string | null;
}): boolean {
  return getPhotocopieDocuments(record).length > 0;
}

/**
 * Champs document à persister : tableau + miroir du 1er PDF (compat).
 */
export function photocopieDocumentFields(
  docs: PhotoCopieDocument[],
): Pick<PhotoCopieRecord, "documents" | "documentKey" | "documentFileName" | "documentContentType"> {
  const cleaned = docs
    .map((d) => ({
      key: String(d.key || "").trim(),
      fileName: String(d.fileName || "").trim(),
      contentType: String(d.contentType || "application/pdf").trim() || "application/pdf",
    }))
    .filter((d) => d.key && d.fileName)
    .slice(0, PHOTOCOPIES_MAX_DOCUMENTS);
  if (cleaned.length === 0) return {};
  const first = cleaned[0];
  return {
    documents: cleaned,
    documentKey: first.key,
    documentFileName: first.fileName,
    documentContentType: first.contentType,
  };
}

/** Photocopie prête encore non vue par le demandeur (signal dashboard). */
export function isPhotocopieReadyUnseen(rec: {
  status: string;
  readySeenAt?: string;
}): boolean {
  return rec.status === "PRETE" && !String(rec.readySeenAt || "").trim();
}

export function photoCopieStatusLabel(status: PhotoCopieStatus): string {
  if (status === "ACCEPTEE") return "À imprimer";
  if (status === "REFUSEE") return "Refusée";
  if (status === "PRETE") return "Prête";
  return "En attente";
}

export function photoCopieStatusBadgeClass(status: PhotoCopieStatus): string {
  if (status === "PRETE") return "bg-emerald-50 text-emerald-800 border-emerald-200";
  if (status === "ACCEPTEE") return "bg-sky-50 text-sky-800 border-sky-200";
  if (status === "REFUSEE") return "bg-rose-50 text-rose-800 border-rose-200";
  return "bg-amber-50 text-amber-800 border-amber-200";
}

/** Normalise le type (anciennes demandes sans champ = couleur). */
export function normalizePhotoCopieTypeImpression(
  value: unknown,
): PhotoCopieTypeImpression {
  const raw = String(value || "")
    .trim()
    .toUpperCase()
    .replace(/[\s-]+/g, "_");
  if (raw === "NOIR_BLANC" || raw === "NB" || raw === "NOIR_ET_BLANC" || raw === "N_B") {
    return "NOIR_BLANC";
  }
  return "COULEUR";
}

export function photoCopieTypeImpressionLabel(type: PhotoCopieTypeImpression): string {
  return type === "NOIR_BLANC" ? "Noir et blanc" : "Couleur";
}

export function isPhotoCopieNoirBlanc(record: {
  typeImpression?: PhotoCopieTypeImpression | string | null;
}): boolean {
  return normalizePhotoCopieTypeImpression(record.typeImpression) === "NOIR_BLANC";
}

/** True si la demande a (ou aurait) dû passer par la direction. */
export function isPhotoCopieDirectionCircuit(record: {
  typeImpression?: PhotoCopieTypeImpression | string | null;
}): boolean {
  return !isPhotoCopieNoirBlanc(record);
}
