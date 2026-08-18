import { extractInesFromText, normalizeIne, scanStudentsInText } from "@/app/lib/ocr-eleve-match";

export type OcrDocumentSegment = {
  pageStart: number;
  pageEnd: number;
  type?: string;
  nom?: string;
  prenom?: string;
  ine?: string;
  /** Dossier élève déjà résolu (découpage ancré identité) — évite un re-matching IA. */
  folderName?: string;
  label?: string;
};

/** Élève connu (eleves.json), pré-normalisé pour le découpage ancré identité. */
export type KnownStudent = {
  ine: string;
  nom: string;
  prenom: string;
  folderName: string;
  normNom: string;
  normPrenom: string;
};

const HEAD_CHARS = 500;
const TAIL_CHARS = 250;

/** Résumé court par page — suffisant pour repérer les frontières de bulletins. */
export function buildPageDigestForSegmentation(
  pageTexts: Record<string, string>,
  pageCount?: number,
): { digest: string; pageCount: number } {
  const pageNumbers = Object.keys(pageTexts)
    .map(Number)
    .filter((n) => Number.isFinite(n) && n > 0)
    .sort((a, b) => a - b);

  const count =
    typeof pageCount === "number" && pageCount > 0
      ? pageCount
      : pageNumbers.length > 0
        ? pageNumbers[pageNumbers.length - 1]
        : 0;

  const pages =
    pageNumbers.length > 0
      ? pageNumbers
      : count > 0
        ? Array.from({ length: count }, (_, i) => i + 1)
        : [];

  const parts: string[] = [];
  for (const p of pages) {
    const raw = (pageTexts[String(p)] || "").replace(/\s+/g, " ").trim();
    if (!raw) {
      parts.push(`--- Page ${p} ---\n(vide)`);
      continue;
    }
    const head = raw.slice(0, HEAD_CHARS);
    const tail = raw.length > HEAD_CHARS + TAIL_CHARS ? raw.slice(-TAIL_CHARS) : "";
    parts.push(
      tail
        ? `--- Page ${p} ---\n${head}\n[…]\n${tail}`
        : `--- Page ${p} ---\n${head}`,
    );
  }

  return { digest: parts.join("\n\n"), pageCount: count || pages.length };
}

function pageFingerprint(pageText: string): string {
  const t = pageText.replace(/\s+/g, " ").trim();
  const ine = t.match(/\b(\d{10,11}[A-Z]?)\b/i)?.[1]?.toUpperCase();
  if (ine) return `ine:${ine}`;
  const head = t
    .slice(0, 700)
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
  return `h:${head.slice(0, 120)}`;
}

/* ───────────────────────── Découpage ancré sur l'identité élève ─────────────────────────
 * Le découpage le plus fiable n'essaie pas de "deviner" des frontières génériques : il
 * s'appuie sur l'INE et la liste connue des élèves (eleves.json). Une page = un propriétaire
 * (l'élève dont c'est le bulletin). Les pages consécutives d'un même élève forment UN document.
 * Une page sans identité détectée = continuation du document courant (on ne coupe JAMAIS dessus).
 */

export type PageOwner = {
  key: string;
  ine?: string;
  nom?: string;
  prenom?: string;
  folderName?: string;
  via: "ine" | "name";
};

/**
 * Identité de l'élève "propriétaire" d'une page.
 * 1) INE d'un élève connu présent dans la page → match sûr.
 * 2) Sinon nom + prénom d'un (et un seul) élève connu présents dans l'en-tête de page.
 * Renvoie null si rien d'exploitable (la page sera rattachée au document courant).
 */
export function detectPageOwner(pageText: string, students: KnownStudent[]): PageOwner | null {
  if (!pageText.trim() || students.length === 0) return null;

  const ines = extractInesFromText(pageText);
  const ineHits = students.filter((s) => s.ine && ines.includes(normalizeIne(s.ine)));
  if (ineHits.length === 1) {
    const s = ineHits[0];
    return {
      key: `ine:${s.ine}`,
      ine: s.ine,
      nom: s.nom,
      prenom: s.prenom,
      folderName: s.folderName,
      via: "ine",
    };
  }

  const nameHits = scanStudentsInText(pageText.slice(0, 1400), students);
  if (nameHits.length === 1) {
    const s = nameHits[0];
    return {
      key: s.ine ? `ine:${s.ine}` : `stu:${s.folderName}`,
      ine: s.ine || undefined,
      nom: s.nom,
      prenom: s.prenom,
      folderName: s.folderName,
      via: "name",
    };
  }
  return null;
}

export type IdentityAnchoredResult = {
  segments: OcrDocumentSegment[];
  detectedPages: number;
  distinctOwners: number;
  pageCount: number;
};

