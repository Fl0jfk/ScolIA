import "server-only";

import type { OcrBatchJob, OcrBatchJobItem, OcrJobTraceEntry } from "@/app/api/agentIAOCR/batch-job/batch-job";
import { flushOcrJobTraces, queueOcrJobTrace } from "@/app/lib/ocr-job-trace-store";

export type { OcrJobTraceEntry };

/** Contexte de traçage propagé dans le pipeline (greppable CloudWatch : `[ocr-batch <jobId>]`). */
export type OcrTraceCtx = {
  batchJobId: string;
  fileName?: string;
  itemIndex?: number;
};

export type OcrTraceScope =
  | "api"
  | "worker"
  | "lock"
  | "relay"
  | "item"
  | "textract"
  | "segment"
  | "classify"
  | "onedrive";

export type OcrTraceLevel = "debug" | "info" | "warn" | "error";

function serializeData(data?: Record<string, unknown>): string {
  if (!data || Object.keys(data).length === 0) return "";
  try {
    return ` | ${JSON.stringify(data)}`;
  } catch {
    return " | [data non sérialisable]";
  }
}

/** Log structuré OCR — console (CloudWatch si dispo) + journal persistant S3 sur le job. */
export function ocrTrace(
  batchJobId: string,
  scope: OcrTraceScope,
  phase: string,
  message: string,
  data?: Record<string, unknown>,
  level: OcrTraceLevel = "info",
): void {
  const line = `[ocr-batch ${batchJobId}] [${scope}] [${phase}] ${message}${serializeData(data)}`;
  // stderr est souvent mieux capturé par les logs container que stdout.
  if (level === "error") console.error(line);
  else if (level === "warn") console.warn(line);
  else console.error(line);

  if (batchJobId && batchJobId.length >= 8) {
    queueOcrJobTrace(batchJobId, scope, phase, message, data, level);
  }
}

export function ocrTraceCtx(
  ctx: OcrTraceCtx | undefined,
  scope: OcrTraceScope,
  phase: string,
  message: string,
  data?: Record<string, unknown>,
  level: OcrTraceLevel = "info",
): void {
  if (!ctx?.batchJobId) return;
  ocrTrace(ctx.batchJobId, scope, phase, message, { ...data, fileName: ctx.fileName, itemIndex: ctx.itemIndex }, level);
}

export function summarizeBatchItem(item: OcrBatchJobItem): Record<string, unknown> {
  return {
    fileName: item.fileName,
    mode: item.mode,
    status: item.status,
    phase: item.phase ?? "ocr_start",
    s3Key: item.s3Key,
    tempPath: item.tempPath,
    pdfPageCount: item.pdfPageCount ?? null,
    ocrPagesRead: item.ocrPagesRead ?? null,
    pageCount: item.pageCount ?? null,
    segmentIndex: item.segmentIndex ?? null,
    segmentsTotal: item.segments?.length ?? null,
    segmentationEngine: item.segmentationEngine ?? null,
    textractJobId: item.textractJobId ?? null,
    hasOcrCache: Boolean(item.ocrCacheKey),
    itemClaimedAt: item.itemClaimedAt ?? null,
  };
}

export function summarizeBatchJob(job: OcrBatchJob): Record<string, unknown> {
  const item = job.items[job.currentItemIndex];
  return {
    status: job.status,
    currentItemIndex: job.currentItemIndex,
    totalItems: job.items.length,
    percent: job.percent,
    completed: job.completed,
    failed: job.failed,
    resultsCount: job.results.length,
    resultsOk: job.results.filter((r) => r.success).length,
    resultsKo: job.results.filter((r) => !r.success).length,
    label: job.label,
    nextRunAt: job.nextRunAt ?? null,
    updatedAt: job.updatedAt,
    originUrl: job.originUrl ?? null,
    currentItem: item ? summarizeBatchItem(item) : null,
    allItems: job.items.map((it, i) => ({
      index: i,
      fileName: it.fileName,
      mode: it.mode,
      status: it.status,
      phase: it.phase ?? "ocr_start",
    })),
  };
}
