import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName, sendS3WithConflictRetry } from "@/app/lib/s3-storage";
import { mergeOcrBatchJobs } from "@/app/lib/ocr-batch-merge";

export type OcrBatchJobStatus =
  | "pending"
  | "processing"
  | "completed"
  | "failed"
  | "cancelled"
  | "needs_token";

export type OcrBatchItemMode = "standard" | "class";

export type OcrJobTraceEntry = {
  t: string;
  scope: string;
  phase: string;
  level: string;
  message: string;
  data?: Record<string, unknown>;
};

export type OcrBatchResult = {
  success: boolean;
  fileName: string;
  error?: string;
  tempOneDrivePath?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  result?: any;
};

export type OcrBatchSegment = {
  pageStart: number;
  pageEnd: number;
  label?: string;
  /** Élève déjà identifié au découpage (ancrage identité) — évite un re-matching IA. */
  ine?: string;
  nom?: string;
  prenom?: string;
  folderName?: string;
};

/**
 * Sous-phase d'un item — permet de reprendre le traitement entre deux invocations
 * `after()` sans jamais bloquer (Textract poll non bloquant, segments incrémentaux).
 */
export type OcrBatchItemPhase =
  | "ocr_start" // doit lancer Textract
  | "ocr_poll" // Textract lancé, en attente du résultat
  | "analyze" // OCR prêt (mode standard) : analyse IA + déplacement
  | "segmenting" // OCR prêt (mode classe) : découpage à calculer
  | "segments"; // découpage prêt : traitement segment par segment

export type OcrBatchJobItem = {
  id: string;
  fileName: string;
  mode: OcrBatchItemMode;
  s3Key: string;
  tempPath: string;
  status: "pending" | "processing" | "done" | "failed";
  phase?: OcrBatchItemPhase;
  textractJobId?: string;
  /** Clé S3 du cache OCR ({text,pageTexts,pageCount}) pour ne pas gonfler le job JSON. */
  ocrCacheKey?: string;
  /** Segments calculés (mode classe). */
  segments?: OcrBatchSegment[];
  /** Index du prochain segment à traiter (mode classe). */
  segmentIndex?: number;
  /** Nombre de pages du PDF (métadonnées fichier, dès le lancement OCR). */
  pdfPageCount?: number;
  /** Pages déjà lues par Textract (progression partielle, si disponible). */
  ocrPagesRead?: number;
  /** Nombre de pages du PDF (renseigné après OCR Textract). */
  pageCount?: number;
  /** Moteur de découpage prévu ou utilisé (après OCR, avant classement). */
  segmentationEngine?: "identity" | "mistral" | "mistral_chunked" | "heuristic";
  /** Horodatage ISO : verrou optimiste anti-traitement concurrent du même item. */
  itemClaimedAt?: string;
  /** Nombre d'erreurs techniques consécutives sur cet item (retry borné avant échec définitif). */
  errorCount?: number;
};

export type OcrBatchJob = {
  jobId: string;
  userId: string;
  status: OcrBatchJobStatus;
  startedAt: string;
  updatedAt: string;
  processingStartedAt?: string;
  /** Ne pas relancer le worker avant cette date (planification sans sleep serveur). */
  nextRunAt?: string;
  accessToken: string;
  /** Refresh token délégué (optionnel) : permet au serveur de renouveler le token sans onglet ouvert. */
  refreshToken?: string;
  /** Origine HTTP du tenant — sert à l'auto-relance serveur (onglet fermé). */
  originUrl?: string;
  items: OcrBatchJobItem[];
  currentItemIndex: number;
  results: OcrBatchResult[];
  label: string;
  percent: number;
  completed: number;
  failed: number;
  error?: string;
  /** Derniers événements serveur (journal de bord, visible sans CloudWatch). */
  traceTail?: OcrJobTraceEntry[];
  /** Dernière relance worker demandée (anti-spam sur /status). */
  lastWorkerKickAt?: string;
};

type OcrCachePayload = {
  text: string;
  pageTexts: Record<string, string>;
  pageCount: number;
};

const OCR_CACHE_PREFIX = "agentIAOCR/batch-ocr/";

