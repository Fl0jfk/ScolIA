export type ProcessResult = {
  success: boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  result?: any;
  error?: string;
  fileName: string;
  /** Chemin OneDrive (ex. Temp/monfichier.pdf) si le fichier est resté dans Temp */
  tempOneDrivePath?: string;
};

export type OcrProcessingStatus = {
  percent: number;
  total: number;
  done: number;
  completed: number;
  failed: number;
  totalKnown: boolean;
  label: string;
};

export const INITIAL_OCR_PROCESSING_STATUS: OcrProcessingStatus = {
  percent: 0,
  total: 0,
  done: 0,
  completed: 0,
  failed: 0,
  totalKnown: false,
  label: "",
};

export const BATCH_JOB_STORAGE_KEY = "agentIAOCR-active-batch-job";
/** Dernier lot terminé — conservé jusqu'au prochain dépôt de fichiers. */
export const BATCH_JOB_LAST_RESULTS_KEY = "agentIAOCR-last-batch-job";

export type OcrProgressDetail = {
  percent: number;
  label: string;
  phase: string;
  phaseLabel: string;
  fileName: string | null;
  fileIndex: number;
  fileTotal: number;
  pageCount: number | null;
  pdfPageCount: number | null;
  ocrPagesRead: number | null;
  segmentIndex: number | null;
  segmentTotal: number | null;
  segmentationEngine: "identity" | "mistral" | "mistral_chunked" | "heuristic" | null;
  documentsProcessed: number;
  documentsSucceeded: number;
  documentsFailed: number;
  updatedAt: string;
  idleSeconds: number;
};

export type OcrServerTraceEntry = {
  t: string;
  scope: string;
  phase: string;
  level: string;
  message: string;
  data?: Record<string, unknown>;
};

export type BatchJobStatusPayload = {
  jobId?: string;
  status?: string;
  label?: string;
  percent?: number;
  currentItemIndex?: number;
  totalItems?: number;
  completed?: number;
  failed?: number;
  results?: ProcessResult[];
  error?: string | null;
  serverManaged?: boolean;
  serverSelfRelays?: boolean;
  traceLog?: OcrServerTraceEntry[];
  progress?: OcrProgressDetail;
};

export type OcrSyncReport = {
  message?: string;
  secteurLabel?: string;
  basePath?: string;
  jsonForYourSecteur?: number;
  created?: number;
  alreadyThere?: number;
  createdFolders?: string[];
  extraFoldersCount?: number;
  extraFoldersOnOneDrive?: string[];
  ambiguousCount?: number;
  ambiguous?: Array<{ folderName: string; mef?: string; reason?: string }>;
  errors?: Array<{ folderName: string; error: string }>;
  otherSecteurCounts?: Record<string, number>;
  mefTableConfigured?: boolean;
};

export type OcrMefCounts = {
  total: number;
  lycee: number;
  college: number;
  ecole: number;
};

export const OCR_PHASE_STEPS: { id: string; label: string }[] = [
  { id: "ocr", label: "1. Lecture Mistral" },
  { id: "segmenting", label: "2. Découpage" },
  { id: "segments", label: "3. Nom & rangement" },
];

/** Ne jamais faire régresser la liste de résultats affichée (poll S3 parfois en retard). */
export function mergeOcrResultsForUi(prev: ProcessResult[], incoming: ProcessResult[]): ProcessResult[] {
  if (incoming.length === 0 && prev.length > 0) return prev;
  const byName = new Map<string, ProcessResult>();
  for (const r of prev) byName.set(r.fileName, r);
  for (const r of incoming) {
    const ex = byName.get(r.fileName);
    if (!ex) byName.set(r.fileName, r);
    else if (r.success && !ex.success) byName.set(r.fileName, r);
    else if (incoming.length >= prev.length) byName.set(r.fileName, r);
  }
  if (incoming.length >= prev.length) {
    return incoming.map((r) => byName.get(r.fileName) ?? r);
  }
  const order = [...prev.map((r) => r.fileName), ...incoming.map((r) => r.fileName)];
  const seen = new Set<string>();
  const merged: ProcessResult[] = [];
  for (const name of order) {
    if (seen.has(name)) continue;
    const row = byName.get(name);
    if (row) {
      merged.push(row);
      seen.add(name);
    }
  }
  return merged;
}

