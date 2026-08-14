import "server-only";

/**
 * Détection précise des zones de signature dans un PDF (devis, etc.)
 * via rasterisation page + Mistral Vision (pixtral-large-latest).
 *
 * pdfjs / @napi-rs/canvas sont chargés en lazy (dynamic import) pour ne pas
 * casser `next build` (collecte des routes API).
 */

import { createRequire } from "node:module";
import path from "node:path";

const MISTRAL_CHAT_ENDPOINT = "https://api.mistral.ai/v1/chat/completions";
const MISTRAL_VISION_MODEL = "pixtral-large-latest";

/** Coordonnées normalisées (0–1) d'une zone de signature. */
export type SignatureBBox = {
  /** Numéro de page 1-indexé. */
  pageNumber: number;
  left: number;
  top: number;
  width: number;
  height: number;
  /** Libellé Vision (ex. "Bon pour accord client"). */
  label?: string;
  /** Score interne 0–100 après filtres. */
  score?: number;
};

export type DetectSignatureZonesOptions = {
  /**
   * Mode devis : prompt + filtres orientés « signature client / direction / bon pour accord ».
   * Ignore les zones transporteur / déjà remplies quand c'est possible.
   */
  mode?: "devis" | "generic";
  /** Nombre max de pages à analyser (défaut 12). */
  maxPages?: number;
  /** Échelle de rendu PNG (défaut 1.5 — bon compromis précision / poids). */
  renderScale?: number;
};

type VisionBBoxRaw = {
  left: number;
  top: number;
  width: number;
  height: number;
  label?: string;
  role?: string;
  confidence?: string;
  already_signed?: boolean;
};

// ---------------------------------------------------------------------------
// Rasterisation PDF → PNG data URL (lazy)
// ---------------------------------------------------------------------------

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
  return path.join(path.dirname(require.resolve("pdfjs-dist/package.json")), "standard_fonts") + path.sep;
}

async function loadPdf(pdfBytes: Uint8Array | Buffer) {
  const { getDocument } = await loadPdfjs();
  const data = pdfBytes instanceof Uint8Array ? pdfBytes : new Uint8Array(pdfBytes);
  const loadingTask = getDocument({
    data,
    useWorkerFetch: false,
    useSystemFonts: true,
    standardFontDataUrl: pdfjsStandardFontsUrl(),
  });
  return loadingTask.promise;
}

type PdfDocument = Awaited<ReturnType<typeof loadPdf>>;
type PdfPage = Awaited<ReturnType<PdfDocument["getPage"]>>;

async function renderPageToPngDataUrl(
  page: Awaited<ReturnType<Awaited<ReturnType<typeof loadPdf>>["getPage"]>>,
  scale: number,
): Promise<string> {
  const { createCanvas } = await loadCanvas();
  const viewport = page.getViewport({ scale });
  const width = Math.ceil(viewport.width);
  const height = Math.ceil(viewport.height);
  const canvas = createCanvas(width, height);
  const context = canvas.getContext("2d");

  await page.render({
    canvasContext: context as unknown as CanvasRenderingContext2D,
    viewport,
    canvas: canvas as unknown as HTMLCanvasElement,
  }).promise;

  return canvas.toDataURL("image/png");
}

// ---------------------------------------------------------------------------
// Prompt Vision — devis (précision)
// ---------------------------------------------------------------------------