export function ocrCacheKey(jobId: string, itemId: string) {
  return `${OCR_CACHE_PREFIX}${jobId}/${itemId}.json`;
}

export async function writeOcrCache(key: string, payload: OcrCachePayload) {
  const s3Client = await getTenantDataS3Client();
  const bucket = await getBucketName();
  await sendS3WithConflictRetry(() =>
    s3Client.send(
      new PutObjectCommand({
        Bucket: bucket,
        Key: key,
        Body: JSON.stringify(payload),
        ContentType: "application/json",
      }),
    ),
  );
}

export async function readOcrCache(key: string): Promise<OcrCachePayload | null> {
  const s3Client = await getTenantDataS3Client();
  try {
    const res = await s3Client.send(
      new GetObjectCommand({ Bucket: await getBucketName(), Key: key }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    return JSON.parse(raw) as OcrCachePayload;
  } catch {
    return null;
  }
}

const JOB_PREFIX = "agentIAOCR/batch-jobs/";
const INDEX_PREFIX = "agentIAOCR/batch-jobs-index/";

function jobKey(jobId: string) {
  return `${JOB_PREFIX}${jobId}.json`;
}

function indexKey(userId: string) {
  return `${INDEX_PREFIX}${userId}.json`;
}

export function newBatchJobId() {
  return `${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
}

export async function readBatchJob(jobId: string): Promise<OcrBatchJob | null> {
  const s3Client = await getTenantDataS3Client();
  try {
    const res = await s3Client.send(
      new GetObjectCommand({
        Bucket: await getBucketName(),
        Key: jobKey(jobId),
      }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    return JSON.parse(raw) as OcrBatchJob;
  } catch {
    return null;
  }
}

export async function writeBatchJob(job: OcrBatchJob) {
  const s3Client = await getTenantDataS3Client();
  const bucket = await getBucketName();
  await sendS3WithConflictRetry(async () => {
    const existing = await readBatchJob(job.jobId);
    if (existing?.status === "cancelled" && job.status !== "cancelled") {
      return;
    }
    const merged = existing ? mergeOcrBatchJobs(existing, job) : job;
    await s3Client.send(
      new PutObjectCommand({
        Bucket: bucket,
        Key: jobKey(job.jobId),
        Body: JSON.stringify({ ...merged, updatedAt: new Date().toISOString() }),
        ContentType: "application/json",
      }),
    );
  });
}

type UserJobIndex = { jobIds: string[]; updatedAt: string };

async function readUserJobIndex(userId: string): Promise<UserJobIndex> {
  const s3Client = await getTenantDataS3Client();
  try {
    const res = await s3Client.send(
      new GetObjectCommand({
        Bucket: await getBucketName(),
        Key: indexKey(userId),
      }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return { jobIds: [], updatedAt: new Date().toISOString() };
    const parsed = JSON.parse(raw) as UserJobIndex;
    return { jobIds: Array.isArray(parsed.jobIds) ? parsed.jobIds : [], updatedAt: parsed.updatedAt };
  } catch {
    return { jobIds: [], updatedAt: new Date().toISOString() };
  }
}

async function writeUserJobIndex(userId: string, index: UserJobIndex) {
  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: indexKey(userId),
      Body: JSON.stringify({ ...index, updatedAt: new Date().toISOString() }),
      ContentType: "application/json",
    }),
  );
}

export async function registerBatchJobForUser(userId: string, jobId: string) {
  const index = await readUserJobIndex(userId);
  const jobIds = [jobId, ...index.jobIds.filter((id) => id !== jobId)].slice(0, 20);
  await writeUserJobIndex(userId, { jobIds, updatedAt: new Date().toISOString() });
}

export async function listRecentBatchJobsForUser(userId: string, limit = 5): Promise<OcrBatchJob[]> {
  const index = await readUserJobIndex(userId);
  const jobs: OcrBatchJob[] = [];
  for (const jobId of index.jobIds.slice(0, limit)) {
    const job = await readBatchJob(jobId);
    if (job && job.userId === userId) jobs.push(job);
  }
  return jobs;
}
