import "server-only";

/**
 * Façade OCR — Mistral OCR (mistral-ocr-latest).
 *
 * Les PDF longs (classe de 90 bulletins) ne partent JAMAIS en un seul appel :
 * on découpe par paquets de pages, avec retry 503/429 et repli page par page
 * si un paquet est trop gros.
 */

import { GetObjectCommand } from "@aws-sdk/client-s3";
import { PDFDocument } from "pdf-lib";
import { extractPdfPagesBytes } from "@/app/lib/ocr-extract-pages";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { ocrTraceCtx, type OcrTraceCtx } from "@/app/lib/ocr-trace";
import {
  buildOcrResultFromPageTexts,
  mergeOcrPageTexts,
  remapOcrPagesToAbsolute,
  OCR_CHUNK_MAX_PAGES,
  OCR_CHUNK_TARGET_PAGES,
  lastPageLooksUnfinished,
  looksLikeNewDocumentStart,
  pageClearlyEndsDocument,
} from "@/app/lib/ocr-textract-pages";

export type OcrTextractResult = {
  text: string;
  pageTexts: Record<string, string>;
  pageCount: number;
};

export {
  mergeOcrPageTexts,
  remapOcrPagesToAbsolute,
  buildOcrResultFromPageTexts,
  OCR_CHUNK_MAX_PAGES,
  OCR_CHUNK_TARGET_PAGES,
  lastPageLooksUnfinished,
  looksLikeNewDocumentStart,
  pageClearlyEndsDocument,
};

type TextractPollResult =
  | { status: "IN_PROGRESS"; pagesRead: number; maxPageSeen: number }
  | { status: "FAILED" }
  | { status: "SUCCEEDED"; result: OcrTextractResult; pagesRead: number };

const _jobCache = new Map<
  string,
  { result: OcrTextractResult } | { error: string } | { inProgress: { pagesRead: number; pageCount: number } }
>();