/** Découpe le PDF en groupant les pages consécutives d'un même élève (INE/nom connus). */
export function identityAnchoredSegments(
  pageTexts: Record<string, string>,
  pageCount: number,
  students: KnownStudent[],
): IdentityAnchoredResult {
  const owners: (PageOwner | null)[] = [];
  for (let p = 1; p <= pageCount; p++) {
    owners.push(detectPageOwner(pageTexts[String(p)] || "", students));
  }

  const detectedPages = owners.filter(Boolean).length;
  const distinctOwners = new Set(owners.filter((o): o is PageOwner => Boolean(o)).map((o) => o.key)).size;

  const makeSeg = (start: number, end: number, owner: PageOwner | null): OcrDocumentSegment => ({
    pageStart: start,
    pageEnd: end,
    nom: owner?.nom,
    prenom: owner?.prenom,
    ine: owner?.ine,
    folderName: owner?.folderName,
    label: owner
      ? `${owner.prenom ?? ""} ${owner.nom ?? ""}`.trim() || `Pages ${start}-${end}`
      : start === end
        ? `Page ${start}`
        : `Pages ${start}-${end}`,
  });

  const segments: OcrDocumentSegment[] = [];
  let segStart = 1;
  let segOwner: PageOwner | null = null;

  for (let p = 1; p <= pageCount; p++) {
    const id = owners[p - 1];
    if (!id) continue; // page sans identité → continuation
    if (segOwner === null) {
      segOwner = id; // adopte l'identité (rattache d'éventuelles pages de garde initiales)
    } else if (id.key !== segOwner.key) {
      segments.push(makeSeg(segStart, p - 1, segOwner));
      segStart = p;
      segOwner = id;
    }
  }
  segments.push(makeSeg(segStart, pageCount, segOwner));

  return { segments, detectedPages, distinctOwners, pageCount };
}

/**
 * Pages N où une coupure est probablement sûre (entre la page N et N+1).
 * Basé sur changement d'INE ou de contenu — ne coupe pas au milieu d'un bulletin 2 pages.
 */
export function findDocumentBoundaryAfterPages(
  pageTexts: Record<string, string>,
  pageCount: number,
): number[] {
  if (pageCount <= 1) return [];

  const pages = Array.from({ length: pageCount }, (_, i) => i + 1);
  const fingerprints = pages.map((p) => pageFingerprint(pageTexts[String(p)] || ""));
  const cuts: number[] = [];

  for (let i = 1; i < pages.length; i++) {
    const prev = fingerprints[i - 1];
    const curr = fingerprints[i];
    const sameStudent =
      prev.startsWith("ine:") && curr.startsWith("ine:") && prev === curr;
    if (sameStudent) continue;

    const boundary =
      prev !== curr &&
      (prev !== "h:" || curr !== "h:") &&
      ((prev.startsWith("ine:") && curr.startsWith("ine:") && prev !== curr) ||
        !prev.startsWith("ine:") ||
        !curr.startsWith("ine:"));

    if (boundary) cuts.push(pages[i - 1]);
  }

  return cuts;
}

/** Blocs de pages pour Mistral : ≤ maxPages, coupures uniquement aux frontières détectées. */
export function buildSafeMistralChunks(
  pageCount: number,
  cutAfterPages: number[],
  maxPagesPerChunk: number,
): Array<{ start: number; end: number }> {
  if (pageCount <= 0) return [];
  if (pageCount <= maxPagesPerChunk) return [{ start: 1, end: pageCount }];

  const cuts = new Set(cutAfterPages.filter((p) => p >= 1 && p < pageCount));
  const chunks: Array<{ start: number; end: number }> = [];
  let start = 1;

  while (start <= pageCount) {
    let end = Math.min(start + maxPagesPerChunk - 1, pageCount);

    if (end < pageCount && cuts.size > 0) {
      while (end > start && !cuts.has(end)) end--;
      if (!cuts.has(end) && end < pageCount) {
        let extended = Math.min(start + maxPagesPerChunk - 1, pageCount);
        while (extended < pageCount && !cuts.has(extended)) extended++;
        end = extended >= pageCount ? pageCount : extended;
      }
    }

    if (end < start) end = start;
    chunks.push({ start, end });
    start = end + 1;
  }

  return chunks;
}

export function slicePageTexts(
  pageTexts: Record<string, string>,
  start: number,
  end: number,
): Record<string, string> {
  const out: Record<string, string> = {};
  for (let p = start; p <= end; p++) {
    const key = String(p);
    if (pageTexts[key] !== undefined) out[key] = pageTexts[key];
  }
  return out;
}

