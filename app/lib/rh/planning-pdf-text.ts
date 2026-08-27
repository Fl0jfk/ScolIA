import "server-only";

import path from "path";
import { createRequire } from "module";

export type PdfTextItem = {
  text: string;
  /** Centre horizontal approximatif. */
  x: number;
  /** Baseline PDF (origine bas-gauche). */
  y: number;
  width: number;
  height: number;
  pageIndex: number;
  pageWidth: number;
  pageHeight: number;
};

type PdfJsModule = typeof import("pdfjs-dist/legacy/build/pdf.mjs");

let _pdfjs: PdfJsModule | null = null;

async function loadPdfjs(): Promise<PdfJsModule> {
  if (!_pdfjs) {
    _pdfjs = await import("pdfjs-dist/legacy/build/pdf.mjs");
  }
  return _pdfjs;
}

function pdfjsStandardFontsUrl(): string {
  const require = createRequire(import.meta.url);
  const root = path.dirname(require.resolve("pdfjs-dist/package.json"));
  return `${path.join(root, "standard_fonts").replace(/\\/g, "/")}/`;
}

/**
 * Extrait le texte natif d’un PDF avec coordonnées (pas d’OCR).
 * Utile pour les grilles Pronote / EDT vectorielles.
 */
export async function extractPdfTextItems(
  pdfBytes: Buffer | Uint8Array,
): Promise<PdfTextItem[]> {
  const { getDocument } = await loadPdfjs();
  const data = pdfBytes instanceof Buffer ? new Uint8Array(pdfBytes) : pdfBytes;
  const loadingTask = getDocument({
    data,
    useWorkerFetch: false,
    useSystemFonts: true,
    standardFontDataUrl: pdfjsStandardFontsUrl(),
  });
  const pdf = await loadingTask.promise;
  const out: PdfTextItem[] = [];

  try {
    for (let pageIndex = 1; pageIndex <= pdf.numPages; pageIndex++) {
      const page = await pdf.getPage(pageIndex);
      const viewport = page.getViewport({ scale: 1 });
      const content = await page.getTextContent();
      for (const raw of content.items) {
        if (!raw || typeof raw !== "object" || !("str" in raw)) continue;
        const it = raw as {
          str: string;
          transform: number[];
          width?: number;
          height?: number;
        };
        const text = String(it.str || "").replace(/\s+/g, " ").trim();
        if (!text) continue;
        const transform = it.transform || [1, 0, 0, 1, 0, 0];
        const x = Number(transform[4]) || 0;
        const y = Number(transform[5]) || 0;
        const width = Number(it.width) || Math.max(4, text.length * 4);
        const height = Number(it.height) || Math.abs(Number(transform[3]) || 10) || 10;
        out.push({
          text,
          x: x + width / 2,
          y,
          width,
          height,
          pageIndex,
          pageWidth: viewport.width,
          pageHeight: viewport.height,
        });
      }
    }
  } finally {
    try {
      const maybeDestroy = (pdf as { destroy?: () => Promise<void> }).destroy;
      if (typeof maybeDestroy === "function") await maybeDestroy.call(pdf);
    } catch {
      /* ignore */
    }
  }

  return out;
}