function newJobId(): string {
  return `mistral-ocr-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;
}

const MISTRAL_OCR_ENDPOINT = "https://api.mistral.ai/v1/ocr";
const MISTRAL_OCR_MODEL = "mistral-ocr-latest";
/** Cible souple (~10 pages) — on allonge jusqu'à la fin du document en cours. */
export const OCR_CHUNK_PAGES = 10;
const OCR_FETCH_TIMEOUT_MS = 90_000;

type MistralOcrPage = {
  index: number;
  markdown: string;
};

type MistralOcrResponse = {
  pages: MistralOcrPage[];
  model?: string;
  usage_info?: unknown;
};

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

function httpStatusFromOcrError(msg: string): number | null {
  const m = /Mistral OCR HTTP (\d+)/.exec(msg);
  return m ? Number(m[1]) : null;
}

function isRetryableOcrError(err: Error): boolean {
  const status = httpStatusFromOcrError(err.message);
  if (status === 429 || status === 500 || status === 502 || status === 503 || status === 504) return true;
  return /timeout|aborted|network|fetch failed|econnreset/i.test(err.message);
}

function isTooLargeOcrError(err: Error): boolean {
  const status = httpStatusFromOcrError(err.message);
  if (status === 413) return true;
  return /too large|50\s*mb|payload|request entity|max.*size/i.test(err.message);
}

async function callMistralOcr(pdfBase64: string, apiKey: string): Promise<MistralOcrResponse> {
  const response = await fetch(MISTRAL_OCR_ENDPOINT, {
    method: "POST",
    signal: AbortSignal.timeout(OCR_FETCH_TIMEOUT_MS),
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: MISTRAL_OCR_MODEL,
      document: {
        type: "document_url",
        document_url: `data:application/pdf;base64,${pdfBase64}`,
      },
      include_image_base64: false,
    }),
  });

  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(`Mistral OCR HTTP ${response.status}: ${body.slice(0, 300)}`);
  }

  return response.json() as Promise<MistralOcrResponse>;
}

async function callMistralOcrWithRetry(pdfBase64: string, apiKey: string): Promise<MistralOcrResponse> {
  let lastErr: Error | null = null;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      return await callMistralOcr(pdfBase64, apiKey);
    } catch (err) {
      lastErr = err instanceof Error ? err : new Error(String(err));
      if (!isRetryableOcrError(lastErr) || attempt === 3) throw lastErr;
      await sleep(1000 * 2 ** (attempt - 1));
    }
  }
  throw lastErr!;
}

async function ocrSinglePdfBytes(
  pdfBytes: Uint8Array,
  apiKey: string,
  pageStart: number,
): Promise<Record<string, string>> {
  const base64 = Buffer.from(pdfBytes).toString("base64");
  const ocr = await callMistralOcrWithRetry(base64, apiKey);
  return remapOcrPagesToAbsolute(ocr.pages ?? [], pageStart);
}

async function extractRangeFromLoadedDoc(
  src: PDFDocument,
  start0: number,
  end0Inclusive: number,
): Promise<Uint8Array> {
  const out = await PDFDocument.create();
  const indices: number[] = [];
  for (let i = start0; i <= end0Inclusive; i++) indices.push(i);
  const copied = await out.copyPages(src, indices);
  copied.forEach((p) => out.addPage(p));
  return out.save();
}

/**
 * OCR d'une plage de pages (1-indexée) d'un PDF S3.
 * Si le paquet est trop gros, repli automatique en pages unitaires.
 */
export async function ocrPdfPageRangeFromS3(
  key: string,
  pageStart: number,
  pageEnd: number,
  trace?: OcrTraceCtx,
): Promise<Record<string, string>> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) throw new Error("MISTRAL_API_KEY manquant (OCR)");

  const start = Math.max(1, pageStart);
  const end = Math.max(start, pageEnd);
  ocrTraceCtx(trace, "textract", "chunk-start", "OCR paquet de pages", { s3Key: key, pages: `${start}-${end}` });

  try {
    const bytes = await extractPdfPagesBytes(key, start, end);
    const pageTexts = await ocrSinglePdfBytes(bytes, apiKey, start);
    ocrTraceCtx(trace, "textract", "chunk-done", "OCR paquet terminé", {
      pages: `${start}-${end}`,
      pagesWithText: Object.keys(pageTexts).length,
    });
    return pageTexts;
  } catch (err) {
    const error = err instanceof Error ? err : new Error(String(err));
    if (start === end || (!isTooLargeOcrError(error) && !isRetryableOcrError(error))) {
      throw error;
    }
    ocrTraceCtx(trace, "textract", "chunk-split", "paquet trop gros / instable — page par page", {
      pages: `${start}-${end}`,
      error: error.message.slice(0, 200),
    }, "warn");
    const pageTexts: Record<string, string> = {};
    for (let p = start; p <= end; p++) {
      const bytes = await extractPdfPagesBytes(key, p, p);
      Object.assign(pageTexts, await ocrSinglePdfBytes(bytes, apiKey, p));
    }
    return pageTexts;
  }
}

async function ocrPdfBytesChunked(
  pdfBytes: Uint8Array,
  apiKey: string,
  trace?: OcrTraceCtx,
  onProgress?: (pagesRead: number, pageCount: number) => void,
): Promise<OcrTextractResult> {
  const src = await PDFDocument.load(pdfBytes, { ignoreEncryption: true });
  const total = src.getPageCount();
  if (total <= 0) throw new Error("PDF sans pages");

  const pageTexts: Record<string, string> = {};
  for (let start0 = 0; start0 < total; ) {
    let size = Math.min(OCR_CHUNK_PAGES, total - start0);
    let advanced = false;
    while (size >= 1 && !advanced) {
      const end0 = start0 + size - 1;
      try {
        const slice = await extractRangeFromLoadedDoc(src, start0, end0);
        Object.assign(pageTexts, await ocrSinglePdfBytes(slice, apiKey, start0 + 1));
        start0 = end0 + 1;
        advanced = true;
        onProgress?.(start0, total);
        ocrTraceCtx(trace, "textract", "chunk-done", "OCR paquet terminé", {
          pagesRead: start0,
          pageCount: total,
        });
      } catch (err) {
        const error = err instanceof Error ? err : new Error(String(err));
        if (size > 1 && (isTooLargeOcrError(error) || isRetryableOcrError(error))) {
          size = Math.max(1, Math.floor(size / 2));
          continue;
        }
        throw error;
      }
    }
    if (!advanced) throw new Error("OCR : impossible d'avancer sur ce PDF");
  }

  return buildOcrResultFromPageTexts(pageTexts, total);
}

async function fetchPdfBytesFromS3(key: string): Promise<Uint8Array> {
  const bucket = await getBucketName();
  const client = await getTenantDataS3Client();
  const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
  const bytes = await res.Body?.transformToByteArray();
  if (!bytes?.length) throw new Error(`PDF vide ou introuvable sur S3 (clé : ${key})`);
  return bytes;
}

export async function startTextractForS3Key(key: string, trace?: OcrTraceCtx): Promise<string> {
  const jobId = newJobId();
  _jobCache.set(jobId, { inProgress: { pagesRead: 0, pageCount: 0 } });
  ocrTraceCtx(trace, "textract", "mistral-start", "Mistral OCR démarré (par paquets)", { s3Key: key, jobId });

  (async () => {
    try {
      const apiKey = await getMistralApiKey();
      if (!apiKey) throw new Error("MISTRAL_API_KEY manquant (OCR)");

      const pdfBytes = await fetchPdfBytesFromS3(key);
      const result = await ocrPdfBytesChunked(pdfBytes, apiKey, trace, (pagesRead, pageCount) => {
        _jobCache.set(jobId, { inProgress: { pagesRead, pageCount } });
      });

      ocrTraceCtx(trace, "textract", "mistral-done", "Mistral OCR terminé", {
        jobId,
        s3Key: key,
        pageCount: result.pageCount,
        textChars: result.text.length,
      });
      _jobCache.set(jobId, { result });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      ocrTraceCtx(trace, "textract", "mistral-error", "Mistral OCR erreur", { jobId, s3Key: key, error: msg }, "error");
      _jobCache.set(jobId, { error: msg });
    }
  })();

  return jobId;
}

export async function pollTextractOnce(
  jobId: string,
  trace?: OcrTraceCtx,
): Promise<TextractPollResult> {
  const cached = _jobCache.get(jobId);

  if (!cached) {
    ocrTraceCtx(trace, "textract", "mistral-poll", "Mistral OCR IN_PROGRESS (pas encore en cache)", { jobId });
    return { status: "IN_PROGRESS", pagesRead: 0, maxPageSeen: 0 };
  }

  if ("error" in cached) {
    ocrTraceCtx(trace, "textract", "mistral-failed", "Mistral OCR FAILED", { jobId, error: cached.error }, "error");
    return { status: "FAILED" };
  }

  if ("inProgress" in cached) {
    return {
      status: "IN_PROGRESS",
      pagesRead: cached.inProgress.pagesRead,
      maxPageSeen: cached.inProgress.pagesRead,
    };
  }

  const { result } = cached;
  ocrTraceCtx(trace, "textract", "mistral-succeeded", "Mistral OCR SUCCEEDED", {
    jobId,
    pageCount: result.pageCount,
    textChars: result.text.length,
  });
  return { status: "SUCCEEDED", result, pagesRead: result.pageCount };
}

export async function runTextractForS3Key(key: string, _maxAttempts = 90): Promise<OcrTextractResult> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) throw new Error("MISTRAL_API_KEY manquant (OCR)");
  const pdfBytes = await fetchPdfBytesFromS3(key);
  const result = await ocrPdfBytesChunked(pdfBytes, apiKey);
  if (!result.text.trim()) throw new Error("Mistral OCR : texte vide");
  return result;
}

export async function runTextractForPdfBytes(pdfBytes: Uint8Array | Buffer): Promise<OcrTextractResult> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) throw new Error("MISTRAL_API_KEY manquant (OCR)");
  const result = await ocrPdfBytesChunked(Buffer.from(pdfBytes), apiKey);
  if (!result.text.trim()) throw new Error("Mistral OCR : texte vide");
  return result;
}
