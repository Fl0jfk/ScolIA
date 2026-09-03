import "server-only";

import { and, eq, isNull, lt, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { absenceIngestJob } from "@/db/schema";
import { canAdminIngest } from "@/app/lib/absences-types";
import { absencesDbReady } from "@/app/lib/absence-db";

export type IngestJobStatus = "pending" | "processing" | "completed" | "failed";
export type IngestJobPhase = "ocr" | "ai" | "saving";

export type IngestJobCreated = {
  id: string;
  teacherName: string;
  startDate: string;
  endDate: string;
};

export type IngestJob = {
  jobId: string;
  userId: string;
  creatorName: string;
  creatorEmail: string;
  creatorRoles: string[];
  status: IngestJobStatus;
  startedAt: string;
  updatedAt: string;
  sourceFileName: string;
  documentKey: string;
  processingStartedAt?: string;
  phase?: IngestJobPhase;
  error?: string;
  code?: string;
  created?: IngestJobCreated[];
  parsed?: Record<string, unknown>;
};

const LOCK_TTL_MS = 15 * 60_000;

export function canIngestFromUser(roles: string[]) {
  return canAdminIngest(roles);
}

export function newJobId() {
  return `${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
}

function parseCreated(raw: string | null): IngestJobCreated[] | undefined {
  if (!raw?.trim()) return undefined;
  try {
    const v = JSON.parse(raw) as unknown;
    return Array.isArray(v) ? (v as IngestJobCreated[]) : undefined;
  } catch {
    return undefined;
  }
}

function parseParsed(raw: string | null): Record<string, unknown> | undefined {
  if (!raw?.trim()) return undefined;
  try {
    const v = JSON.parse(raw) as unknown;
    return v && typeof v === "object" && !Array.isArray(v)
      ? (v as Record<string, unknown>)
      : undefined;
  } catch {
    return undefined;
  }
}

function rowToJob(row: typeof absenceIngestJob.$inferSelect): IngestJob {
  return {
    jobId: row.jobId,
    userId: row.userId,
    creatorName: row.creatorName,
    creatorEmail: row.creatorEmail,
    creatorRoles: row.creatorRoles ?? [],
    status: row.status as IngestJobStatus,
    startedAt: row.startedAt.toISOString(),
    updatedAt: row.updatedAt.toISOString(),
    sourceFileName: row.sourceFileName,
    documentKey: row.documentKey,
    ...(row.processingStartedAt
      ? { processingStartedAt: row.processingStartedAt.toISOString() }
      : {}),
    ...(row.phase ? { phase: row.phase as IngestJobPhase } : {}),
    ...(row.error ? { error: row.error } : {}),
    ...(row.code ? { code: row.code } : {}),
    ...(parseCreated(row.createdPayload) ? { created: parseCreated(row.createdPayload) } : {}),
    ...(parseParsed(row.parsedPayload) ? { parsed: parseParsed(row.parsedPayload) } : {}),
  };
}

export async function readIngestJob(jobId: string): Promise<IngestJob | null> {
  const etabId = await absencesDbReady();
  if (!etabId) return null;
  const db = getDb();
  const [row] = await db
    .select()
    .from(absenceIngestJob)
    .where(and(eq(absenceIngestJob.etablissementId, etabId), eq(absenceIngestJob.jobId, jobId)))
    .limit(1);
  return row ? rowToJob(row) : null;
}

export async function writeIngestJob(job: IngestJob): Promise<void> {
  const etabId = await absencesDbReady();
  if (!etabId) throw new Error("[absences] Postgres requis pour les jobs d’ingest");
  const db = getDb();
  const now = new Date();
  const startedAt = new Date(job.startedAt);
  const updatedAt = now;
  const values = {
    jobId: job.jobId,
    etablissementId: etabId,
    userId: job.userId,
    creatorName: job.creatorName || "",
    creatorEmail: job.creatorEmail || "",
    creatorRoles: Array.isArray(job.creatorRoles) ? job.creatorRoles.map(String) : [],
    status: job.status,
    startedAt: Number.isNaN(startedAt.getTime()) ? now : startedAt,
    updatedAt,
    sourceFileName: job.sourceFileName || "",
    documentKey: job.documentKey,
    processingStartedAt: job.processingStartedAt
      ? new Date(job.processingStartedAt)
      : null,
    phase: job.phase ?? null,
    error: job.error ?? null,
    code: job.code ?? null,
    createdPayload: job.created ? JSON.stringify(job.created) : null,
    parsedPayload: job.parsed ? JSON.stringify(job.parsed) : null,
  };
  await db
    .insert(absenceIngestJob)
    .values(values)
    .onConflictDoUpdate({
      target: absenceIngestJob.jobId,
      set: {
        userId: values.userId,
        creatorName: values.creatorName,
        creatorEmail: values.creatorEmail,
        creatorRoles: values.creatorRoles,
        status: values.status,
        updatedAt: values.updatedAt,
        sourceFileName: values.sourceFileName,
        documentKey: values.documentKey,
        processingStartedAt: values.processingStartedAt,
        phase: values.phase,
        error: values.error,
        code: values.code,
        createdPayload: values.createdPayload,
        parsedPayload: values.parsedPayload,
      },
    });
}

/** Verrou worker Postgres (TTL) — remplace IfNoneMatch S3. */
export async function acquireIngestJobLock(jobId: string, workerId: string): Promise<boolean> {
  const etabId = await absencesDbReady();
  if (!etabId) return false;
  const db = getDb();
  const now = new Date();
  const staleBefore = new Date(now.getTime() - LOCK_TTL_MS);
  const updated = await db
    .update(absenceIngestJob)
    .set({ lockedAt: now, lockedBy: workerId, updatedAt: now })
    .where(
      and(
        eq(absenceIngestJob.etablissementId, etabId),
        eq(absenceIngestJob.jobId, jobId),
        or(isNull(absenceIngestJob.lockedAt), lt(absenceIngestJob.lockedAt, staleBefore)),
      ),
    )
    .returning({ jobId: absenceIngestJob.jobId });
  return updated.length > 0;
}

export async function releaseIngestJobLock(jobId: string, workerId?: string): Promise<void> {
  const etabId = await absencesDbReady();
  if (!etabId) return;
  const db = getDb();
  const conditions = [
    eq(absenceIngestJob.etablissementId, etabId),
    eq(absenceIngestJob.jobId, jobId),
  ];
  if (workerId) {
    conditions.push(eq(absenceIngestJob.lockedBy, workerId));
  }
  await db
    .update(absenceIngestJob)
    .set({ lockedAt: null, lockedBy: null, updatedAt: new Date() })
    .where(and(...conditions));
}

/** Nettoyage opportuniste des jobs terminés > 7 jours. */
export async function purgeOldIngestJobs(days = 7): Promise<number> {
  const etabId = await absencesDbReady();
  if (!etabId) return 0;
  const db = getDb();
  const cutoff = new Date(Date.now() - days * 24 * 60 * 60 * 1000);
  const deleted = await db
    .delete(absenceIngestJob)
    .where(
      and(
        eq(absenceIngestJob.etablissementId, etabId),
        sql`${absenceIngestJob.status} in ('completed', 'failed')`,
        lt(absenceIngestJob.updatedAt, cutoff),
      ),
    )
    .returning({ jobId: absenceIngestJob.jobId });
  return deleted.length;
}
