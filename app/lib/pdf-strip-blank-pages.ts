import "server-only";

/**
 * Retire les pages vraiment blanches d’un PDF scanné (recto-verso photocopieuse).
 * Rasterisation pdfjs + @napi-rs/canvas, seuils prudents (voir pdf-blank-page-ink).
 */

import { createRequire } from "node:module";
import path from "node:path";
import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { PDFDocument } from "pdf-lib";
import {
  computeInkStatsFromRgba,
  isBlankPageFromInkStats,
} from "@/app/lib/pdf-blank-page-ink";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

type PdfJsModule = typeof import("pdfjs-dist/legacy/build/pdf.mjs");
type CanvasModule = typeof import("@napi-rs/canvas");

let _pdfjs: PdfJsModule | null = null;
let _canvas: CanvasModule | null = null;

async function loadPdfjs(): Promise<PdfJsModule> {
  if (!_pdfjs) {
    _pdfjs = await import("pdfjs-dist/legacy/build/pdf.mjs");
  }
  return _pdfjs;
}

async function loadCanvas(): Promise<CanvasModule> {
  if (!_canvas) {
    _canvas = await import("@napi-rs/canvas");
  }
  return _canvas;
}

function pdfjsStandardFontsUrl(): string {
  const require = createRequire(__filename);
  const root = path.dirname(require.resolve("pdfjs-dist/package.json"));
  return `${path.join(root, "standard_fonts").replace(/\\/g, "/")}/`;
}

/** Échelle basse : assez pour détecter encre / tampon, rapide sur scans multi-pages. */
const RENDER_SCALE = 0.65;

export type StripBlankPagesResult = {
  bytes: Uint8Array;
  pageCountBefore: number;
  pageCountAfter: number;
  /** Numéros de pages retirées (1-indexés, source). */
  removedPageNumbers: number[];
  changed: boolean;
};

async function loadPdfDocument(pdfBytes: Uint8Array) {
  const { getDocument } = await loadPdfjs();
  // Copie obligatoire : pdfjs peut détacher le ArrayBuffer, et pdf-lib renvoie parfois une vue.
  const data = new Uint8Array(pdfBytes);
  const loadingTask = getDocument({
    data,
    useWorkerFetch: false,
    useSystemFonts: true,
    standardFontDataUrl: pdfjsStandardFontsUrl(),
  });
  return loadingTask.promise;
}

async function pageLooksBlank(
  page: Awaited<ReturnType<Awaited<ReturnType<typeof loadPdfDocument>>["getPage"]>>,
): Promise<boolean> {
  const { createCanvas } = await loadCanvas();
  const viewport = page.getViewport({ scale: RENDER_SCALE });
  const width = Math.max(1, Math.ceil(viewport.width));
  const height = Math.max(1, Math.ceil(viewport.height));
  const canvas = createCanvas(width, height);
  const context = canvas.getContext("2d");

  // Fond blanc explicite (pages PDF sans contenu → blanc, pas transparent).
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, width, height);

  await page.render({
    canvasContext: context as unknown as CanvasRenderingContext2D,
    viewport,
    canvas: canvas as unknown as HTMLCanvasElement,
  }).promise;

  const imageData = context.getImageData(0, 0, width, height);
  const stats = computeInkStatsFromRgba(imageData.data, width, height);
  return isBlankPageFromInkStats(stats);
}

/**
 * Analyse le PDF et renvoie une copie sans les pages jugées blanches.
 * En cas d’échec ou si tout serait retiré : bytes d’origine, `changed: false`.
 */
export async function stripBlankPagesFromPdfBytes(
  pdfBytes: Uint8Array | Buffer,
): Promise<StripBlankPagesResult> {
  const input = new Uint8Array(pdfBytes);
  const unchanged = (pageCount: number): StripBlankPagesResult => ({
    bytes: input,
    pageCountBefore: pageCount,
    pageCountAfter: pageCount,
    removedPageNumbers: [],
    changed: false,
  });

  let pageCount = 0;
  try {
    const pdf = await loadPdfDocument(input);
    pageCount = pdf.numPages;
    if (pageCount <= 1) return unchanged(pageCount);

    const keepIndices: number[] = [];
    const removedPageNumbers: number[] = [];

    for (let pageNumber = 1; pageNumber <= pageCount; pageNumber++) {
      const page = await pdf.getPage(pageNumber);
      let blank = false;
      try {
        blank = await pageLooksBlank(page);
      } catch (err) {
        // En doute (rendu impossible) → on garde la page.
        console.warn("[pdf-strip-blank-pages] render page failed — keeping", {
          pageNumber,
          err: err instanceof Error ? err.message : String(err),
        });
        blank = false;
      }
      if (blank) {
        removedPageNumbers.push(pageNumber);
      } else {
        keepIndices.push(pageNumber - 1);
      }
    }

    // Ne jamais tout supprimer : si toutes blanches, on laisse le PDF intact.
    if (keepIndices.length === 0 || removedPageNumbers.length === 0) {
      return unchanged(pageCount);
    }

    const src = await PDFDocument.load(input, { ignoreEncryption: true });
    const out = await PDFDocument.create();
    const copied = await out.copyPages(src, keepIndices);
    for (const p of copied) out.addPage(p);
    const bytes = await out.save();

    return {
      bytes: new Uint8Array(bytes),
      pageCountBefore: pageCount,
      pageCountAfter: keepIndices.length,
      removedPageNumbers,
      changed: true,
    };
  } catch (err) {
    console.warn("[pdf-strip-blank-pages] strip failed — keeping original", {
      err: err instanceof Error ? err.message : String(err),
    });
    return unchanged(pageCount);
  }
}

/**
 * Télécharge un PDF S3, retire les pages blanches, réécrit la même clé si besoin.
 * Échecs silencieux côté appelant (log interne) — ne bloque pas l’upload métier.
 */
export async function stripBlankPagesInS3Object(
  storedKey: string,
): Promise<StripBlankPagesResult | null> {
  const key = s3Key(storedKey);
  const bucket = await getBucketName();
  const s3 = await getTenantDataS3Client();

  const obj = await s3.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
  const raw = await obj.Body?.transformToByteArray();
  if (!raw?.length) return null;

  const result = await stripBlankPagesFromPdfBytes(raw);
  if (!result.changed) return result;

  await s3.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: key,
      Body: result.bytes,
      ContentType: "application/pdf",
    }),
  );

  console.info("[pdf-strip-blank-pages] pages blanches retirées", {
    key,
    before: result.pageCountBefore,
    after: result.pageCountAfter,
    removed: result.removedPageNumbers,
  });

  return result;
}

export function looksLikePdfUpload(opts: {
  mimeType?: string | null;
  s3Key?: string | null;
  fileName?: string | null;
}): boolean {
  const mime = String(opts.mimeType || "").toLowerCase();
  if (mime.includes("pdf")) return true;
  const name = String(opts.fileName || opts.s3Key || "").toLowerCase();
  return name.endsWith(".pdf");
}