export function formatOcrIdleHint(seconds: number): string | null {
  if (seconds < 90) return null;
  if (seconds < 3600) {
    const min = Math.floor(seconds / 60);
    return `Dernière activité il y a ${min} min — sur un gros PDF, Mistral peut analyser 10–20 min sans mise à jour visible.`;
  }
  const h = Math.floor(seconds / 3600);
  return `Dernière activité il y a ${h} h — si rien ne bouge, vérifiez les résultats ou relancez.`;
}

export function getOcrActivePhaseIndex(progressDetail: OcrProgressDetail | null): number {
  if (!progressDetail) return -1;
  if (progressDetail.phase === "ocr") return 0;
  if (progressDetail.phase === "segmenting") return 1;
  if (progressDetail.phase === "segments" || progressDetail.phase === "analyze") return 2;
  return -1;
}

export function ocrDropZoneClass(
  active: boolean,
  variant: "blue" | "violet",
  opts: { dropsAvailable: boolean; dropDisabled: boolean; ocrProcessing: boolean },
): string {
  if (!opts.dropsAvailable) {
    return "relative overflow-hidden border-2 border-dashed rounded-3xl p-10 text-center border-slate-200 bg-slate-50/90 cursor-not-allowed opacity-80";
  }
  return `relative overflow-hidden border-2 border-dashed rounded-3xl p-10 text-center transition-all duration-300 group
    ${active ? (variant === "violet" ? "border-violet-600 bg-violet-50 scale-[1.01]" : "border-blue-600 bg-blue-50 scale-[1.01]") : "border-gray-300 bg-white hover:border-blue-400 hover:bg-gray-50"}
    ${opts.dropDisabled ? "opacity-60 cursor-not-allowed shadow-none" : "cursor-pointer shadow-lg hover:shadow-xl"}
    ${opts.ocrProcessing ? "ring-4 ring-blue-400/40 border-blue-500 bg-blue-50/80" : ""}`;
}

export type OcrSuggestedEleve = {
  nom: string;
  prenom: string;
  classe?: string;
  folderName: string;
  folderPath?: string;
  score?: number;
  matchedBy?: string;
};

export function ocrSuggestedEleves(result: ProcessResult): OcrSuggestedEleve[] {
  const raw = result.result?.matchCandidates ?? result.result?.matchDebug?.candidates ?? [];
  if (!Array.isArray(raw)) return [];
  return raw
    .map((c: Record<string, unknown>) => ({
      nom: String(c.nom || ""),
      prenom: String(c.prenom || ""),
      classe: c.classe ? String(c.classe) : undefined,
      folderName: String(c.folderName || ""),
      folderPath: c.folderPath ? String(c.folderPath) : undefined,
      score: typeof c.score === "number" ? c.score : undefined,
      matchedBy: c.matchedBy ? String(c.matchedBy) : undefined,
    }))
    .filter((c: OcrSuggestedEleve) => c.folderName && c.nom);
}

export function ocrExtractedSummary(result: ProcessResult): string | null {
  const eleve = result.result?.eleve;
  if (!eleve || typeof eleve !== "object") return null;
  const nom = String(eleve.nom || "").trim();
  const prenom = String(eleve.prénom || eleve.prenom || "").trim();
  const classe = String(eleve.classe || "").trim();
  const ine = String(eleve.ine || "").trim();
  const origin = String(result.result?.matchDebug?.origin || "").trim();
  const parts = [nom && prenom ? `${nom} ${prenom}` : nom || prenom, classe, ine ? `INE ${ine}` : "", origin]
    .filter(Boolean);
  return parts.length ? parts.join(" · ") : null;
}

