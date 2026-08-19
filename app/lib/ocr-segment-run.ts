import "server-only";

import {
  buildPageDigestForSegmentation,
  buildSafeMistralChunks,
  ensureFullPageCoverage,
  findDocumentBoundaryAfterPages,
  heuristicClassSegments,
  identityAnchoredSegments,
  mergeAdjacentSegments,
  slicePageTexts,
  type KnownStudent,
  type OcrDocumentSegment,
} from "@/app/lib/ocr-segmentation";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { ocrTraceCtx, type OcrTraceCtx } from "@/app/lib/ocr-trace";

/** Retry Mistral sur erreurs transitoires (503, 429, 500, 504). */
async function fetchMistralWithRetry(
  url: string,
  init: RequestInit,
  maxAttempts = 3,
): Promise<Response> {
  const RETRYABLE = new Set([429, 500, 503, 504]);
  let lastRes: Response | null = null;
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    const res = await fetch(url, init);
    if (res.ok || !RETRYABLE.has(res.status)) return res;
    lastRes = res;
    if (attempt < maxAttempts) {
      await new Promise((r) => setTimeout(r, 1000 * 2 ** (attempt - 1)));
    }
  }
  return lastRes!;
}

const MISTRAL_TIMEOUT_MS = 22_000;
const SEGMENTATION_MODEL = "mistral-small-latest";
const DIGEST_PROMPT_LIMIT = 32_000;
/** Appel Mistral unique si le PDF tient dans une requête. */
export const MISTRAL_SINGLE_CALL_MAX_PAGES = 30;
/** Taille max d'un bloc Mistral (coupures uniquement entre documents). */
export const MISTRAL_CHUNK_MAX_PAGES = 30;

export type SegmentationEngine = "identity" | "mistral" | "mistral_chunked" | "heuristic";

/** Couverture minimale (pages avec élève détecté) pour faire confiance au découpage par identité. */
const IDENTITY_MIN_COVERAGE = 0.5;

export function resolveSegmentationEngine(pageCount: number): SegmentationEngine {
  return pageCount > MISTRAL_SINGLE_CALL_MAX_PAGES ? "mistral_chunked" : "mistral";
}

export type DocumentSegment = OcrDocumentSegment;

function parseSegmentationJson(raw: string): {
  mode: "single" | "multi";
  segments: DocumentSegment[];
} | null {
  let content = raw.trim();
  content = content.replace(/`{3}json/gi, "").replace(/`{3}/g, "");
  const start = content.indexOf("{");
  const end = content.lastIndexOf("}");
  if (start === -1 || end === -1) return null;
  try {
    const parsed = JSON.parse(content.slice(start, end + 1)) as {
      mode?: string;
      segments?: DocumentSegment[];
    };
    const mode = parsed.mode === "multi" ? "multi" : "single";
    const segments = Array.isArray(parsed.segments) ? parsed.segments : [];
    const cleaned = segments
      .map((s) => ({
        pageStart: Math.max(1, Number(s.pageStart) || 1),
        pageEnd: Math.max(1, Number(s.pageEnd) || 1),
        type: s.type,
        nom: s.nom,
        prenom: s.prenom,
        ine: s.ine,
        label: s.label,
      }))
      .map((s) => (s.pageEnd < s.pageStart ? { ...s, pageEnd: s.pageStart } : s));
    if (cleaned.length === 0 && mode === "single") {
      return { mode: "single", segments: [{ pageStart: 1, pageEnd: 1 }] };
    }
    return { mode, segments: cleaned };
  } catch {
    return null;
  }
}

function finalizeSegments(mode: "single" | "multi", segments: DocumentSegment[], pageCount: number) {
  let m = mode;
  let segs = segments;
  if (segs.length === 0) {
    m = "single";
    segs = [{ pageStart: 1, pageEnd: pageCount > 0 ? pageCount : 1 }];
  }
  if (segs.length === 1) m = "single";
  return { mode: m, segments: segs, segmentCount: segs.length };
}