/** Fusionne les segments adjacents (même élève sur une frontière de bloc). */
export function mergeAdjacentSegments(segments: OcrDocumentSegment[]): OcrDocumentSegment[] {
  const sorted = [...segments].sort((a, b) => a.pageStart - b.pageStart);
  const merged: OcrDocumentSegment[] = [];

  for (const seg of sorted) {
    const last = merged[merged.length - 1];
    if (!last) {
      merged.push({ ...seg });
      continue;
    }
    const sameIne =
      last.ine && seg.ine && last.ine.toUpperCase() === seg.ine.toUpperCase();
    const adjacent = last.pageEnd + 1 === seg.pageStart;
    if (adjacent && sameIne) {
      last.pageEnd = Math.max(last.pageEnd, seg.pageEnd);
      if (!last.ine && seg.ine) last.ine = seg.ine;
      if (!last.nom && seg.nom) last.nom = seg.nom;
      if (!last.prenom && seg.prenom) last.prenom = seg.prenom;
      continue;
    }
    merged.push({ ...seg });
  }

  return merged;
}

/**
 * Repli si Mistral dépasse le timeout hébergeur (~30 s).
 * Export Charlemagne : souvent 1 bulletin = 1 page, ou 2 pages si INE identique.
 */
export function heuristicClassSegments(
  pageTexts: Record<string, string>,
  pageCount: number,
): { mode: "single" | "multi"; segments: OcrDocumentSegment[] } {
  if (pageCount <= 1) {
    return {
      mode: "single",
      segments: [{ pageStart: 1, pageEnd: 1 }],
    };
  }

  const pages = Array.from({ length: pageCount }, (_, i) => i + 1);
  const fingerprints = pages.map((p) => pageFingerprint(pageTexts[String(p)] || ""));

  const segments: OcrDocumentSegment[] = [];
  let segStart = 1;

  for (let i = 1; i < pages.length; i++) {
    const prev = fingerprints[i - 1];
    const curr = fingerprints[i];
    const sameStudent =
      prev.startsWith("ine:") && curr.startsWith("ine:") && prev === curr;
    if (sameStudent) continue;

    const boundary =
      (prev !== curr && prev !== "h:" && curr !== "h:") ||
      (prev.startsWith("ine:") && curr.startsWith("ine:") && prev !== curr);

    if (boundary) {
      segments.push({ pageStart: segStart, pageEnd: pages[i - 1] });
      segStart = pages[i];
    }
  }
  segments.push({ pageStart: segStart, pageEnd: pages[pages.length - 1] });

  // Aucune frontière fiable trouvée → on NE découpe PAS page par page (cause de bugs).
  // On préfère un seul document couvrant tout le PDF, quitte à le classer en un bloc.
  if (segments.length <= 1) {
    return { mode: "single", segments: [{ pageStart: 1, pageEnd: pageCount }] };
  }

  return { mode: "multi", segments };
}

/** Pages couvertes par les segments (1-indexées). */
export function maxSegmentPageEnd(segments: OcrDocumentSegment[]): number {
  if (segments.length === 0) return 0;
  return Math.max(...segments.map((s) => s.pageEnd));
}

/**
 * Complète les segments si l'IA n'a vu qu'une partie du PDF (digest tronqué).
 * Les pages manquantes sont RATTACHÉES au segment voisin (continuation du même document),
 * jamais transformées en mini-documents d'une page (ce qui produisait le découpage page/page).
 */
export function ensureFullPageCoverage(
  segments: OcrDocumentSegment[],
  pageCount: number,
): { segments: OcrDocumentSegment[]; coverageFixed: boolean } {
  if (pageCount <= 0 || segments.length === 0) {
    return { segments, coverageFixed: false };
  }

  const sorted = [...segments].sort((a, b) => a.pageStart - b.pageStart);
  let fixed = false;

  // Étend chaque segment jusqu'au début du suivant pour absorber les pages manquantes intermédiaires.
  for (let i = 0; i < sorted.length - 1; i++) {
    const gapStart = sorted[i].pageEnd + 1;
    const nextStart = sorted[i + 1].pageStart;
    if (nextStart > gapStart) {
      sorted[i] = { ...sorted[i], pageEnd: nextStart - 1 };
      fixed = true;
    }
  }

  // Pages de garde avant le premier segment → rattachées au premier document.
  if (sorted[0].pageStart > 1) {
    sorted[0] = { ...sorted[0], pageStart: 1 };
    fixed = true;
  }
  // Pages restantes après le dernier segment → rattachées au dernier document.
  const last = sorted[sorted.length - 1];
  if (last.pageEnd < pageCount) {
    sorted[sorted.length - 1] = { ...last, pageEnd: pageCount };
    fixed = true;
  }

  return { segments: sorted, coverageFixed: fixed };
}
