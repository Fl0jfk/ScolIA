import type { OcrBatchJob, OcrBatchJobItem } from "@/app/api/agentIAOCR/batch-job/batch-job";
import { resolveSegmentationEngine, type SegmentationEngine } from "@/app/lib/ocr-segment-run";

type OcrBatchProgressPhase =
  | "pending"
  | "ocr"
  | "segmenting"
  | "segments"
  | "analyze"
  | "done"
  | "idle";

type OcrBatchProgressView = {
  percent: number;
  label: string;
  phase: OcrBatchProgressPhase;
  phaseLabel: string;
  fileName: string | null;
  fileIndex: number;
  fileTotal: number;
  pageCount: number | null;
  pdfPageCount: number | null;
  ocrPagesRead: number | null;
  segmentIndex: number | null;
  segmentTotal: number | null;
  segmentationEngine: SegmentationEngine | null;
  documentsProcessed: number;
  documentsSucceeded: number;
  documentsFailed: number;
  updatedAt: string;
  idleSeconds: number;
};

/** Nombre de pages connu uniquement après lecture OCR (pas pendant ocr_start / ocr_poll). */
function isPageCountKnown(item: OcrBatchJobItem | null): boolean {
  if (!item?.pageCount || item.pageCount < 1) return false;
  const phase = item.phase ?? "ocr_start";
  return phase !== "ocr_start" && phase !== "ocr_poll";
}

/** Nombre de documents connus uniquement une fois le découpage terminé. */
function areSegmentsKnown(item: OcrBatchJobItem | null): boolean {
  return item?.phase === "segments" && (item.segments?.length ?? 0) > 0;
}

function itemWeight(item: OcrBatchJobItem): number {
  if (item.segments && item.segments.length > 0) return item.segments.length;
  // Avant découpage : on ne devine pas le nombre de documents à partir des pages.
  return 1;
}

function itemCompletedWeight(item: OcrBatchJobItem): number {
  const w = itemWeight(item);
  if (item.status === "done" || item.status === "failed") return w;
  if (item.phase === "segments" && item.segments?.length) {
    return Math.min(item.segmentIndex ?? 0, item.segments.length);
  }
  if (item.phase === "segmenting") return 0.7;
  if (item.phase === "analyze") return 0.9;
  if (item.phase === "ocr_poll" || item.phase === "ocr_start") {
    const total = item.pdfPageCount;
    const read = item.ocrPagesRead ?? 0;
    if (total && total > 0 && read > 0) return Math.min(0.88, (read / total) * 0.88);
    if (total && total > 0) return 0.1;
  }
  if (item.phase === "ocr_poll") return 0.4;
  if (item.phase === "ocr_start") return 0.08;
  return 0;
}

function resolveUiPhase(item: OcrBatchJobItem | null): OcrBatchProgressPhase {
  if (!item) return "idle";
  if (item.status === "done" || item.status === "failed") return "done";
  const phase = item.phase ?? "ocr_start";
  if (phase === "ocr_start" || phase === "ocr_poll") return "ocr";
  if (phase === "segmenting") return "segmenting";
  if (phase === "segments") return "segments";
  if (phase === "analyze") return "analyze";
  return "pending";
}

function resolveSegmentationEngineForItem(item: OcrBatchJobItem | null): SegmentationEngine | null {
  if (!item) return null;
  if (item.segmentationEngine) return item.segmentationEngine;
  const pages = item.pageCount ?? item.pdfPageCount;
  if (!pages || pages < 1) return null;
  if ((item.phase ?? "ocr_start") === "ocr_start" || item.phase === "ocr_poll") return null;
  return resolveSegmentationEngine(pages);
}

function segmentingPhaseLabel(item: OcrBatchJobItem | null): string {
  const engine = resolveSegmentationEngineForItem(item);
  if (engine === "heuristic") return "Mistral a lu le PDF — découpage automatique";
  if (engine === "mistral_chunked") return "Mistral en déduit le découpage (par blocs)";
  if (engine === "mistral") return "Mistral en déduit le découpage";
  return "Mistral prépare le découpage";
}

