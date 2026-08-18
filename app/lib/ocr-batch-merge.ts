import type {
  OcrBatchJob,
  OcrBatchJobItem,
  OcrBatchItemPhase,
  OcrBatchResult,
  OcrBatchSegment,
} from "@/app/api/agentIAOCR/batch-job/batch-job";

export function ocrSegmentResultLabel(
  fileName: string,
  pageStart: number,
  pageEnd: number,
): string {
  return `${fileName} [p.${pageStart}-${pageEnd}]`;
}

/** Fusionne les résultats sans perdre un succès déjà enregistré. */
export function mergeBatchResults(
  existing: OcrBatchResult[],
  incoming: OcrBatchResult[],
): OcrBatchResult[] {
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

export function segmentHasResult(
  results: OcrBatchResult[],
  fileName: string,
  seg: Pick<OcrBatchSegment, "pageStart" | "pageEnd">,
): boolean {
  const label = ocrSegmentResultLabel(fileName, seg.pageStart, seg.pageEnd);
  return results.some((r) => r.fileName === label);
}

export function firstUnfinishedSegmentIndex(
  item: Pick<OcrBatchJobItem, "fileName" | "segments">,
  results: OcrBatchResult[],
): number {
  const segs = item.segments ?? [];
  if (segs.length === 0) return 0;
  const idx = segs.findIndex((s) => !segmentHasResult(results, item.fileName, s));
  return idx < 0 ? segs.length : idx;
}

export function allItemSegmentsCovered(
  item: Pick<OcrBatchJobItem, "fileName" | "segments" | "status">,
  results: OcrBatchResult[],
): boolean {
  const segs = item.segments ?? [];
  if (segs.length > 0) {
    return segs.every((s) => segmentHasResult(results, item.fileName, s));
  }
  return (
    item.status === "done" ||
    item.status === "failed" ||
    results.some((r) => r.fileName === item.fileName)
  );
}

export function isOcrBatchJobFullyCovered(job: OcrBatchJob): boolean {
  if (!job.items.length) return false;
  return job.items.every((item) => allItemSegmentsCovered(item, job.results));
}

export function firstUnfinishedItemIndex(job: Pick<OcrBatchJob, "items" | "results">): number {
  return job.items.findIndex((item) => !allItemSegmentsCovered(item, job.results));
}

const PHASE_ORDER: OcrBatchItemPhase[] = [
  "ocr_start",
  "ocr_poll",
  "analyze",
  "segmenting",
  "segments",
];

function preferPhase(
  a?: OcrBatchItemPhase,
  b?: OcrBatchItemPhase,
): OcrBatchItemPhase | undefined {
  const ia = a ? PHASE_ORDER.indexOf(a) : -1;
  const ib = b ? PHASE_ORDER.indexOf(b) : -1;
  if (ib >= ia) return b ?? a;
  return a ?? b;
}

function mergeItem(ex: OcrBatchJobItem, inc: OcrBatchJobItem, results: OcrBatchResult[]): OcrBatchJobItem {
  const exSegs = ex.segments?.length ?? 0;
  const incSegs = inc.segments?.length ?? 0;
  const richer = incSegs >= exSegs ? inc : ex;
  const other = richer === inc ? ex : inc;
  const merged: OcrBatchJobItem = {
    ...other,
    ...richer,
    ocrCacheKey: richer.ocrCacheKey || other.ocrCacheKey,
    textractJobId: richer.textractJobId || other.textractJobId,
    phase: preferPhase(ex.phase, inc.phase),
    segments: (incSegs >= exSegs ? inc.segments : ex.segments) ?? richer.segments,
  };

  if ((merged.segments?.length ?? 0) > 0) {
    merged.segmentIndex = firstUnfinishedSegmentIndex(merged, results);
    if (allItemSegmentsCovered(merged, results)) {
      merged.status = "done";
      merged.itemClaimedAt = undefined;
    } else if (merged.status === "done" || merged.status === "failed") {
      merged.status = "processing";
    }
    return merged;
  }

  if (allItemSegmentsCovered(merged, results)) {
    if (merged.status !== "failed") merged.status = "done";
    merged.itemClaimedAt = undefined;
  }
  return merged;
}

/**
 * Fusionne deux snapshots du même lot. Les résultats déjà obtenus ne sont jamais
 * écrasés ; un PDF classe n'est « terminé » que si chaque segment a un résultat.
 */
export function mergeOcrBatchJobs(existing: OcrBatchJob, incoming: OcrBatchJob): OcrBatchJob {
  if (existing.status === "cancelled") return existing;
  if (incoming.status === "cancelled") {
    return {
      ...existing,
      ...incoming,
      status: "cancelled",
      results: mergeBatchResults(existing.results, incoming.results),
    };
  }

  const results = mergeBatchResults(existing.results, incoming.results);
  const itemCount = Math.max(existing.items.length, incoming.items.length);
  const items: OcrBatchJobItem[] = [];
  for (let i = 0; i < itemCount; i++) {
    const ex = existing.items[i];
    const inc = incoming.items[i];
    if (ex && inc) items.push(mergeItem(ex, inc, results));
    else items.push((inc ?? ex)!);
  }

  const unfinished = firstUnfinishedItemIndex({ items, results });
  const allCovered = isOcrBatchJobFullyCovered({
    ...incoming,
    items,
    results,
  });

  let status = incoming.status;
  if (existing.status === "needs_token" && incoming.status !== "completed") {
    status = incoming.status === "processing" || incoming.status === "pending" ? incoming.status : "needs_token";
  }
  if (!allCovered) {
    if (status === "completed" || status === "failed") status = "processing";
  } else if (status !== "needs_token") {
    status = "completed";
  }

  return {
    ...existing,
    ...incoming,
    results,
    items,
    status,
    currentItemIndex: allCovered ? items.length : Math.max(0, unfinished),
    error: status === "processing" || status === "pending" ? undefined : incoming.error ?? existing.error,
  };
}

/** Si un lot « terminé » a encore des documents sans résultat, le rouvre. */
export function reopenIncompleteOcrBatchJob(job: OcrBatchJob): OcrBatchJob | null {
  if (job.status === "cancelled" || job.status === "needs_token") return null;
  if (isOcrBatchJobFullyCovered(job)) return null;
  return mergeOcrBatchJobs(job, { ...job, status: "processing", error: undefined });
}