export function ocrFailureHint(result: ProcessResult): string {
  const err = (result.error || "").toLowerCase();
  if (err.includes("élève") || err.includes("eleve") || err.includes("identifi")) {
    if (ocrSuggestedEleves(result).length > 0) {
      return "L’identité n’est pas assez certaine pour ranger tout seul. Choisissez l’élève ci-dessous.";
    }
    return "Le nom ou prénom de l'élève n'a pas été reconnu clairement dans le document.";
  }
  if (err.includes("incomplet") || err.includes("filename")) {
    return "Le type de document ou les informations attendues (classe, date…) n'ont pas pu être lues.";
  }
  if (err.includes("ocr") || err.includes("texte")) {
    return "Le texte du PDF est illisible ou trop pauvre pour une analyse fiable.";
  }
  if (err.includes("déplacé") || err.includes("deplace") || err.includes("folder") || err.includes("graph")) {
    return "Le dossier de destination n'a pas pu être créé ou atteint sur OneDrive.";
  }
  return "Le rangement automatique n'a pas abouti pour ce fichier.";
}

/** Distingue un échec « métier » (à classer à la main) d'une vraie erreur technique. */
export function ocrFailureCategory(result: ProcessResult): { label: string; technical: boolean } {
  const err = (result.error || "").toLowerCase();
  if (err.includes("élève") || err.includes("eleve") || err.includes("identifi")) {
    return {
      label: ocrSuggestedEleves(result).length > 0 ? "À valider" : "Élève non trouvé",
      technical: false,
    };
  }
  if (err.includes("incomplet") || err.includes("filename")) {
    return { label: "Lecture incomplète", technical: false };
  }
  if (err.includes("ocr") || err.includes("mistral") || err.includes("textract")) {
    return { label: "Lecture Mistral échouée", technical: true };
  }
  if (
    err.includes("token") ||
    err.includes("onedrive") ||
    err.includes("graph") ||
    err.includes("déplac") ||
    err.includes("deplac") ||
    err.includes("upload") ||
    err.includes("401") ||
    err.includes("429") ||
    err.includes("500")
  ) {
    return { label: "Erreur technique", technical: true };
  }
  return { label: "Échec", technical: true };
}

export function buildOcrProgressCaption({
  isUploadPhase,
  processingStatus,
  progressDetail,
  sessionDocTotal,
  sessionDocProcessed,
}: {
  isUploadPhase: boolean;
  processingStatus: OcrProcessingStatus;
  progressDetail: OcrProgressDetail | null;
  sessionDocTotal: number | null;
  sessionDocProcessed: number;
}): string {
  if (isUploadPhase) {
    if (processingStatus.totalKnown && processingStatus.total > 1) {
      return `Fichier ${Math.min(processingStatus.done + 1, processingStatus.total)} / ${processingStatus.total}`;
    }
    return "Envoi en cours…";
  }
  if (progressDetail) {
    if (progressDetail.phase === "segments" && sessionDocTotal) {
      return `Document ${sessionDocProcessed} / ${sessionDocTotal}`;
    }
    if (progressDetail.phase === "ocr") {
      if (progressDetail.pdfPageCount) {
        if (progressDetail.ocrPagesRead && progressDetail.ocrPagesRead > 0) {
          return `Page ${progressDetail.ocrPagesRead} / ${progressDetail.pdfPageCount}`;
        }
        return `0 / ${progressDetail.pdfPageCount} page(s)`;
      }
      return "Mistral lit le document…";
    }
    if (progressDetail.phase === "segmenting") {
      if (progressDetail.pageCount) {
        const engine = progressDetail.segmentationEngine;
        if (engine === "identity") return `Repérage élèves · ${progressDetail.pageCount} p.`;
        if (engine === "heuristic") return `Découpage auto · ${progressDetail.pageCount} p.`;
        if (engine === "mistral_chunked" || engine === "mistral") {
          return `Mistral découpe · ${progressDetail.pageCount} p.`;
        }
        return `Découpage · ${progressDetail.pageCount} p.`;
      }
      return "Mistral en déduit le découpage…";
    }
    if (progressDetail.fileTotal > 1) {
      return `Fichier ${progressDetail.fileIndex} / ${progressDetail.fileTotal}`;
    }
    if (progressDetail.pageCount) {
      return `${progressDetail.pageCount} page${progressDetail.pageCount > 1 ? "s" : ""}`;
    }
    return "";
  }
  if (processingStatus.totalKnown) {
    return `${processingStatus.done} / ${processingStatus.total} document${processingStatus.total > 1 ? "s" : ""}`;
  }
  return "";
}

export function tempOneDriveDisplayPath(path: string): string {
  return path.replace(/^Temp\//, "");
}
