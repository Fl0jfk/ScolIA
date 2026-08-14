import "server-only";

/**
 * Façade OCR — remplace AWS Textract par Mistral OCR (mistral-ocr-latest).
 *
 * Les signatures publiques sont conservées à l'identique pour éviter de modifier
 * tous les appelants :
 *   - startTextractForS3Key  → lance l'OCR, cache le résultat, retourne un jobId local
 *   - pollTextractOnce       → retourne immédiatement SUCCEEDED (OCR Mistral sync)
 *   - runTextractForS3Key    → variante bloquante historique
 *
 * Mistral OCR est synchrone → pas de vrai polling. Pour rester compatible avec
 * le pipeline batch (ocr-batch-process.ts, rh-ocr-batch.ts) qui appelle start puis poll,
 * start() stocke le résultat en mémoire keyed par un jobId UUID et poll() le renvoie.
 */

import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { ocrTraceCtx, type OcrTraceCtx } from "@/app/lib/ocr-trace";

// ---------------------------------------------------------------------------
// Types publics
// ---------------------------------------------------------------------------

type OcrTextractResult = {
  text: string;
  pageTexts: Record<string, string>;
  pageCount: number;
};

type TextractPollResult =
  | { status: "IN_PROGRESS"; pagesRead: number; maxPageSeen: number }
  | { status: "FAILED" }
  | { status: "SUCCEEDED"; result: OcrTextractResult; pagesRead: number };

// ---------------------------------------------------------------------------
// Cache en mémoire (jobId → résultat) — même cycle de vie que le process Node.
// Les jobs batch sont traités dans la même invocation serverless ; ok pour Scaleway Containers.
// ---------------------------------------------------------------------------

const _jobCache = new Map<string, { result: OcrTextractResult } | { error: string }>();

function newJobId(): string {
  return `mistral-ocr-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;
}

// ---------------------------------------------------------------------------
// Mistral OCR
// ---------------------------------------------------------------------------

const MISTRAL_OCR_ENDPOINT = "https://api.mistral.ai/v1/ocr";
const MISTRAL_OCR_MODEL = "mistral-ocr-latest";

type MistralOcrPage = {
  index: number;
  markdown: string;
};

type MistralOcrResponse = {
  pages: MistralOcrPage[];
  model?: string;
  usage_info?: unknown;
};

async function callMistralOcr(
  pdfBase64: string,
  apiKey: string,
): Promise<MistralOcrResponse> {
  const response = await fetch(MISTRAL_OCR_ENDPOINT, {
    method: "POST",
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

function mistralOcrResponseToResult(ocr: MistralOcrResponse): OcrTextractResult {
  const pageTexts: Record<string, string> = {};

  const pages = (ocr.pages ?? []).sort((a, b) => a.index - b.index);

  for (const page of pages) {
    // Les pages Mistral OCR sont 0-indexed → on expose en 1-indexed (compat Textract)
    const pageNum = page.index + 1;
    const text = (page.markdown ?? "").trim();
    if (text) pageTexts[String(pageNum)] = text;
  }

  const sortedNums = Object.keys(pageTexts)
    .map(Number)
    .sort((a, b) => a - b);

  const text = sortedNums
    .map((p) => `--- Page ${p} ---\n${pageTexts[String(p)]}`)
    .join("\n\n");

  return { text, pageTexts, pageCount: sortedNums.length };
}

// ---------------------------------------------------------------------------
// Récupération du PDF depuis Scaleway S3
// ---------------------------------------------------------------------------

async function fetchPdfBytesFromS3(key: string): Promise<Uint8Array> {
  const bucket = await getBucketName();
  const client = await getTenantDataS3Client();
  const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
  const bytes = await res.Body?.transformToByteArray();
  if (!bytes?.length) throw new Error(`PDF vide ou introuvable sur S3 (clé : ${key})`);
  return bytes;
}

// ---------------------------------------------------------------------------
// API publique — mêmes signatures que l'ancienne façade Textract
// ---------------------------------------------------------------------------

/**
 * Lance l'OCR Mistral sur la clé S3 et retourne un jobId local (UUID).
 * Le résultat est mis en cache pour être récupéré via pollTextractOnce.
 */
export async function startTextractForS3Key(key: string, trace?: OcrTraceCtx): Promise<string> {
  const jobId = newJobId();
  ocrTraceCtx(trace, "textract", "mistral-start", "Mistral OCR démarré", { s3Key: key, jobId });

  // Lancement async : le caller peut enchaîner sur pollTextractOnce immédiatement.
  // On stocke le résultat (ou l'erreur) dans le cache pour la prochaine consultation.
  (async () => {
    try {
      const apiKey = await getMistralApiKey();
      if (!apiKey) throw new Error("MISTRAL_API_KEY manquant (OCR)");

      const pdfBytes = await fetchPdfBytesFromS3(key);
      const base64 = Buffer.from(pdfBytes).toString("base64");

      const ocrResponse = await callMistralOcr(base64, apiKey);
      const result = mistralOcrResponseToResult(ocrResponse);

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

/**
 * Consulte l'état du job Mistral OCR.
 * Si le résultat est déjà en cache : SUCCEEDED immédiatement.
 * Sinon : IN_PROGRESS (le caller doit rappeler après quelques secondes).
 */
export async function pollTextractOnce(
  jobId: string,
  trace?: OcrTraceCtx,
): Promise<TextractPollResult> {
  const cached = _jobCache.get(jobId);

  if (!cached) {
    // Encore en cours
    ocrTraceCtx(trace, "textract", "mistral-poll", "Mistral OCR IN_PROGRESS (pas encore en cache)", { jobId });
    return { status: "IN_PROGRESS", pagesRead: 0, maxPageSeen: 0 };
  }

  if ("error" in cached) {
    ocrTraceCtx(trace, "textract", "mistral-failed", "Mistral OCR FAILED", { jobId, error: cached.error }, "error");
    return { status: "FAILED" };
  }

  const { result } = cached;
  ocrTraceCtx(trace, "textract", "mistral-succeeded", "Mistral OCR SUCCEEDED", {
    jobId,
    pageCount: result.pageCount,
    textChars: result.text.length,
  });
  return { status: "SUCCEEDED", result, pagesRead: result.pageCount };
}

/**
 * Variante bloquante historique — conservée pour compat.
 * Effectue l'OCR Mistral en une seule fois (sans passer par le cache).
 */
export async function runTextractForS3Key(
  key: string,
  _maxAttempts = 90,
): Promise<OcrTextractResult> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) throw new Error("MISTRAL_API_KEY manquant (OCR)");

  const pdfBytes = await fetchPdfBytesFromS3(key);
  const base64 = Buffer.from(pdfBytes).toString("base64");

  const ocrResponse = await callMistralOcr(base64, apiKey);
  const result = mistralOcrResponseToResult(ocrResponse);

  if (!result.text.trim()) throw new Error("Mistral OCR : texte vide");
  return result;
}

/**
 * OCR direct sur des bytes PDF (sans passer par S3) — utile pour les callers
 * qui ont déjà le buffer en mémoire (travel-devis-ocr, personnel-document-text, etc.).
 */
export async function runTextractForPdfBytes(pdfBytes: Uint8Array | Buffer): Promise<OcrTextractResult> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) throw new Error("MISTRAL_API_KEY manquant (OCR)");

  const base64 = Buffer.from(pdfBytes).toString("base64");
  const ocrResponse = await callMistralOcr(base64, apiKey);
  const result = mistralOcrResponseToResult(ocrResponse);

  if (!result.text.trim()) throw new Error("Mistral OCR : texte vide");
  return result;
}
