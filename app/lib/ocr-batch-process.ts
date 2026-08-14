import "server-only";

import { randomUUID } from "crypto";
import { after } from "next/server";
import { DeleteObjectCommand, GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import {
  ocrCacheKey,
  readBatchJob,
  readOcrCache,
  writeBatchJob,
  writeOcrCache,
  type OcrBatchJob,
  type OcrBatchJobItem,
  type OcrBatchResult,
  type OcrBatchSegment,
} from "@/app/api/agentIAOCR/batch-job/batch-job";
import { analyzeDocMatchEleve, loadKnownStudentsForSegmentation } from "@/app/lib/ocr-analyze-eleve";
import type { KnownStudent } from "@/app/lib/ocr-segmentation";
import { extractPdfPagesBytes, getPdfPageCountFromS3 } from "@/app/lib/ocr-extract-pages";
import {
  deleteOneDrivePath,
  moveOneDriveFile,
  uploadBytesToOneDriveUnique,
} from "@/app/lib/ocr-graph-ops";
import { runDocumentSegmentation, resolveSegmentationEngine } from "@/app/lib/ocr-segment-run";
import { pollTextractOnce, startTextractForS3Key } from "@/app/lib/ocr-textract";
import { buildTextFromPages } from "@/app/lib/eleves-config";
import { resolveOneDriveProfileForClerkUserServer } from "@/app/lib/onedrive-user-profiles.server";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";
import { getMicrosoftAccessTokenFromRefresh } from "@/app/lib/graph-microsoft-delegated";
import { getClerkClientForTenant } from "@/app/lib/tenant-clerk";
import { getTenant } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import {
  ocrTrace,
  summarizeBatchItem,
  summarizeBatchJob,
  type OcrTraceCtx,
} from "@/app/lib/ocr-trace";
import { flushOcrJobTraces } from "@/app/lib/ocr-job-trace-store";

const RUN_LOCK_PREFIX = "agentIAOCR/batch-locks/";
/** Au-delà de cet âge, un lock est considéré orphelin (worker tué) et peut être volé. */
const LOCK_TTL_MS = 75_000;
/** Budget d'une invocation `after()` — on s'arrête bien avant le timeout de la fonction. */
const RUN_BUDGET_MS = 55_000;
/** Délai entre deux tours de polling Textract. */
const OCR_POLL_DELAY_MS = 2_000;
/** Un item claimé par un autre worker reste exclusif pendant cette durée. */
const ITEM_CLAIM_TTL_MS = 60_000;
/** Tentatives sur une erreur technique avant d'abandonner DÉFINITIVEMENT un document. */
const MAX_ITEM_ERRORS = 3;
/** Petite pause avant de réessayer un item en erreur transitoire (S3 / réseau). */
const ITEM_RETRY_DELAY_MS = 1_500;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function itemCtx(job: OcrBatchJob, itemIndex: number): OcrTraceCtx {
  const item = job.items[itemIndex];
  return { batchJobId: job.jobId, fileName: item?.fileName, itemIndex };
}

function runLockKey(jobId: string) {
  return `${RUN_LOCK_PREFIX}${jobId}.lock`;
}

type RunLockPayload = { acquiredAt: string; token: string };

async function readRunLock(jobId: string): Promise<RunLockPayload | null> {
  const s3Client = await getTenantDataS3Client();
  try {
    const res = await s3Client.send(
      new GetObjectCommand({ Bucket: await getBucketName(), Key: runLockKey(jobId) }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<RunLockPayload>;
    if (!parsed.acquiredAt || !parsed.token) return null;
    return { acquiredAt: parsed.acquiredAt, token: parsed.token };
  } catch {
    return null;
  }
}

/**
 * Lock S3 sans IfNoneMatch — Scaleway renvoie souvent
 * « A conflicting conditional operation… » sur les PUT conditionnels.
 * Stratégie : écriture + relecture du token (optimistic).
 */
async function acquireRunLock(jobId: string): Promise<boolean> {
  const existing = await readRunLock(jobId);
  if (existing) {
    const ageMs = Date.now() - new Date(existing.acquiredAt).getTime();
    if (!Number.isNaN(ageMs) && ageMs < LOCK_TTL_MS) {
      ocrTrace(jobId, "lock", "wait", "lock actif — attente", { lockAgeMs: ageMs, lockTtlMs: LOCK_TTL_MS });
      return false;
    }
    ocrTrace(jobId, "lock", "steal", "lock orphelin — écrasement", {
      lockAgeMs: Number.isNaN(ageMs) ? null : ageMs,
      lockTtlMs: LOCK_TTL_MS,
    });
  }

  const token = randomUUID();
  const s3Client = await getTenantDataS3Client();
  try {
    await s3Client.send(
      new PutObjectCommand({
        Bucket: await getBucketName(),
        Key: runLockKey(jobId),
        Body: JSON.stringify({ acquiredAt: new Date().toISOString(), token }),
        ContentType: "application/json",
      }),
    );
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    ocrTrace(jobId, "lock", "put-fail", "échec écriture lock", { message: msg.slice(0, 200) }, "warn");
    return false;
  }

  await sleep(120);
  const verify = await readRunLock(jobId);
  if (verify?.token === token) {
    ocrTrace(jobId, "lock", "acquire", "lock S3 acquis");
    return true;
  }
  ocrTrace(jobId, "lock", "lost-race", "lock perdu (autre worker)", {
    expected: token.slice(0, 8),
    got: verify?.token?.slice(0, 8) ?? null,
  });
  return false;
}

async function releaseRunLock(jobId: string) {
  const s3Client = await getTenantDataS3Client();
  try {
    await s3Client.send(
      new DeleteObjectCommand({ Bucket: await getBucketName(), Key: runLockKey(jobId) }),
    );
    ocrTrace(jobId, "lock", "release", "lock S3 libéré");
  } catch {
    ocrTrace(jobId, "lock", "release-fail", "échec libération lock (ignoré)", undefined, "warn");
  }
}

/**
 * Origine HTTP pour l'auto-relance (job → env → plateforme).
 */
export function resolveWorkerOrigin(job?: Pick<OcrBatchJob, "originUrl"> | null): string | undefined {
  const fromJob = job?.originUrl?.trim();
  if (fromJob) return fromJob.replace(/\/+$/, "");
  const explicit = process.env.OCR_WORKER_BASE_URL?.trim();
  if (explicit) return explicit.replace(/\/+$/, "");
  const app = process.env.NEXT_PUBLIC_APP_URL?.trim() || process.env.PLATFORM_APP_URL?.trim();
  return app ? app.replace(/\/+$/, "") : undefined;
}

/**
 * Planifie la prochaine invocation du worker (HTTP interne si possible, sinon after()).
 * Permet au lot d'avancer même si l'onglet est fermé ou le PC en veille.
 */
async function scheduleWorkerContinuation(
  originUrl: string | undefined,
  jobId: string,
  delayMs: number,
): Promise<void> {
  const delay = Math.max(0, Math.min(8_000, delayMs));
  const secret = process.env.OCR_WORKER_SECRET?.trim();
  const origin = originUrl || resolveWorkerOrigin(null);

  if (secret && origin) {
    try {
      const res = await fetch(`${origin}/api/agentIAOCR/batch-job/internal-run`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-ocr-worker-secret": secret,
        },
        body: JSON.stringify({ jobId, delayMs: delay }),
      });
      if (res.ok) {
        ocrTrace(jobId, "relay", "http-ok", "auto-relance HTTP acceptée", { delayMs: delay, origin });
        return;
      }
      ocrTrace(jobId, "relay", "http-fail", "auto-relance HTTP refusée — repli after()", {
        status: res.status,
        origin,
      }, "warn");
    } catch (err) {
      ocrTrace(jobId, "relay", "http-error", "auto-relance HTTP échouée — repli after()", {
        origin,
        error: err instanceof Error ? err.message : String(err),
      }, "warn");
    }
  } else {
    ocrTrace(jobId, "relay", "after-fallback", "auto-relance via after() (pas de chaîne HTTP)", {
      hasSecret: Boolean(secret),
      origin: origin ?? null,
      delayMs: delay,
    }, secret && !origin ? "warn" : "info");
  }

  after(async () => {
    try {
      if (delay > 0) await sleep(delay);
      ocrTrace(jobId, "relay", "after-run", "exécution worker via after()");
      await runOcrBatchJob(jobId);
    } catch (err) {
      ocrTrace(jobId, "relay", "after-error", "after() relance en erreur", {
        error: err instanceof Error ? err.message : String(err),
      }, "error");
    }
  });
}

/** Démarre ou relance le worker (chaîne HTTP interne si configurée). */
export async function kickOcrBatchWorker(jobId: string, originUrl?: string): Promise<void> {
  ocrTrace(jobId, "relay", "kick", "démarrage / relance worker demandée", {
    origin: originUrl ?? resolveWorkerOrigin(null) ?? null,
    hasSecret: Boolean(process.env.OCR_WORKER_SECRET?.trim()),
  });
  await scheduleWorkerContinuation(originUrl, jobId, 0);
}

/** Lot jamais démarré ou bloqué — candidat à une relance serveur. */
const PENDING_KICK_MS = 12_000;
const KICK_DEBOUNCE_MS = 15_000;

export function isBatchJobStale(job: OcrBatchJob): boolean {
  if (job.status !== "processing" && job.status !== "pending") return false;

  // Lot créé mais worker jamais passé en processing (cas le plus fréquent en serverless).
  if (job.status === "pending" && !job.processingStartedAt) {
    const started = new Date(job.startedAt).getTime();
    if (!Number.isNaN(started) && Date.now() - started >= PENDING_KICK_MS) return true;
  }

  const updatedAt = new Date(job.updatedAt).getTime();
  const staleByUpdate = !Number.isNaN(updatedAt) && Date.now() - updatedAt > LOCK_TTL_MS;
  let staleByNextRun = false;
  if (job.nextRunAt) {
    const next = new Date(job.nextRunAt).getTime();
    staleByNextRun = !Number.isNaN(next) && Date.now() > next + 3_000;
  }
  const stale = staleByUpdate || staleByNextRun;
  if (stale) {
    ocrTrace(job.jobId, "worker", "stale", "lot considéré bloqué", {
      status: job.status,
      updatedAt: job.updatedAt,
      nextRunAt: job.nextRunAt ?? null,
      staleByUpdate,
      staleByNextRun,
      processingStartedAt: job.processingStartedAt ?? null,
      ...summarizeBatchJob(job),
    }, "warn");
  }
  return stale;
}

/** Relance depuis /status (avec anti-spam). */
export function shouldKickWorkerFromStatus(job: OcrBatchJob): boolean {
  if (job.status === "completed" || job.status === "failed" || job.status === "needs_token") {
    return false;
  }
  if (!isBatchJobStale(job)) return false;
  const last = job.lastWorkerKickAt ? new Date(job.lastWorkerKickAt).getTime() : 0;
  if (!Number.isNaN(last) && Date.now() - last < KICK_DEBOUNCE_MS) return false;
  return true;
}

export async function recordWorkerKick(jobId: string): Promise<void> {
  const job = await readBatchJob(jobId);
  if (!job) return;
  await writeBatchJob({ ...job, lastWorkerKickAt: new Date().toISOString() });
}

async function deleteOcrCacheForJob(job: OcrBatchJob) {
  const s3Client = await getTenantDataS3Client();
  const bucket = await getBucketName();
  await Promise.all(
    job.items
      .map((it) => it.ocrCacheKey)
      .filter((k): k is string => Boolean(k))
      .map((Key) =>
        s3Client
          .send(new DeleteObjectCommand({ Bucket: bucket, Key }))
          .catch(() => undefined),
      ),
  );
}

import { buildBatchProgressView, computeProgress } from "@/app/lib/ocr-batch-progress";

async function patchJob(jobId: string, patch: Partial<OcrBatchJob>) {
  const job = await readBatchJob(jobId);
  if (!job) return null;
  const next = { ...job, ...patch, updatedAt: new Date().toISOString() };
  await writeBatchJob(next);
  return next;
}

async function getOdProfileForUser(userId: string) {
  const clerk = await getClerkClientForTenant();
  const user = await clerk.users.getUser(userId);
  return resolveOneDriveProfileForClerkUserServer({
    lastName: user.lastName,
    emailAddresses: user.emailAddresses?.map((e) => ({ emailAddress: e.emailAddress })),
    primaryEmailAddress: user.primaryEmailAddress
      ? { emailAddress: user.primaryEmailAddress.emailAddress }
      : null,
  });
}

class TokenExpiredError extends Error {
  constructor() {
    super("Session OneDrive expirée");
    this.name = "TokenExpiredError";
  }
}

/**
 * Contexte d'exécution : porte le token courant et sait le renouveler côté serveur
 * (refresh token du job ou refresh token délégué par cycle dans les secrets tenant).
 */
type WorkerCtx = {
  jobId: string;
  token: string;
  odProfile: OneDriveUserProfile | null;
  refreshToken: string | null;
  /** Élèves connus (eleves.json) — découpage ancré identité, chargés une seule fois par invocation. */
  knownStudents: KnownStudent[];
};

async function resolveServerRefreshToken(
  job: OcrBatchJob,
  odProfile: OneDriveUserProfile | null,
): Promise<string | null> {
  if (job.refreshToken?.trim()) return job.refreshToken.trim();
  if (!odProfile) return null;
  try {
    const tenant = await getTenant();
    const secrets = await getTenantSecrets(tenant.slug);
    const rt = secrets?.microsoft?.oneDriveBySecteur?.[odProfile.secteur]?.refreshToken;
    return rt?.trim() || null;
  } catch {
    return null;
  }
}

/** Tente un renouvellement serveur du token (sans onglet ouvert). */
async function tryServerTokenRefresh(ctx: WorkerCtx): Promise<boolean> {
  if (!ctx.refreshToken) return false;
  const res = await getMicrosoftAccessTokenFromRefresh(ctx.refreshToken);
  if ("error" in res) return false;
  ctx.token = res.accessToken;
  await patchJob(ctx.jobId, { accessToken: res.accessToken });
  return true;
}

/**
 * Rejoue une opération Graph en gérant un 401 :
 *  → tente un refresh serveur, sinon lève TokenExpiredError (le job passera needs_token).
 */
async function withToken<T extends { ok: boolean; status?: number }>(
  ctx: WorkerCtx,
  op: (token: string) => Promise<T>,
): Promise<T> {
  let res = await op(ctx.token);
  if (res.ok || res.status !== 401) return res;
  if (await tryServerTokenRefresh(ctx)) {
    res = await op(ctx.token);
    if (res.ok || res.status !== 401) return res;
  }
  throw new TokenExpiredError();
}

type StepOutcome =
  | { kind: "continue"; label?: string }
  | { kind: "wait"; delayMs: number; label?: string }
  | { kind: "result"; results: OcrBatchResult[]; itemDone: boolean; label?: string };

async function patchItem(
  jobId: string,
  itemIndex: number,
  patch: Partial<OcrBatchJobItem>,
  extra?: Partial<OcrBatchJob>,
) {
  const job = await readBatchJob(jobId);
  if (!job) return;
  await writeBatchJob({
    ...job,
    ...extra,
    items: job.items.map((it, i) => (i === itemIndex ? { ...it, ...patch } : it)),
    updatedAt: new Date().toISOString(),
  });
}

function hasSuccessfulResult(job: OcrBatchJob, fileName: string): boolean {
  return job.results.some((r) => r.fileName === fileName && r.success);
}

/** Fusionne les résultats sans perdre un succès déjà enregistré (polls / écritures concurrentes). */
function mergeBatchResults(existing: OcrBatchResult[], incoming: OcrBatchResult[]): OcrBatchResult[] {
  const byName = new Map<string, OcrBatchResult>();
  for (const r of existing) byName.set(r.fileName, r);
  for (const r of incoming) {
    const prev = byName.get(r.fileName);
    if (!prev) {
      byName.set(r.fileName, r);
      continue;
    }
    if (r.success && !prev.success) byName.set(r.fileName, r);
    else if (!r.success && prev.success) continue;
    else byName.set(r.fileName, r);
  }
  const order = [...existing.map((r) => r.fileName), ...incoming.map((r) => r.fileName)];
  const seen = new Set<string>();
  const merged: OcrBatchResult[] = [];
  for (const name of order) {
    if (seen.has(name)) continue;
    const row = byName.get(name);
    if (row) {
      merged.push(row);
      seen.add(name);
    }
  }
  for (const [name, row] of byName) {
    if (!seen.has(name)) merged.push(row);
  }
  return merged;
}

/**
 * Empêche deux workers de traiter le même item en parallèle.
 * @returns proceed = on continue ; skip-advance = item déjà fini ; defer = autre worker actif.
 */
async function resolveItemClaim(
  jobId: string,
  job: OcrBatchJob,
  itemIndex: number,
): Promise<"proceed" | "skip-advance" | "defer"> {
  const item = job.items[itemIndex];
  if (!item) return "skip-advance";

  if (item.status === "done" || item.status === "failed") {
    ocrTrace(jobId, "item", "skip", `item déjà ${item.status}`, summarizeBatchItem(item));
    return "skip-advance";
  }

  if (hasSuccessfulResult(job, item.fileName)) {
    ocrTrace(jobId, "item", "skip", "item déjà en succès dans results", summarizeBatchItem(item));
    return "skip-advance";
  }

  if (item.itemClaimedAt && item.status === "processing") {
    const age = Date.now() - new Date(item.itemClaimedAt).getTime();
    if (age >= 0 && age < ITEM_CLAIM_TTL_MS) {
      ocrTrace(jobId, "item", "defer", "item claimé par autre worker", {
        ...summarizeBatchItem(item),
        claimAgeMs: age,
        claimTtlMs: ITEM_CLAIM_TTL_MS,
      });
      return "defer";
    }
    ocrTrace(jobId, "item", "claim-expired", "claim expiré — reprise", {
      ...summarizeBatchItem(item),
      claimAgeMs: age,
    });
  }

  await patchItem(jobId, itemIndex, {
    status: "processing",
    itemClaimedAt: new Date().toISOString(),
  });
  ocrTrace(jobId, "item", "claim", "item claimé par ce worker", summarizeBatchItem(item));
  return "proceed";
}

async function analyzeAndMove(
  ctx: WorkerCtx,
  text: string,
  sourcePath: string,
  displayName: string,
): Promise<OcrBatchResult> {
  const trace: OcrTraceCtx = { batchJobId: ctx.jobId, fileName: displayName };
  ocrTrace(ctx.jobId, "classify", "start", "analyse + classement document", {
    displayName,
    sourcePath,
    textChars: text.length,
    odSecteur: ctx.odProfile?.secteur ?? null,
  });

  const ai = await analyzeDocMatchEleve(text, ctx.odProfile, trace);
  const extracted = `nom=${ai?.nom ?? "?"} prénom=${ai?.prénom ?? "?"} ine=${ai?.ine ?? "?"}`;
  ocrTrace(ctx.jobId, "classify", "extracted", extracted, {
    displayName,
    matchDebug: ai?.matchDebug ?? {},
    fileName: ai?.fileName ?? null,
    oneDriveFolderPath: ai?.oneDriveFolderPath ?? null,
  });

  if (!ai?.fileName) {
    ocrTrace(ctx.jobId, "classify", "fail", "analyse IA incomplète (pas de nom de fichier)", { displayName }, "warn");
    return {
      success: false,
      error: "Analyse IA incomplète.",
      fileName: displayName,
      result: ai,
      tempOneDrivePath: sourcePath,
    };
  }
  if (!ai.oneDriveFolderPath) {
    ocrTrace(
      ctx.jobId,
      "classify",
      "no-match",
      "élève non identifié — fichier laissé dans Temp",
      {
        displayName,
        profilOneDrive: ctx.odProfile ? ctx.odProfile.secteur : null,
        extracted: { nom: ai.nom, prenom: ai.prénom, ine: ai.ine },
      },
      "warn",
    );
    return {
      success: false,
      error:
        "Élève non identifié — le fichier reste dans Temp. Rangez-le à la main ou repassez-le en mode Standard.",
      fileName: displayName,
      result: ai,
      tempOneDrivePath: sourcePath,
    };
  }
  ocrTrace(ctx.jobId, "onedrive", "move-start", "déplacement OneDrive", {
    from: sourcePath,
    to: `${ai.oneDriveFolderPath}/${ai.fileName}.pdf`,
  });
  const move = await withToken(ctx, (token) =>
    moveOneDriveFile(token, sourcePath, ai.oneDriveFolderPath as string, `${ai.fileName}.pdf`),
  );
  if (!move.ok) {
    if (move.status === 404) {
      ocrTrace(ctx.jobId, "onedrive", "move-skip-404", "source Temp absente — déjà rangé", { displayName });
      const oneDriveItemPath = `${ai.oneDriveFolderPath}/${ai.fileName}.pdf`;
      return { success: true, result: { ...ai, oneDriveItemPath }, fileName: displayName };
    }
    ocrTrace(
      ctx.jobId,
      "onedrive",
      "move-fail",
      "déplacement impossible",
      { displayName, status: move.status, detail: move.detail.slice(0, 300) },
      "error",
    );
    return {
      success: false,
      error: `Déplacement impossible : ${move.detail.slice(0, 200)}`,
      fileName: displayName,
      result: ai,
      tempOneDrivePath: sourcePath,
    };
  }
  ocrTrace(ctx.jobId, "onedrive", "move-ok", "document rangé", {
    displayName,
    destination: `${ai.oneDriveFolderPath}/${move.finalFileName}`,
  });
  const oneDriveItemPath = `${ai.oneDriveFolderPath}/${move.finalFileName}`;
  return { success: true, result: { ...ai, oneDriveItemPath }, fileName: displayName };
}

/**
 * Classe un segment découpé : texte OCR déjà en cache (pas de re-OCR),
 * PDF extrait depuis S3, dépôt direct dans le dossier élève.
 * Le PDF classe entier reste dans Temp (originalTempPath) jusqu'à la fin du lot.
 */
async function analyzeAndFileSegment(
  ctx: WorkerCtx,
  text: string,
  pdfBytes: Uint8Array,
  displayName: string,
  originalTempPath: string,
  knownStudent?: { ine?: string; nom: string; prenom: string; folderName: string },
): Promise<OcrBatchResult> {
  const trace: OcrTraceCtx = { batchJobId: ctx.jobId, fileName: displayName };
  ocrTrace(ctx.jobId, "classify", "segment-start", "classement segment (texte OCR existant)", {
    displayName,
    textChars: text.length,
    pdfBytes: pdfBytes.length,
    originalTempPath,
    prematchedEleve: knownStudent?.folderName ?? null,
  });

  const ai = await analyzeDocMatchEleve(text, ctx.odProfile, trace, { segmentMode: true, knownStudent });

  if (!ai?.fileName) {
    return {
      success: false,
      error: "Analyse IA incomplète.",
      fileName: displayName,
      result: ai,
      tempOneDrivePath: originalTempPath,
    };
  }
  if (!ai.oneDriveFolderPath) {
    return {
      success: false,
      error:
        "Élève non identifié — le PDF classe entier reste dans Temp pour traitement manuel.",
      fileName: displayName,
      result: ai,
      tempOneDrivePath: originalTempPath,
    };
  }

  ocrTrace(ctx.jobId, "onedrive", "segment-upload-dest", "dépôt direct dossier élève", {
    folder: ai.oneDriveFolderPath,
    fileName: ai.fileName,
  });

  const upload = await withToken(ctx, (token) =>
    uploadBytesToOneDriveUnique(token, ai.oneDriveFolderPath as string, `${ai.fileName}.pdf`, pdfBytes),
  );
  if (!upload.ok) {
    ocrTrace(
      ctx.jobId,
      "onedrive",
      "segment-upload-fail",
      "dépôt dossier élève impossible",
      { detail: upload.detail.slice(0, 300) },
      "error",
    );
    return {
      success: false,
      error: `Dépôt élève impossible : ${upload.detail.slice(0, 200)}`,
      fileName: displayName,
      result: ai,
      tempOneDrivePath: originalTempPath,
    };
  }

  ocrTrace(ctx.jobId, "onedrive", "segment-upload-ok", "segment rangé", {
    destination: upload.path,
    fileName: upload.fileName,
  });
  return {
    success: true,
    result: { ...ai, oneDriveItemPath: upload.path, oneDriveFinalFileName: upload.fileName },
    fileName: displayName,
  };
}

async function stepItem(
  ctx: WorkerCtx,
  job: OcrBatchJob,
  itemIndex: number,
): Promise<StepOutcome> {
  const item = job.items[itemIndex];
  const phase = item.phase ?? "ocr_start";
  const trace = itemCtx(job, itemIndex);

  ocrTrace(job.jobId, "item", "step", `micro-étape phase=${phase}`, summarizeBatchItem(item));

  if (phase === "ocr_start") {
    ocrTrace(job.jobId, "textract", "start", "lancement Textract", {
      fileName: item.fileName,
      mode: item.mode,
      s3Key: item.s3Key,
    });
    const textractJobId = await startTextractForS3Key(item.s3Key, trace);
    let pdfPageCount: number | undefined;
    try {
      pdfPageCount = await getPdfPageCountFromS3(item.s3Key);
      ocrTrace(job.jobId, "textract", "pdf-meta", "nombre de pages PDF (métadonnées)", {
        fileName: item.fileName,
        pdfPageCount,
      });
    } catch (metaErr) {
      ocrTrace(job.jobId, "textract", "pdf-meta-fail", "métadonnées PDF indisponibles", {
        fileName: item.fileName,
        error: metaErr instanceof Error ? metaErr.message : String(metaErr),
      }, "warn");
    }
    await patchItem(job.jobId, itemIndex, {
      status: "processing",
      phase: "ocr_poll",
      textractJobId,
      pdfPageCount,
      ocrPagesRead: 0,
    });
    const ocrLabel = pdfPageCount
      ? `Mistral analyse votre document — ${item.fileName} : 0 / ${pdfPageCount} page(s)…`
      : `Mistral analyse votre document — ${item.fileName}`;
    ocrTrace(job.jobId, "textract", "polling", "passage en ocr_poll", {
      textractJobId,
      pdfPageCount: pdfPageCount ?? null,
    });
    return { kind: "wait", delayMs: OCR_POLL_DELAY_MS, label: ocrLabel };
  }

  if (phase === "ocr_poll") {
    if (!item.textractJobId) {
      ocrTrace(job.jobId, "textract", "reset", "textractJobId manquant — retour ocr_start", undefined, "warn");
      await patchItem(job.jobId, itemIndex, { phase: "ocr_start" });
      return { kind: "continue" };
    }
    const poll = await pollTextractOnce(item.textractJobId, trace);
    if (poll.status === "IN_PROGRESS") {
      const pagesRead = Math.max(item.ocrPagesRead ?? 0, poll.pagesRead);
      const pdfTotal = item.pdfPageCount;
      if (pagesRead !== item.ocrPagesRead) {
        await patchItem(job.jobId, itemIndex, { ocrPagesRead: pagesRead });
      }
      ocrTrace(job.jobId, "textract", "poll", "Textract en cours", {
        textractJobId: item.textractJobId,
        pagesRead,
        pdfTotal: pdfTotal ?? null,
        maxPageSeen: poll.maxPageSeen ?? null,
      });
      const label = pdfTotal
        ? pagesRead > 0
          ? `Mistral lit le document — ${item.fileName} : page ${pagesRead} / ${pdfTotal}…`
          : `Mistral analyse votre document — ${item.fileName} : 0 / ${pdfTotal} page(s)…`
        : `Mistral analyse votre document — ${item.fileName}`;
      return { kind: "wait", delayMs: OCR_POLL_DELAY_MS, label };
    }
    if (poll.status === "FAILED") {
      ocrTrace(job.jobId, "textract", "fail", "Textract FAILED", {
        fileName: item.fileName,
        textractJobId: item.textractJobId,
      }, "error");
      return {
        kind: "result",
        itemDone: true,
        results: [
          {
            success: false,
            error: "La lecture Mistral (OCR) a échoué sur ce fichier.",
            fileName: item.fileName,
            tempOneDrivePath: item.tempPath,
          },
        ],
      };
    }
    const cacheKey = ocrCacheKey(job.jobId, item.id);
    await writeOcrCache(cacheKey, poll.result);
    const needsSegmentation = item.mode === "class" && (poll.result.pageCount ?? 1) > 1;
    const nextPhase = needsSegmentation ? "segmenting" : "analyze";
    const segEngine = needsSegmentation
      ? resolveSegmentationEngine(poll.result.pageCount ?? 1)
      : undefined;
    ocrTrace(job.jobId, "textract", "done", "OCR terminé", {
      fileName: item.fileName,
      pageCount: poll.result.pageCount,
      textChars: poll.result.text.length,
      needsSegmentation,
      nextPhase,
      segmentationEngine: segEngine ?? null,
      cacheKey,
    });
    await patchItem(job.jobId, itemIndex, {
      ocrCacheKey: cacheKey,
      phase: nextPhase,
      pageCount: poll.result.pageCount,
      ocrPagesRead: poll.result.pageCount,
      segmentationEngine: segEngine,
    });
    const ocrLabel = needsSegmentation
      ? `Mistral a terminé la lecture — ${poll.result.pageCount} page(s), découpage à venir…`
      : `Mistral déduit le nom et le rangement — ${item.fileName}`;
    return { kind: "continue", label: ocrLabel };
  }

  const ocr = item.ocrCacheKey ? await readOcrCache(item.ocrCacheKey) : null;
  if (!ocr) {
    ocrTrace(job.jobId, "item", "cache-miss", "cache OCR absent — retour ocr_start", summarizeBatchItem(item), "warn");
    await patchItem(job.jobId, itemIndex, { phase: "ocr_start", textractJobId: undefined });
    return { kind: "continue" };
  }

  if (phase === "analyze") {
    ocrTrace(job.jobId, "item", "analyze", "mode standard / PDF unitaire", {
      fileName: item.fileName,
      mode: item.mode,
      pageCount: ocr.pageCount,
    });
    const result = await analyzeAndMove(ctx, ocr.text, item.tempPath, item.fileName);
    ocrTrace(job.jobId, "item", "analyze-done", "résultat analyse", {
      fileName: item.fileName,
      success: result.success,
      error: result.error ?? null,
    });
    return { kind: "result", results: [result], itemDone: true };
  }

  if (phase === "segmenting") {
    const engine =
      item.segmentationEngine ?? resolveSegmentationEngine(ocr.pageCount ?? item.pdfPageCount ?? 0);
    const engineHint =
      ctx.knownStudents.length > 0
        ? "repérage des élèves connus (INE + noms) pour grouper les pages de chaque bulletin"
        : engine === "mistral_chunked"
          ? "Mistral découpe par blocs (coupures entre documents uniquement)"
          : engine === "mistral"
            ? "Mistral cherche les frontières de chaque document"
            : "repérage automatique des documents (règles locales, sans IA)";
    ocrTrace(job.jobId, "segment", "start", "découpage documents", {
      fileName: item.fileName,
      pageCount: ocr.pageCount,
      engine,
      engineHint,
    });
    await patchJob(job.jobId, {
      label: `Mistral en déduit le découpage — ${engineHint} (${ocr.pageCount} page${ocr.pageCount > 1 ? "s" : ""})…`,
      updatedAt: new Date().toISOString(),
    });
    const segData = await runDocumentSegmentation(
      { pageTexts: ocr.pageTexts, pageCount: ocr.pageCount, knownStudents: ctx.knownStudents },
      trace,
    );
    const segments = (segData.segments || []) as OcrBatchSegment[];
    await patchItem(job.jobId, itemIndex, { segmentationEngine: segData.engine ?? engine });
    ocrTrace(job.jobId, "segment", "done", "segmentation terminée", {
      fileName: item.fileName,
      mode: segData.mode,
      engine: segData.engine,
      segmentCount: segments.length,
      segments: segments.map((s) => ({
        p: `${s.pageStart}-${s.pageEnd}`,
        label: s.label ?? null,
      })),
    });

    if (segData.mode === "single" || segments.length <= 1) {
      const seg = segments[0] || { pageStart: 1, pageEnd: ocr.pageCount || 1 };
      ocrTrace(job.jobId, "segment", "single", "un seul document — classement direct", {
        pages: `${seg.pageStart}-${seg.pageEnd}`,
      });
      const slice = buildTextFromPages(ocr.pageTexts, seg.pageStart, seg.pageEnd, ocr.text);
      const one = await analyzeAndMove(ctx, slice || ocr.text, item.tempPath, item.fileName);
      return { kind: "result", results: [one], itemDone: true };
    }

    await patchItem(job.jobId, itemIndex, { phase: "segments", segments, segmentIndex: 0 });
    return {
      kind: "continue",
      label: `Découpage terminé — ${segments.length} document${segments.length > 1 ? "s" : ""} détecté${segments.length > 1 ? "s" : ""}, classement…`,
    };
  }

  // phase === "segments"
  const segments = item.segments ?? [];
  const segIndex = item.segmentIndex ?? 0;
  const total = segments.length;
  if (segIndex >= total) {
    ocrTrace(job.jobId, "item", "segments-done", "tous les segments traités", { total });
    return { kind: "result", results: [], itemDone: true };
  }

  const seg = segments[segIndex];
  const label = `${item.fileName} [p.${seg.pageStart}-${seg.pageEnd}]`;
  const isLast = segIndex + 1 >= total;

  ocrTrace(job.jobId, "item", "segment", `classement segment ${segIndex + 1}/${total}`, {
    label,
    pages: `${seg.pageStart}-${seg.pageEnd}`,
    isLast,
  });

  const freshForSeg = await readBatchJob(job.jobId);
  if (freshForSeg && hasSuccessfulResult(freshForSeg, label)) {
    ocrTrace(job.jobId, "item", "segment-skip", "segment déjà en succès", { label });
    await patchItem(job.jobId, itemIndex, { segmentIndex: segIndex + 1 });
    return {
      kind: "result",
      results: [],
      itemDone: isLast,
      label: `Segment ${segIndex + 1}/${total} — ${item.fileName}`,
    };
  }

  let segResult: OcrBatchResult;

  try {
    const slice = buildTextFromPages(ocr.pageTexts, seg.pageStart, seg.pageEnd, ocr.text);
    if (!slice.trim()) {
      ocrTrace(job.jobId, "item", "segment-empty", "texte segment vide", { label }, "warn");
      segResult = {
        success: false,
        error: "Texte du segment vide — PDF classe entier laissé dans Temp.",
        fileName: label,
        tempOneDrivePath: item.tempPath,
      };
    } else {
      ocrTrace(job.jobId, "item", "segment-extract", "extraction pages PDF depuis S3 (OCR déjà fait)", {
        s3Key: item.s3Key,
        pages: `${seg.pageStart}-${seg.pageEnd}`,
      });
      const pdfBytes = await extractPdfPagesBytes(item.s3Key, seg.pageStart, seg.pageEnd);
      const knownStudent =
        seg.folderName && seg.nom && seg.prenom
          ? { ine: seg.ine, nom: seg.nom, prenom: seg.prenom, folderName: seg.folderName }
          : undefined;
      segResult = await analyzeAndFileSegment(ctx, slice, pdfBytes, label, item.tempPath, knownStudent);
    }
  } catch (segErr) {
    if (segErr instanceof TokenExpiredError) throw segErr;
    const msg = segErr instanceof Error ? segErr.message : String(segErr);
    ocrTrace(job.jobId, "item", "segment-error", "échec technique segment", { label, error: msg }, "error");
    segResult = {
      success: false,
      error: `${msg} — PDF classe entier laissé dans Temp (${item.fileName}).`,
      fileName: label,
      tempOneDrivePath: item.tempPath,
    };
  }

  if (isLast) {
    const freshEnd = await readBatchJob(job.jobId);
    const segPrefix = `${item.fileName} [p.`;
    const priorSegFailures = (freshEnd?.results ?? job.results).some(
      (r) =>
        r.fileName.startsWith(segPrefix) &&
        !r.success &&
        r.tempOneDrivePath === item.tempPath,
    );
    const keepOriginalInTemp = priorSegFailures || !segResult.success;
    if (keepOriginalInTemp) {
      ocrTrace(
        job.jobId,
        "onedrive",
        "original-kept",
        "PDF classe conservé dans Temp (segments en échec ou dernier segment non classé)",
        { tempPath: item.tempPath },
        "warn",
      );
    } else {
      try {
        await withToken(ctx, async (token) => {
          await deleteOneDrivePath(token, item.tempPath);
          return { ok: true as const };
        });
        ocrTrace(job.jobId, "onedrive", "original-deleted", "original classe supprimé du Temp", {
          tempPath: item.tempPath,
        });
      } catch (delErr) {
        if (delErr instanceof TokenExpiredError) throw delErr;
        ocrTrace(job.jobId, "onedrive", "original-delete-fail", "suppression original Temp échouée", {
          tempPath: item.tempPath,
          error: delErr instanceof Error ? delErr.message : String(delErr),
        }, "warn");
      }
    }
  }

  ocrTrace(job.jobId, "item", "segment-result", "résultat segment", {
    label,
    success: segResult.success,
    error: segResult.error ?? null,
    segmentIndex: segIndex + 1,
    total,
  });
  await patchItem(job.jobId, itemIndex, { segmentIndex: segIndex + 1 });
  return {
    kind: "result",
    results: [segResult],
    itemDone: isLast,
    label: `Segment ${segIndex + 1}/${total} — ${item.fileName}`,
  };
}

/**
 * @param opts.selfChain
 *   true (défaut) : le worker s'auto-relance côté serveur (chaîne HTTP interne / after())
 *     pour avancer même onglet fermé — utilisé par la création et internal-run.
 *   false : un seul chunk est exécuté puis on rend la main SANS planifier de relance serveur.
 *     C'est le client (poll /process) qui relancera. Fiable quand la page reste ouverte,
 *     même sans secret d'auto-relance configuré sur Scaleway.
 */
export async function runOcrBatchJob(
  jobId: string,
  opts?: { selfChain?: boolean },
) {
  const selfChain = opts?.selfChain !== false;
  const invokeStartedAt = Date.now();
  ocrTrace(jobId, "worker", "invoke", "runOcrBatchJob appelé", { selfChain });

  const pre = await readBatchJob(jobId);
  if (!pre) {
    ocrTrace(jobId, "worker", "abort", "job introuvable", undefined, "warn");
    return;
  }
  if (pre.status === "completed" || pre.status === "failed" || pre.status === "needs_token") {
    ocrTrace(jobId, "worker", "skip", "job déjà terminal", { status: pre.status });
    return;
  }

  const workerOrigin = resolveWorkerOrigin(pre);
  ocrTrace(jobId, "worker", "context", "état initial du lot", {
    ...summarizeBatchJob(pre),
    workerOrigin: workerOrigin ?? null,
    hasWorkerSecret: Boolean(process.env.OCR_WORKER_SECRET?.trim()),
  });

  if (pre.nextRunAt && Date.now() < new Date(pre.nextRunAt).getTime()) {
    const delay = Math.min(8_000, new Date(pre.nextRunAt).getTime() - Date.now());
    ocrTrace(jobId, "worker", "defer-nextRunAt", "nextRunAt futur — relance planifiée", {
      nextRunAt: pre.nextRunAt,
      delayMs: delay,
      selfChain,
    });
    if (selfChain) await scheduleWorkerContinuation(workerOrigin, jobId, delay);
    return;
  }

  if (!(await acquireRunLock(jobId))) {
    ocrTrace(jobId, "worker", "defer-lock", "lock non acquis — nouvelle tentative planifiée", { selfChain }, "warn");
    if (selfChain) await scheduleWorkerContinuation(workerOrigin, jobId, 4_000);
    return;
  }

  // Délai d'auto-relance serveur : > 0 si le worker s'interrompt avec du travail restant.
  let chainDelayMs: number | null = null;
  const originUrl = workerOrigin;

  try {
    let job = await readBatchJob(jobId);
    if (!job || job.status === "completed" || job.status === "failed") return;

    const odProfile = await getOdProfileForUser(job.userId);
    const knownStudents = await loadKnownStudentsForSegmentation(odProfile);
    const ctx: WorkerCtx = {
      jobId,
      token: job.accessToken,
      odProfile,
      refreshToken: await resolveServerRefreshToken(job, odProfile),
      knownStudents,
    };
    ocrTrace(jobId, "worker", "start", "worker démarré", {
      totalItems: job.items.length,
      currentItemIndex: job.currentItemIndex,
      profilOneDrive: odProfile ? `${odProfile.secteur} (${odProfile.basePath})` : null,
      refreshTokenServeur: Boolean(ctx.refreshToken),
      knownStudents: knownStudents.length,
      runBudgetMs: RUN_BUDGET_MS,
    });
    if (!odProfile) {
      ocrTrace(
        jobId,
        "worker",
        "warn-profile",
        "AUCUN profil OneDrive — risque élève non identifié",
        undefined,
        "warn",
      );
    }

    await patchJob(jobId, {
      status: "processing",
      processingStartedAt: new Date().toISOString(),
      nextRunAt: undefined,
    });

    const startedAt = Date.now();

    while (true) {
      const elapsedMs = Date.now() - startedAt;
      if (elapsedMs > RUN_BUDGET_MS) {
        ocrTrace(jobId, "worker", "budget", "budget invocation épuisé — reprise planifiée", {
          elapsedMs,
          runBudgetMs: RUN_BUDGET_MS,
          currentItemIndex: job.currentItemIndex,
          totalItems: job.items.length,
        });
        await patchJob(jobId, {
          status: "processing",
          nextRunAt: new Date().toISOString(),
          label: `Reprise automatique… (${job.currentItemIndex}/${job.items.length})`,
        });
        chainDelayMs = 0;
        return;
      }

      job = await readBatchJob(jobId);
      if (!job || job.status === "failed" || job.status === "completed" || job.status === "needs_token") {
        return;
      }

      if (job.currentItemIndex >= job.items.length) {
        // Garde anti-complétion prématurée : ne JAMAIS marquer « terminé » s'il reste un document
        // non clos (index avancé à tort par une écriture concurrente / un recalage). On recale
        // l'index sur le premier item inachevé plutôt que de figer le lot trop tôt.
        const firstUnfinished = job.items.findIndex(
          (it) => it.status !== "done" && it.status !== "failed",
        );
        if (firstUnfinished >= 0) {
          ocrTrace(jobId, "worker", "complete-guard", "fin de file mais document non terminé — recalage", {
            firstUnfinished,
            itemStatuses: job.items.map((it) => it.status),
          }, "warn");
          await patchJob(jobId, { currentItemIndex: firstUnfinished });
          continue;
        }
        const completed = job.results.filter((r) => r.success).length;
        const failed = job.results.filter((r) => !r.success).length;
        await patchJob(jobId, {
          status: "completed",
          percent: 100,
          completed,
          failed,
          nextRunAt: undefined,
          label: `Terminé — ${job.results.length} document${job.results.length > 1 ? "s" : ""} traité${job.results.length > 1 ? "s" : ""}`,
        });
        ocrTrace(jobId, "worker", "complete", "lot terminé", {
          completed,
          failed,
          totalResults: job.results.length,
          durationMs: Date.now() - invokeStartedAt,
        });
        await deleteOcrCacheForJob(job);
        return;
      }

      const itemIndex = job.currentItemIndex;
      const item = job.items[itemIndex];

      ocrTrace(jobId, "worker", "loop", "tour de boucle", {
        elapsedMs: Date.now() - startedAt,
        itemIndex,
        fileName: item?.fileName,
        phase: item?.phase ?? "ocr_start",
      });

      const claim = await resolveItemClaim(jobId, job, itemIndex);
      if (claim === "skip-advance") {
        await patchJob(jobId, { currentItemIndex: itemIndex + 1 });
        continue;
      }
      if (claim === "defer") {
        chainDelayMs = OCR_POLL_DELAY_MS;
        return;
      }

      try {
        const outcome = await stepItem(ctx, job, itemIndex);

        if (outcome.kind === "wait") {
          ocrTrace(jobId, "worker", "wait", "attente micro-étape", {
            delayMs: outcome.delayMs,
            label: outcome.label ?? null,
            elapsedMs: Date.now() - startedAt,
          });
          if (Date.now() - startedAt + outcome.delayMs < RUN_BUDGET_MS) {
            if (outcome.label) await patchJob(jobId, { label: outcome.label });
            await sleep(outcome.delayMs);
            continue;
          }
          ocrTrace(jobId, "worker", "defer-wait", "budget insuffisant pour wait — report nextRunAt", {
            delayMs: outcome.delayMs,
            elapsedMs: Date.now() - startedAt,
          });
          await patchJob(jobId, {
            status: "processing",
            nextRunAt: new Date(Date.now() + outcome.delayMs).toISOString(),
            label: outcome.label ?? job.label,
          });
          chainDelayMs = outcome.delayMs;
          return;
        }

        if (outcome.kind === "continue") {
          ocrTrace(jobId, "worker", "continue", "micro-étape continue sans résultat", {
            label: outcome.label ?? null,
          });
          if (outcome.label) await patchJob(jobId, { label: outcome.label });
          continue;
        }

        // outcome.kind === "result"
        const current = await readBatchJob(jobId);
        if (!current) return;
        const newResults = outcome.results.filter((r) => {
          if (current.results.some((ex) => ex.fileName === r.fileName && ex.success)) {
            ocrTrace(jobId, "worker", "dedup", "doublon ignoré (déjà succès)", { fileName: r.fileName });
            return false;
          }
          return true;
        });
        const nextResults = mergeBatchResults(current.results, newResults);
        const nextIndex = outcome.itemDone ? itemIndex + 1 : itemIndex;
        const itemSuccess =
          outcome.itemDone &&
          outcome.results.every(
            (r) =>
              r.success ||
              current.results.some((ex) => ex.fileName === r.fileName && ex.success),
          );
        const prog = computeProgress({ ...current, results: nextResults, currentItemIndex: nextIndex });
        ocrTrace(jobId, "worker", "result", "résultats micro-étape enregistrés", {
          newResults: newResults.map((r) => ({ fileName: r.fileName, success: r.success, error: r.error })),
          itemDone: outcome.itemDone,
          nextItemIndex: nextIndex,
          percent: prog.percent,
          itemSuccess,
        });
        await writeBatchJob({
          ...current,
          results: nextResults,
          currentItemIndex: nextIndex,
          items: current.items.map((it, i) =>
            i === itemIndex
              ? {
                  ...it,
                  errorCount: 0,
                  ...(outcome.itemDone
                    ? { status: itemSuccess ? ("done" as const) : ("failed" as const), itemClaimedAt: undefined }
                    : {}),
                }
              : it,
          ),
          percent: prog.percent,
          completed: prog.completed,
          failed: prog.failed,
          label: outcome.label ?? current.label,
          updatedAt: new Date().toISOString(),
        });
        continue;
      } catch (err) {
        if (err instanceof TokenExpiredError) {
          ocrTrace(jobId, "worker", "needs_token", "session OneDrive expirée", {
            fileName: item.fileName,
          }, "warn");
          await patchJob(jobId, {
            status: "needs_token",
            error: "Session OneDrive expirée. Reconnectez Microsoft sur la page pour reprendre.",
            label: "En attente de reconnexion OneDrive…",
          });
          return;
        }
        const message = err instanceof Error ? err.message : String(err);
        const current = await readBatchJob(jobId);
        if (!current) return;
        const prevErrors = current.items[itemIndex]?.errorCount ?? 0;
        const errorCount = prevErrors + 1;
        if (errorCount < MAX_ITEM_ERRORS) {
          // Erreur probablement transitoire (S3 / réseau / Graph) : on NE fait PAS échouer tout
          // le document. On réessaie le même item depuis sa phase courante (segmentIndex conservé),
          // ce qui évite qu'un simple hoquet S3 abandonne un PDF classe à moitié traité.
          ocrTrace(jobId, "worker", "item-retry", "échec technique item — nouvelle tentative", {
            fileName: item.fileName,
            error: message,
            attempt: errorCount,
            maxAttempts: MAX_ITEM_ERRORS,
          }, "warn");
          await patchItem(jobId, itemIndex, {
            errorCount,
            itemClaimedAt: undefined,
            status: "processing",
          });
          await sleep(ITEM_RETRY_DELAY_MS);
          continue;
        }
        ocrTrace(jobId, "worker", "item-error", "échec technique item (définitif après tentatives)", {
          fileName: item.fileName,
          error: message,
          attempts: errorCount,
        }, "error");
        await writeBatchJob({
          ...current,
          results: [
            ...current.results,
            { success: false, error: message, fileName: item.fileName, tempOneDrivePath: item.tempPath },
          ],
          items: current.items.map((it, i) =>
            i === itemIndex ? { ...it, status: "failed", itemClaimedAt: undefined, errorCount } : it,
          ),
          currentItemIndex: itemIndex + 1,
          updatedAt: new Date().toISOString(),
        });
        continue;
      }
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    // Ne jamais faire échouer le lot pour une course S3 / lock (retry au prochain chunk).
    if (/conflicting conditional operation/i.test(msg)) {
      ocrTrace(jobId, "worker", "lock-race", "course S3 ignorée — le client / worker réessaiera", {
        error: msg.slice(0, 200),
      }, "warn");
      return;
    }
    ocrTrace(jobId, "worker", "fatal", "erreur fatale worker", {
      error: msg,
      stack: error instanceof Error ? error.stack?.slice(0, 500) : undefined,
    }, "error");
    const j = await readBatchJob(jobId);
    if (j && j.status !== "completed") {
      await patchJob(jobId, {
        status: "failed",
        error: msg,
        label: "Échec du traitement",
      });
    }
  } finally {
    await releaseRunLock(jobId);
    const snapshot = await readBatchJob(jobId);
    const stillRunning =
      snapshot &&
      snapshot.status === "processing" &&
      snapshot.currentItemIndex < snapshot.items.length;
    const chainOrigin = resolveWorkerOrigin(snapshot) ?? originUrl;

    if (!selfChain) {
      // Mode piloté par le client : pas d'auto-relance serveur, le prochain /process reprendra.
      ocrTrace(jobId, "relay", "client-driven", "fin de chunk — reprise déléguée au client (pas d'auto-relance)", {
        stillRunning,
        chainDelayMs,
      });
    } else if (chainDelayMs !== null) {
      ocrTrace(jobId, "relay", "chain", "auto-relance planifiée (finally)", {
        chainDelayMs,
        origin: chainOrigin ?? null,
        stillRunning,
      });
      await scheduleWorkerContinuation(chainOrigin, jobId, chainDelayMs);
    } else if (stillRunning) {
      ocrTrace(jobId, "relay", "safety", "lot incomplet — relance de sécurité serveur", {
        origin: chainOrigin ?? null,
        ...summarizeBatchJob(snapshot!),
      }, "warn");
      await scheduleWorkerContinuation(chainOrigin, jobId, 0);
    }
    ocrTrace(jobId, "worker", "invoke-end", "fin invocation runOcrBatchJob", {
      durationMs: Date.now() - invokeStartedAt,
      stillRunning,
      chainDelayMs,
    });
    await flushOcrJobTraces(jobId);
  }
}

export async function resumeBatchJobWithToken(jobId: string, accessToken: string) {
  const job = await readBatchJob(jobId);
  if (!job) return null;
  if (job.status !== "needs_token") return job;
  const next: OcrBatchJob = {
    ...job,
    accessToken,
    status: "processing",
    error: undefined,
    processingStartedAt: new Date().toISOString(),
    nextRunAt: undefined,
    label: "Reprise du traitement…",
  };
  await writeBatchJob(next);
  return next;
}

/** Met à jour le token OneDrive sur un job actif (sans interrompre le traitement). */
export async function refreshBatchJobAccessToken(jobId: string, accessToken: string) {
  const job = await readBatchJob(jobId);
  if (!job) return null;
  if (job.status === "completed" || job.status === "failed") return job;
  await writeBatchJob({ ...job, accessToken });
  return job;
}

/** Arrête un lot OCR serveur en cours (fichiers déjà traités conservés dans les résultats). */
export async function cancelBatchJob(jobId: string): Promise<OcrBatchJob | null> {
  const job = await readBatchJob(jobId);
  if (!job) return null;
  if (job.status === "completed" || job.status === "failed") return job;

  const cancelled: OcrBatchJob = {
    ...job,
    status: "failed",
    error: "Traitement annulé par l'utilisateur.",
    label: "Traitement annulé",
    updatedAt: new Date().toISOString(),
  };
  await writeBatchJob(cancelled);
  await releaseRunLock(jobId);
  return cancelled;
}