const DEVIS_VISION_SYSTEM = `Tu analyses l'image d'une page de DEVIS de transport scolaire (autocar / bus / voyage).
Ta mission : localiser TOUTES les zones où la DIRECTION / le CLIENT (l'école) doit apposer sa signature ou son cachet.

Zones À INCLURE (client / école / direction) :
- "Bon pour accord", "Bon pour commande", "Lu et approuvé"
- "Signature du client", "Signature et cachet", "Cachet de l'établissement"
- "Visa direction", "Signature du chef d'établissement", "Pour acceptation"
- Case / ligne vide prévue pour la signature du donneur d'ordre

Zones À EXCLURE :
- Signature / cachet déjà présents du TRANSPORTEUR / de la société émettrice
- En-têtes, logos, QR codes, tableaux de prix
- Mentions légales en bas de page sans case signature
- Zones déjà remplies d'une signature manuscrite ou tampon

Réponds UNIQUEMENT avec un objet JSON :
{
  "zones": [
    {
      "left": 0.0 à 1.0,
      "top": 0.0 à 1.0,
      "width": 0.0 à 1.0,
      "height": 0.0 à 1.0,
      "label": "libellé court",
      "role": "client_direction" | "transporteur" | "other",
      "confidence": "high" | "medium" | "low",
      "already_signed": false
    }
  ]
}

Coordonnées normalisées : left/top = coin haut-gauche (0,0 = haut-gauche de l'image).
Inclus TOUTES les zones client/direction de la page (plusieurs devis = plusieurs cases).
Si aucune zone client crédible : { "zones": [] }.`;

const GENERIC_VISION_SYSTEM = `Tu analyses l'image d'une page de document administratif.
Identifie TOUTES les zones réservées à une signature / cachet / visa encore vides.
Réponds UNIQUEMENT avec un objet JSON { "zones": [ { left, top, width, height, label, role, confidence, already_signed } ] }.
Coordonnées normalisées 0–1 (origine haut-gauche). Si aucune zone : { "zones": [] }.`;

async function detectZonesOnPageImage(
  pageImageDataUrl: string,
  pageMarkdownHint: string,
  apiKey: string,
  mode: "devis" | "generic",
): Promise<VisionBBoxRaw[]> {
  const system = mode === "devis" ? DEVIS_VISION_SYSTEM : GENERIC_VISION_SYSTEM;

  const res = await fetch(MISTRAL_CHAT_ENDPOINT, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: MISTRAL_VISION_MODEL,
      messages: [
        { role: "system", content: system },
        {
          role: "user",
          content: [
            { type: "image_url", image_url: pageImageDataUrl },
            {
              type: "text",
              text:
                (pageMarkdownHint
                  ? `Extrait texte de la page (contexte OCR éventuel) :\n${pageMarkdownHint.slice(0, 1200)}\n\n`
                  : "") +
                "Retourne l'objet JSON des zones de signature client/direction.",
            },
          ],
        },
      ],
      temperature: 0.0,
      response_format: { type: "json_object" },
    }),
  });

  if (!res.ok) {
    const body = await res.text().catch(() => "");
    console.error(`[pdf-signature-vision] Vision HTTP ${res.status}: ${body.slice(0, 250)}`);
    return [];
  }

  const data = (await res.json()) as {
    choices?: { message?: { content?: string } }[];
  };
  const raw = data.choices?.[0]?.message?.content?.trim() || "";

  try {
    const parsed = JSON.parse(raw) as unknown;
    if (Array.isArray(parsed)) return parsed as VisionBBoxRaw[];
    if (parsed && typeof parsed === "object") {
      const obj = parsed as Record<string, unknown>;
      if (Array.isArray(obj.zones)) return obj.zones as VisionBBoxRaw[];
      for (const val of Object.values(obj)) {
        if (Array.isArray(val)) return val as VisionBBoxRaw[];
      }
    }
  } catch {
    console.warn("[pdf-signature-vision] JSON Vision invalide:", raw.slice(0, 200));
  }
  return [];
}

// ---------------------------------------------------------------------------
// Filtres / scoring / dédup
// ---------------------------------------------------------------------------

