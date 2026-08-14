import "server-only";

import { readBatchJob, writeBatchJob, type OcrJobTraceEntry } from "@/app/api/agentIAOCR/batch-job/batch-job";

type TraceLevel = "debug" | "info" | "warn" | "error";

const MAX_TRACE_ENTRIES = 200;
const pending = new Map<string, OcrJobTraceEntry[]>();
const flushTimers = new Map<string, ReturnType<typeof setTimeout>>();

/** File d'attente d'une entrée de trace (flush groupé vers S3 / job JSON). */
export function queueOcrJobTrace(
  jobId: string,
  scope: string,
  phase: string,
  message: string,
  data?: Record<string, unknown>,
  level: TraceLevel = "info",
): void {
  const batch = pending.get(jobId) ?? [];
  batch.push({
    t: new Date().toISOString(),
    scope,
    phase,
    level,
    message,
    data: data && Object.keys(data).length > 0 ? data : undefined,
  });
  pending.set(jobId, batch);

  if (batch.length >= 25) {
    void flushOcrJobTraces(jobId);
    return;
  }

  if (!flushTimers.has(jobId)) {
    flushTimers.set(
      jobId,
      setTimeout(() => {
        flushTimers.delete(jobId);
        void flushOcrJobTraces(jobId);
      }, 400),
    );
  }
}

/** Écrit les traces en attente dans le JSON du job (visible via /status). */
export async function flushOcrJobTraces(jobId: string): Promise<void> {
  const batch = pending.get(jobId);
  if (!batch?.length) return;
  pending.delete(jobId);
  const timer = flushTimers.get(jobId);
  if (timer) {
    clearTimeout(timer);
    flushTimers.delete(jobId);
  }

  try {
    const job = await readBatchJob(jobId);
    if (!job) return;
    const prev = job.traceTail ?? [];
    const next = [...prev, ...batch];
    const trimmed = next.length > MAX_TRACE_ENTRIES ? next.slice(-MAX_TRACE_ENTRIES) : next;
    await writeBatchJob({ ...job, traceTail: trimmed });
  } catch (err) {
    console.error(`[ocr-batch ${jobId}] [trace-store] flush échoué:`, err);
  }
}

export function getOcrJobTraceTail(job: { traceTail?: OcrJobTraceEntry[] }): OcrJobTraceEntry[] {
  return job.traceTail ?? [];
}