async function callMistralSegmentation(
  digest: string,
  pageCount: number,
  opts?: { absoluteStart?: number; absoluteEnd?: number; trace?: OcrTraceCtx },
): Promise<{ mode: "single" | "multi"; segments: DocumentSegment[] } | null> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) {
    ocrTraceCtx(opts?.trace, "segment", "mistral-skip", "pas de clé Mistral", undefined, "warn");
    return null;
  }

  const absStart = opts?.absoluteStart;
  const absEnd = opts?.absoluteEnd;
  const pagesHint =
    absStart && absEnd
      ? `Ce bloc couvre les pages ${absStart} à ${absEnd} du PDF complet. Utilise OBLIGATOIREMENT ces numéros absolus dans pageStart/pageEnd (ceux indiqués après "--- Page N ---").`
      : pageCount > 0
        ? `Le PDF contient ${pageCount} page(s). Chaque bloc commence par "--- Page N ---".`
        : "Les pages sont marquées par --- Page N ---.";

  const prompt = `
Tu analyses un PDF qui peut contenir un ou plusieurs documents (bulletins scolaires, convocations, courriers, contrats, attestations…).

${pagesHint}

Détermine les DOCUMENTS LOGIQUES DISTINCTS.

Règles IMPORTANTES :
- Un même document s'étale TRÈS SOUVENT sur PLUSIEURS pages consécutives : regroupe-les en UN seul segment.
- Indices qu'une page est la SUITE du document précédent (ne pas couper) :
    • pas de nouveau destinataire / nouveau nom clairement identifiable en en-tête
    • numéro de document, de convocation ou de dossier identique à la page précédente
    • mention "page X/Y" ou "suite" ou "…/2"
    • le contenu est la continuation logique (liste de matières, annexe, tableau de frais…)
- Ne crée un nouveau segment QUE lorsqu'un NOUVEAU document distinct commence clairement.
- Une page ambiguë ou sans marqueur de rupture est la SUITE du document courant : ne la sépare JAMAIS.
- N'émets JAMAIS un segment d'une seule page par défaut si la page suivante peut en être la suite.
- Segments sans chevauchement, couvrant toutes les pages.
- Ne devine pas : nom/prénom/ine seulement si visibles dans le résumé de page.

JSON uniquement :
{"mode":"single"|"multi","segments":[{"pageStart":1,"pageEnd":1,"nom":null,"prenom":null,"ine":null,"label":"..."}]}

Résumé OCR par page :
---
${digest.slice(0, DIGEST_PROMPT_LIMIT)}
---
`;

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), MISTRAL_TIMEOUT_MS);
  const chunkLabel =
    opts?.absoluteStart && opts?.absoluteEnd
      ? `p.${opts.absoluteStart}-${opts.absoluteEnd}`
      : `pages=${pageCount}`;

  ocrTraceCtx(opts?.trace, "segment", "mistral-call", "appel Mistral segmentation", {
    chunk: chunkLabel,
    digestChars: digest.length,
    model: SEGMENTATION_MODEL,
  });

  try {
    const res = await fetchMistralWithRetry("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      signal: controller.signal,
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: SEGMENTATION_MODEL,
        messages: [{ role: "user", content: prompt }],
        temperature: 0,
        response_format: { type: "json_object" },
      }),
    });

    if (!res.ok) {
      ocrTraceCtx(opts?.trace, "segment", "mistral-http-fail", "Mistral segmentation HTTP erreur", {
        status: res.status,
        chunk: chunkLabel,
      }, "warn");
      return null;
    }
    const data = await res.json();
    const raw = data.choices?.[0]?.message?.content || "";
    const parsed = parseSegmentationJson(raw);
    ocrTraceCtx(opts?.trace, "segment", "mistral-ok", "réponse Mistral segmentation", {
      chunk: chunkLabel,
      parsed: parsed
        ? { mode: parsed.mode, segmentCount: parsed.segments.length }
        : null,
      rawChars: raw.length,
    });
    return parsed;
  } catch (err) {
    ocrTraceCtx(opts?.trace, "segment", "mistral-error", "Mistral segmentation exception", {
      chunk: chunkLabel,
      error: err instanceof Error ? err.message : String(err),
    }, "warn");
    return null;
  } finally {
    clearTimeout(timer);
  }
}

async function segmentChunkWithMistral(
  pageTexts: Record<string, string>,
  start: number,
  end: number,
  trace?: OcrTraceCtx,
): Promise<DocumentSegment[] | null> {
  const chunkTexts = slicePageTexts(pageTexts, start, end);
  const built = buildPageDigestForSegmentation(chunkTexts, end - start + 1);
  if (!built.digest.trim()) {
    ocrTraceCtx(trace, "segment", "chunk-empty", "digest vide pour bloc", { pages: `${start}-${end}` }, "warn");
    return null;
  }

  const result = await callMistralSegmentation(built.digest, end - start + 1, {
    absoluteStart: start,
    absoluteEnd: end,
    trace,
  });
  if (!result?.segments.length) return null;

  const clamped = result.segments
    .map((s) => ({
      ...s,
      pageStart: Math.max(start, Math.min(s.pageStart, end)),
      pageEnd: Math.max(start, Math.min(s.pageEnd, end)),
    }))
    .filter((s) => s.pageStart <= s.pageEnd);

  return clamped.length > 0 ? clamped : null;
}