function buildLabel(job: OcrBatchJob, item: OcrBatchJobItem | null, phase: OcrBatchProgressPhase): string {
  if (!item) {
    if (job.status === "completed") {
      return `Terminé — ${job.results.length} document${job.results.length > 1 ? "s" : ""} traité${job.results.length > 1 ? "s" : ""}`;
    }
    if (job.status === "cancelled") {
      return job.label || "Traitement annulé";
    }
    return job.label || "Traitement en cours…";
  }

  const name = item.fileName;
  const pagesKnown = isPageCountKnown(item);
  const pages = pagesKnown ? item.pageCount : undefined;

  if (phase === "ocr") {
    const total = item.pdfPageCount;
    const read = item.ocrPagesRead ?? 0;
    if (total && read > 0) {
      return `Mistral lit le document — ${name} : page ${read} / ${total}…`;
    }
    if (total) {
      return `Mistral analyse votre document — ${name} (0 / ${total} page(s))…`;
    }
    return `Mistral analyse votre document — ${name}…`;
  }

  if (phase === "segmenting") {
    const engine = resolveSegmentationEngineForItem(item);
    if (engine === "heuristic") {
      return pages
        ? `Mistral a terminé la lecture — repérage automatique sur ${pages} page${pages > 1 ? "s" : ""}…`
        : `Mistral a terminé la lecture — repérage automatique des documents…`;
    }
    if (engine === "mistral_chunked") {
      return pages
        ? `Mistral en déduit le découpage de ${pages} pages (par blocs, aux frontières)…`
        : `Mistral en déduit le découpage par blocs…`;
    }
    if (engine === "mistral") {
      return pages
        ? `Mistral en déduit le découpage sur ${pages} page${pages > 1 ? "s" : ""}…`
        : `Mistral en déduit le découpage du PDF…`;
    }
    return pages
      ? `Mistral prépare le découpage (${pages} page${pages > 1 ? "s" : ""})…`
      : `Mistral prépare le découpage…`;
  }

  if (phase === "segments" && item.segments?.length) {
    const total = item.segments.length;
    const current = Math.min((item.segmentIndex ?? 0) + 1, total);
    const seg = item.segments[item.segmentIndex ?? 0];
    const pageHint = seg ? ` (pages ${seg.pageStart}–${seg.pageEnd})` : "";
    return `Mistral déduit le nom et le rangement — document ${current}/${total} — ${name}${pageHint}`;
  }

  if (phase === "analyze") {
    return `Mistral déduit le nom et où ranger — ${name}`;
  }

  return job.label || `Traitement — ${name}`;
}

/** Progression fine (segments, pages OCR) pour l'UI et l'API status. */
export function buildBatchProgressView(job: OcrBatchJob): OcrBatchProgressView {
  const fileTotal = job.items.length;
  const fileIndex = Math.min(job.currentItemIndex + 1, Math.max(fileTotal, 1));
  const currentItem = job.items[job.currentItemIndex] ?? null;

  let totalWeight = 0;
  let doneWeight = 0;
  for (let i = 0; i < job.items.length; i++) {
    const item = job.items[i]!;
    const w = itemWeight(item);
    totalWeight += w;
    if (i < job.currentItemIndex) {
      doneWeight += w;
    } else if (i === job.currentItemIndex) {
      doneWeight += itemCompletedWeight(item);
    }
  }

  const percent =
    job.status === "completed"
      ? 100
      : totalWeight > 0
        ? Math.min(99, Math.round((doneWeight / totalWeight) * 100))
        : job.percent;

  const phase = job.status === "completed" ? "done" : resolveUiPhase(currentItem);
  const phaseLabel =
    phase === "segmenting"
      ? segmentingPhaseLabel(currentItem)
      : phase === "segments"
        ? "Mistral déduit le nom et le rangement"
        : phase === "ocr"
          ? "Mistral analyse le document (OCR)"
          : phase === "analyze"
            ? "Mistral déduit le nom et le rangement"
            : phase === "done"
              ? "Terminé"
              : phase === "pending"
                ? "En attente"
                : "—";
  const label = buildLabel(job, currentItem, phase);
  const updatedAt = job.updatedAt || job.startedAt;
  const idleSeconds = Math.max(0, Math.round((Date.now() - new Date(updatedAt).getTime()) / 1000));

  return {
    percent,
    label,
    phase,
    phaseLabel,
    fileName: currentItem?.fileName ?? null,
    fileIndex: fileTotal > 0 ? fileIndex : 0,
    fileTotal,
    pageCount: isPageCountKnown(currentItem) ? (currentItem?.pageCount ?? null) : null,
    pdfPageCount: currentItem?.pdfPageCount ?? null,
    ocrPagesRead:
      resolveUiPhase(currentItem) === "ocr" ? (currentItem?.ocrPagesRead ?? null) : null,
    segmentIndex: areSegmentsKnown(currentItem)
      ? Math.min((currentItem!.segmentIndex ?? 0) + 1, currentItem!.segments!.length)
      : null,
    segmentTotal: areSegmentsKnown(currentItem) ? (currentItem!.segments!.length ?? null) : null,
    segmentationEngine: resolveSegmentationEngineForItem(currentItem),
    documentsProcessed: job.results.length,
    documentsSucceeded: job.results.filter((r) => r.success).length,
    documentsFailed: job.results.filter((r) => !r.success).length,
    updatedAt,
    idleSeconds,
  };
}

export function computeProgress(job: OcrBatchJob) {
  const view = buildBatchProgressView(job);
  return {
    percent: view.percent,
    completed: view.documentsSucceeded,
    failed: view.documentsFailed,
  };
}
