import assert from "node:assert/strict";
import test from "node:test";
import type { OcrBatchJob, OcrBatchJobItem, OcrBatchResult } from "../api/agentIAOCR/batch-job/batch-job";
import {
  allItemSegmentsCovered,
  firstUnfinishedSegmentIndex,
  isOcrBatchJobFullyCovered,
  mergeOcrBatchJobs,
  ocrSegmentResultLabel,
  reopenIncompleteOcrBatchJob,
} from "./ocr-batch-merge";

function item(fileName: string, pages: Array<[number, number]>): OcrBatchJobItem {
  return {
    id: "item_1",
    fileName,
    mode: "class",
    s3Key: "k",
    tempPath: "Temp/x.pdf",
    status: "processing",
    phase: "segments",
    segments: pages.map(([pageStart, pageEnd]) => ({ pageStart, pageEnd })),
    segmentIndex: 0,
  };
}

function job(partial: Partial<OcrBatchJob> & { items: OcrBatchJobItem[]; results: OcrBatchResult[] }): OcrBatchJob {
  return {
    jobId: "j1",
    userId: "u1",
    status: "processing",
    startedAt: "t0",
    updatedAt: "t0",
    accessToken: "tok",
    currentItemIndex: 0,
    label: "",
    percent: 0,
    completed: 0,
    failed: 0,
    ...partial,
  };
}

function ok(fileName: string, a: number, b: number): OcrBatchResult {
  return { success: true, fileName: ocrSegmentResultLabel(fileName, a, b) };
}

test("8 segments : une écriture périmée ne fait pas disparaître les documents manquants", () => {
  const file = "SCAN.pdf";
  const pages: Array<[number, number]> = [
    [1, 2],
    [3, 4],
    [5, 6],
    [7, 8],
    [9, 10],
    [11, 12],
    [13, 14],
    [15, 16],
  ];
  const it = item(file, pages);
  const workerA = job({
    items: [{ ...it, status: "done", segmentIndex: 8 }],
    results: [ok(file, 1, 2), ok(file, 3, 4), ok(file, 7, 8), ok(file, 11, 12), ok(file, 15, 16)],
    status: "completed",
    currentItemIndex: 1,
  });
  const workerB = job({
    items: [{ ...it, status: "processing", segmentIndex: 6 }],
    results: [ok(file, 1, 2), ok(file, 3, 4), ok(file, 5, 6), ok(file, 7, 8)],
    status: "processing",
    currentItemIndex: 0,
  });

  const merged = mergeOcrBatchJobs(workerA, workerB);
  assert.equal(merged.results.length, 6);
  assert.equal(allItemSegmentsCovered(merged.items[0]!, merged.results), false);
  assert.equal(merged.status, "processing");
  assert.equal(firstUnfinishedSegmentIndex(merged.items[0]!, merged.results), 4);

  const reopened = reopenIncompleteOcrBatchJob(workerA);
  assert.ok(reopened);
  assert.equal(reopened.status, "processing");
  assert.equal(isOcrBatchJobFullyCovered(reopened), false);
  assert.equal(firstUnfinishedSegmentIndex(reopened.items[0]!, reopened.results), 2);
});

test("un lot n'est complet que si chaque segment a un résultat", () => {
  const file = "SCAN.pdf";
  const it = item(file, [
    [1, 2],
    [3, 4],
  ]);
  const incomplete = job({
    items: [{ ...it, status: "done" }],
    results: [ok(file, 1, 2)],
    status: "completed",
  });
  assert.equal(isOcrBatchJobFullyCovered(incomplete), false);
  assert.equal(reopenIncompleteOcrBatchJob(incomplete)?.status, "processing");

  const complete = job({
    items: [{ ...it, status: "done" }],
    results: [ok(file, 1, 2), ok(file, 3, 4)],
    status: "completed",
  });
  assert.equal(isOcrBatchJobFullyCovered(complete), true);
  assert.equal(reopenIncompleteOcrBatchJob(complete), null);
});
