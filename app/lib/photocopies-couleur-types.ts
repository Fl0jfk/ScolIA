export type PhotoCopieStatus = "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "PRETE";

export type PhotoCopieEtablissement = string;

export type PhotoCopieActor = {
  userId: string;
  name: string;
  email?: string;
};

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
};

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