async function segmentWithMistralChunks(
  pageTexts: Record<string, string>,
  pageCount: number,
  trace?: OcrTraceCtx,
): Promise<{ mode: "single" | "multi"; segments: DocumentSegment[] } | null> {
  const boundaries = findDocumentBoundaryAfterPages(pageTexts, pageCount);
  const chunks = buildSafeMistralChunks(pageCount, boundaries, MISTRAL_CHUNK_MAX_PAGES);
  ocrTraceCtx(trace, "segment", "chunks-plan", "plan découpage par blocs", {
    pageCount,
    boundariesCount: boundaries.length,
    chunks: chunks.map((c) => `${c.start}-${c.end}`),
  });
  if (chunks.length === 0) return null;

  const allSegments: DocumentSegment[] = [];

  for (let ci = 0; ci < chunks.length; ci++) {
    const chunk = chunks[ci];
    ocrTraceCtx(trace, "segment", "chunk-start", `bloc ${ci + 1}/${chunks.length}`, {
      pages: `${chunk.start}-${chunk.end}`,
    });
    const mistralSegs = await segmentChunkWithMistral(pageTexts, chunk.start, chunk.end, trace);
    const segs =
      mistralSegs ??
      heuristicClassSegments(slicePageTexts(pageTexts, chunk.start, chunk.end), chunk.end - chunk.start + 1)
        .segments;
    ocrTraceCtx(trace, "segment", "chunk-done", `bloc ${ci + 1}/${chunks.length} segmenté`, {
      pages: `${chunk.start}-${chunk.end}`,
      segmentCount: segs.length,
      fallbackHeuristic: !mistralSegs,
    });
    allSegments.push(...segs);
  }

  const merged = mergeAdjacentSegments(allSegments);
  const fixed = ensureFullPageCoverage(merged, pageCount);
  return {
    mode: fixed.segments.length > 1 ? "multi" : "single",
    segments: fixed.segments,
  };
}

