export type PhotoCopieStatus = "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "PRETE";

export type PhotoCopieEtablissement = string;

export type PhotoCopieActor = {
  userId: string;
  name: string;
  email?: string;
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
  createdBy: PhotoCopieActor & { email: string };
  submittedBy?: PhotoCopieActor & { roles?: string[] };
  etablissement: PhotoCopieEtablissement;
  motif: string;
  classesOuMatiere: string;
  nombrePhotocopies: number;
  documentKey?: string;
  documentFileName?: string;
  documentContentType?: string;
  decidedBy?: PhotoCopieActor;
  decidedAt?: string;
  directionNote?: string;
  readyAt?: string;
  readyBy?: string;
  /** Demandeur a ouvert le module après « prête » → signal dashboard acquitté. */
  readySeenAt?: string;
};

/** Photocopie prête encore non vue par le demandeur (signal dashboard). */
export function isPhotocopieReadyUnseen(rec: {
  status: string;
  readySeenAt?: string;
}): boolean {
  return rec.status === "PRETE" && !String(rec.readySeenAt || "").trim();
}

export function photoCopieStatusLabel(status: PhotoCopieStatus): string {
  if (status === "ACCEPTEE") return "Acceptée";
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
