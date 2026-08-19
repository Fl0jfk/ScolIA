/** Pages Mistral (0-index du PDF envoyé) → numéros 1-index du PDF original. */
export function remapOcrPagesToAbsolute(
  pages: Array<{ index: number; markdown?: string }>,
  pageStart: number,
): Record<string, string> {
  const pageTexts: Record<string, string> = {};
  for (const page of pages) {
    const pageNum = pageStart + page.index;
    const text = (page.markdown ?? "").trim();
    if (text) pageTexts[String(pageNum)] = text;
  }
  return pageTexts;
}

export function buildOcrResultFromPageTexts(
  pageTexts: Record<string, string>,
  pageCount: number,
): { text: string; pageTexts: Record<string, string>; pageCount: number } {
  const sortedNums = Object.keys(pageTexts)
    .map(Number)
    .filter((n) => Number.isFinite(n) && n > 0)
    .sort((a, b) => a - b);
  const text = sortedNums.map((p) => `--- Page ${p} ---\n${pageTexts[String(p)]}`).join("\n\n");
  return { text, pageTexts, pageCount: pageCount > 0 ? pageCount : sortedNums.length };
}

export function mergeOcrPageTexts(
  existing: Record<string, string>,
  extra: Record<string, string>,
  pageCount: number,
): { text: string; pageTexts: Record<string, string>; pageCount: number } {
  return buildOcrResultFromPageTexts({ ...existing, ...extra }, pageCount);
}

/** Cible souple : une dizaine de pages, jamais une coupe fixe au milieu d'un bulletin. */
export const OCR_CHUNK_TARGET_PAGES = 10;
/** Plafond de sécurité si on n'arrive pas à trouver une fin de document. */
export const OCR_CHUNK_MAX_PAGES = 14;

const INE_RE = /\b[0-9OIl]{9,11}[A-Z]\b/i;
const PAGE_FRACTION_RE = /(?:page\s*)?(\d{1,2})\s*(?:\/|sur)\s*(\d{1,2})\b/i;
const DOC_START_HINT_RE =
  /\b(bulletin|relev[eé] de notes|convocation\s+n|livret scolaire|certificat de scolarit)/i;

export function extractPageIne(text: string): string | null {
  const m = INE_RE.exec(text || "");
  return m ? m[0].toUpperCase().replace(/O/g, "0").replace(/I/g, "1").replace(/L/g, "1") : null;
}

export function extractPageFraction(text: string): { current: number; total: number } | null {
  const m = PAGE_FRACTION_RE.exec(text || "");
  if (!m) return null;
  const current = Number(m[1]);
  const total = Number(m[2]);
  if (!Number.isFinite(current) || !Number.isFinite(total) || total < 1) return null;
  return { current, total };
}

/** La page ressemble à une fin de document (2/2, 3/3…). */
export function pageClearlyEndsDocument(text: string): boolean {
  const frac = extractPageFraction(text);
  return Boolean(frac && frac.current >= frac.total && frac.total >= 2);
}

/** La page ressemble à une suite (1/2, 2/3, en-tête de bulletin sans page finale). */
export function lastPageLooksUnfinished(text: string): boolean {
  const frac = extractPageFraction(text);
  if (frac && frac.current < frac.total) return true;
  if (frac && frac.current >= frac.total) return false;
  return DOC_START_HINT_RE.test(text || "");
}

/** La page suivante commence un autre document (nouvel INE, nouveau 1/n). */
export function looksLikeNewDocumentStart(prevText: string, nextText: string): boolean {
  const ineA = extractPageIne(prevText);
  const ineB = extractPageIne(nextText);
  if (ineA && ineB && ineA !== ineB) return true;
  const fracA = extractPageFraction(prevText);
  const fracB = extractPageFraction(nextText);
  if (fracB?.current === 1 && fracA && fracA.current >= fracA.total) return true;
  if (fracB?.current === 1 && fracA && fracA.current > 1) return true;
  if (DOC_START_HINT_RE.test(nextText || "") && pageClearlyEndsDocument(prevText)) return true;
  return false;
}