export async function runDocumentSegmentation(
  input: {
    pageTexts?: Record<string, string> | null;
    pageCount?: number;
    text?: string;
    /** Élèves connus (eleves.json) — active le découpage ancré identité (fiable, sans IA). */
    knownStudents?: KnownStudent[];
  },
  trace?: OcrTraceCtx,
) {
  const pageTexts = input.pageTexts && typeof input.pageTexts === "object" ? input.pageTexts : null;
  const pageCount = typeof input.pageCount === "number" && input.pageCount > 0 ? input.pageCount : 0;
  const text = typeof input.text === "string" ? input.text : "";
  const knownStudents = Array.isArray(input.knownStudents) ? input.knownStudents : [];

  ocrTraceCtx(trace, "segment", "run-start", "runDocumentSegmentation", {
    pageCount,
    hasPageTexts: Boolean(pageTexts && Object.keys(pageTexts).length > 0),
    textChars: text.length,
    knownStudents: knownStudents.length,
  });

  let resolvedPageCount =
    pageCount > 0
      ? pageCount
      : pageTexts
        ? Object.keys(pageTexts).length
        : 0;

  // ── Priorité 1 : découpage ancré sur l'identité élève (INE + noms connus). ──
  // Le plus fiable et le plus rapide : aucun appel IA, fonctionne même sur 100+ pages.
  if (pageTexts && resolvedPageCount > 1 && knownStudents.length > 0) {
    const anchored = identityAnchoredSegments(pageTexts, resolvedPageCount, knownStudents);
    const coverage = anchored.detectedPages / resolvedPageCount;
    ocrTraceCtx(trace, "segment", "identity-try", "découpage ancré identité", {
      detectedPages: anchored.detectedPages,
      pageCount: resolvedPageCount,
      coverage: Math.round(coverage * 100) / 100,
      distinctOwners: anchored.distinctOwners,
      minCoverage: IDENTITY_MIN_COVERAGE,
    });
    if (coverage >= IDENTITY_MIN_COVERAGE) {
      const segs =
        anchored.distinctOwners <= 1
          ? [{ ...anchored.segments[0], pageStart: 1, pageEnd: resolvedPageCount }]
          : anchored.segments;
      ocrTraceCtx(trace, "segment", "identity-ok", "découpage par identité retenu", {
        segmentCount: segs.length,
        segments: segs.map((s) => ({ p: `${s.pageStart}-${s.pageEnd}`, eleve: s.folderName ?? s.label ?? null })),
      });
      return {
        ...finalizeSegments(segs.length > 1 ? "multi" : "single", segs, resolvedPageCount),
        pageCount: resolvedPageCount,
        engine: "identity" as const,
      };
    }
    ocrTraceCtx(trace, "segment", "identity-skip", "couverture identité insuffisante — repli IA", {
      coverage: Math.round(coverage * 100) / 100,
    }, "warn");
  }

  if (pageTexts && resolvedPageCount > MISTRAL_SINGLE_CALL_MAX_PAGES) {
    ocrTraceCtx(trace, "segment", "route-chunked", "PDF > 30 pages → mistral_chunked", {
      resolvedPageCount,
    });
    const chunked = await segmentWithMistralChunks(pageTexts, resolvedPageCount, trace);
    if (chunked) {
      return {
        ...finalizeSegments(chunked.mode, chunked.segments, resolvedPageCount),
        pageCount: resolvedPageCount,
        engine: "mistral_chunked" as const,
      };
    }
  }

  let digest = "";
  if (pageTexts && Object.keys(pageTexts).length > 0) {
    const built = buildPageDigestForSegmentation(pageTexts, resolvedPageCount);
    digest = built.digest;
    resolvedPageCount = built.pageCount || resolvedPageCount;
  } else if (text) {
    digest = text.slice(0, DIGEST_PROMPT_LIMIT);
    if (!resolvedPageCount) {
      const matches = text.match(/--- Page (\d+) ---/g) || [];
      resolvedPageCount = matches.length;
    }
  } else {
    throw new Error("pageTexts ou text requis");
  }

  if (!digest.trim()) throw new Error("Texte OCR vide");

  const digestTooLongForMistral = digest.length > DIGEST_PROMPT_LIMIT;

  let result: { mode: "single" | "multi"; segments: DocumentSegment[] } | null = null;
  let engine: SegmentationEngine = "mistral";

  if (pageTexts && resolvedPageCount > 0 && digestTooLongForMistral) {
    ocrTraceCtx(trace, "segment", "route-digest-long", "digest trop long → blocs ou heuristique", {
      digestChars: digest.length,
      limit: DIGEST_PROMPT_LIMIT,
    });
    const chunked = await segmentWithMistralChunks(pageTexts, resolvedPageCount, trace);
    if (chunked) {
      result = chunked;
      engine = "mistral_chunked";
    } else {
      result = heuristicClassSegments(pageTexts, resolvedPageCount);
      engine = "heuristic";
      ocrTraceCtx(trace, "segment", "heuristic-fallback", "repli heuristique (chunked échoué)", {
        segmentCount: result.segments.length,
      }, "warn");
    }
  } else {
    ocrTraceCtx(trace, "segment", "route-single", "appel Mistral unique", {
      resolvedPageCount,
      digestChars: digest.length,
    });
    result = (await callMistralSegmentation(digest, resolvedPageCount, { trace })) ?? null;
    if (result && resolvedPageCount > 0) {
      const fixed = ensureFullPageCoverage(result.segments, resolvedPageCount);
      result = {
        ...result,
        mode: fixed.segments.length > 1 ? "multi" : result.mode,
        segments: fixed.segments,
      };
    }
    if (!result && pageTexts && resolvedPageCount > 0) {
      result = heuristicClassSegments(pageTexts, resolvedPageCount);
      engine = "heuristic";
      ocrTraceCtx(trace, "segment", "heuristic-fallback", "repli heuristique (Mistral single échoué)", {
        segmentCount: result.segments.length,
      }, "warn");
    }
  }

  if (!result) {
    ocrTraceCtx(trace, "segment", "fail", "segmentation impossible", undefined, "error");
    throw new Error("Segmentation impossible");
  }

  ocrTraceCtx(trace, "segment", "run-done", "segmentation terminée", {
    engine,
    mode: result.mode,
    segmentCount: result.segments.length,
    pageCount: resolvedPageCount,
  });

  return {
    ...finalizeSegments(result.mode, result.segments, resolvedPageCount),
    pageCount: resolvedPageCount,
    engine,
  };
}