function normalizeLabel(s: string): string {
  return String(s || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .trim();
}

function scoreDevisZone(z: VisionBBoxRaw): number {
  let score = 0;
  const label = normalizeLabel(z.label || "");
  const role = normalizeLabel(z.role || "");
  const conf = normalizeLabel(z.confidence || "");

  if (z.already_signed) return -100;
  if (role.includes("transporteur")) return -80;
  if (role === "other" && !label) return -20;

  if (role.includes("client") || role.includes("direction")) score += 40;

  if (/bon\s+pour/.test(label)) score += 30;
  if (/signature/.test(label)) score += 20;
  if (/cachet/.test(label)) score += 12;
  if (/visa/.test(label)) score += 10;
  if (/accord|commande|acceptation|approuv/.test(label)) score += 15;
  if (/client|etablissement|direction|ecole|college|lycee/.test(label)) score += 18;
  if (/transporteur|emetteur|societe|compagnie/.test(label) && !/client/.test(label)) score -= 50;

  if (conf === "high") score += 20;
  else if (conf === "medium") score += 8;
  else if (conf === "low") score -= 15;

  // Géométrie plausible pour une case signature
  const left = Number(z.left) || 0;
  const top = Number(z.top) || 0;
  const width = Number(z.width) || 0;
  const height = Number(z.height) || 0;

  if (width < 0.08 || height < 0.02) score -= 40;
  if (width > 0.7 || height > 0.35) score -= 25;
  if (top < 0.35) score -= 10; // signatures devis rarement en haut de page
  if (top >= 0.55) score += 10;
  if (left + width > 1.05 || top + height > 1.05) score -= 30;

  return score;
}

function sanitizeBBox(z: VisionBBoxRaw, pageNumber: number, score: number): SignatureBBox | null {
  const left = Math.max(0, Math.min(0.95, Number(z.left) || 0));
  const top = Math.max(0, Math.min(0.95, Number(z.top) || 0));
  const width = Math.max(0.05, Math.min(0.9, Number(z.width) || 0.3));
  const height = Math.max(0.03, Math.min(0.25, Number(z.height) || 0.08));

  // Re-clamp pour rester dans la page
  const w = Math.min(width, 1 - left);
  const h = Math.min(height, 1 - top);
  if (w < 0.05 || h < 0.025) return null;

  return {
    pageNumber,
    left,
    top,
    width: w,
    height: h,
    label: z.label ? String(z.label).slice(0, 120) : undefined,
    score,
  };
}

function iou(a: SignatureBBox, b: SignatureBBox): number {
  if (a.pageNumber !== b.pageNumber) return 0;
  const ax2 = a.left + a.width;
  const ay2 = a.top + a.height;
  const bx2 = b.left + b.width;
  const by2 = b.top + b.height;
  const ix1 = Math.max(a.left, b.left);
  const iy1 = Math.max(a.top, b.top);
  const ix2 = Math.min(ax2, bx2);
  const iy2 = Math.min(ay2, by2);
  const iw = Math.max(0, ix2 - ix1);
  const ih = Math.max(0, iy2 - iy1);
  const inter = iw * ih;
  if (inter <= 0) return 0;
  const union = a.width * a.height + b.width * b.height - inter;
  return union > 0 ? inter / union : 0;
}

function dedupeZones(zones: SignatureBBox[], iouThreshold = 0.45): SignatureBBox[] {
  const sorted = [...zones].sort((a, b) => (b.score ?? 0) - (a.score ?? 0));
  const kept: SignatureBBox[] = [];
  for (const z of sorted) {
    if (kept.some((k) => iou(k, z) >= iouThreshold)) continue;
    kept.push(z);
  }
  // Ordre de lecture : page puis position
  return kept.sort((a, b) => a.pageNumber - b.pageNumber || a.top - b.top || a.left - b.left);
}

function filterAndScoreZones(
  raw: VisionBBoxRaw[],
  pageNumber: number,
  mode: "devis" | "generic",
): SignatureBBox[] {
  const out: SignatureBBox[] = [];
  const minScore = mode === "devis" ? 25 : 10;

  for (const z of raw) {
    const score = mode === "devis" ? scoreDevisZone(z) : scoreDevisZone(z) + 10;
    if (score < minScore) {
      console.log(
        `[pdf-signature-vision] Zone rejetée p${pageNumber} score=${score} label="${z.label || ""}" role="${z.role || ""}"`,
      );
      continue;
    }
    const sanitized = sanitizeBBox(z, pageNumber, score);
    if (sanitized) out.push(sanitized);
  }
  return out;
}

// ---------------------------------------------------------------------------
// API publique
// ---------------------------------------------------------------------------

/**
 * Détecte toutes les zones de signature crédibles sur un PDF.
 * Pour les devis : mode "devis" (défaut recommandé) → multi-page / multi-cases.
 */
export async function detectAllSignatureZones(
  pdfBytes: Uint8Array | Buffer,
  apiKey: string,
  options: DetectSignatureZonesOptions = {},
): Promise<SignatureBBox[]> {
  const mode = options.mode ?? "devis";
  const maxPages = options.maxPages ?? 12;
  const renderScale = options.renderScale ?? 1.5;

  let pdf: PdfDocument;
  try {
    pdf = await loadPdf(pdfBytes);
  } catch (err) {
    console.error("[pdf-signature-vision] Impossible de charger le PDF:", err);
    return [];
  }

  const pageCount = Math.min(pdf.numPages, maxPages);
  const allZones: SignatureBBox[] = [];

  try {
    for (let pageIndex = 1; pageIndex <= pageCount; pageIndex++) {
      let page: PdfPage;
      try {
        page = await pdf.getPage(pageIndex);
      } catch (err) {
        console.warn(`[pdf-signature-vision] Page ${pageIndex} illisible:`, err);
        continue;
      }

      let dataUrl: string;
      try {
        dataUrl = await renderPageToPngDataUrl(page, renderScale);
      } catch (err) {
        console.error(`[pdf-signature-vision] Rendu page ${pageIndex} échoué:`, err);
        continue;
      }

      // Petit extrait texte natif PDF (sans OCR) pour contextualiser Vision
      let textHint = "";
      try {
        const textContent = await page.getTextContent();
        textHint = textContent.items
          .map((it: unknown) => (it && typeof it === "object" && "str" in it ? String((it as { str: string }).str) : ""))
          .join(" ")
          .slice(0, 1200);
      } catch {
        /* ignore */
      }

      const rawZones = await detectZonesOnPageImage(dataUrl, textHint, apiKey, mode);
      const filtered = filterAndScoreZones(rawZones, pageIndex, mode);
      allZones.push(...filtered);

      console.log(
        `[pdf-signature-vision] Page ${pageIndex}/${pageCount}: ${rawZones.length} brutes → ${filtered.length} retenues`,
      );
    }
  } finally {
    // Libération mémoire si l'API le permet (selon version pdfjs)
    try {
      const maybeDestroy = (pdf as PdfDocument & { destroy?: () => Promise<void> }).destroy;
      if (typeof maybeDestroy === "function") await maybeDestroy.call(pdf);
    } catch {
      /* ignore */
    }
  }

  const deduped = dedupeZones(allZones);
  console.log(`[pdf-signature-vision] Total zones retenues: ${deduped.length}`);
  return deduped;
}

/**
 * Convertit une BBox normalisée (origine haut-gauche) en coordonnées pdf-lib
 * (origine bas-gauche). Place la signature dans / juste sous la zone.
 */
export function signatureBBoxToPdfLibCoords(
  pageWidth: number,
  pageHeight: number,
  bbox: SignatureBBox,
  sigDrawWidth: number,
  sigDrawHeight: number,
  gapBelowLabelPt = 2,
): { x: number; y: number } {
  // Centrer horizontalement dans la zone si possible
  const zoneLeftPt = pageWidth * bbox.left;
  const zoneWidthPt = pageWidth * bbox.width;
  let x = zoneLeftPt + Math.max(0, (zoneWidthPt - sigDrawWidth) / 2);

  // Placer verticalement dans la partie basse de la zone détectée
  const zoneBottomPdfY = pageHeight * (1 - bbox.top - bbox.height);
  const zoneTopPdfY = pageHeight * (1 - bbox.top);
  const zoneHeightPt = zoneTopPdfY - zoneBottomPdfY;

  let y: number;
  if (zoneHeightPt >= sigDrawHeight + gapBelowLabelPt) {
    // Assez de place dans la zone : coller en bas de la zone
    y = zoneBottomPdfY + gapBelowLabelPt;
  } else {
    // Zone trop petite (souvent juste le libellé) : juste en dessous
    y = zoneBottomPdfY - gapBelowLabelPt - sigDrawHeight;
  }

  const margin = 8;
  x = Math.max(margin, Math.min(x, pageWidth - sigDrawWidth - margin));
  y = Math.max(margin, Math.min(y, pageHeight - sigDrawHeight - margin));
  return { x, y };
}
