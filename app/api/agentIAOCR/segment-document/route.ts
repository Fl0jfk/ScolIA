import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";

import {
  buildPageDigestForSegmentation,
  ensureFullPageCoverage,
  heuristicClassSegments,
  maxSegmentPageEnd,
  type OcrDocumentSegment,
} from "@/app/lib/ocr-segmentation";
import { getMistralApiKey } from "@/app/lib/tenant-config";

export const maxDuration = 60;

const MISTRAL_TIMEOUT_MS = 22_000;
const SEGMENTATION_MODEL = "mistral-small-2603";
/** Résumé OCR envoyé à Mistral — au-delà, l'IA ne voit pas toutes les pages. */
const DIGEST_PROMPT_LIMIT = 32_000;
/** Au-dessus de ce seuil, on préfère l'heuristique (toutes les pages). */
const MISTRAL_MAX_RELIABLE_PAGES = 42;

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
      .map((s) =>
        s.pageEnd < s.pageStart ? { ...s, pageEnd: s.pageStart } : s,
      );
    if (cleaned.length === 0 && mode === "single") {
      return { mode: "single", segments: [{ pageStart: 1, pageEnd: 1 }] };
    }
    return { mode, segments: cleaned };
  } catch {
    return null;
  }
}

function finalizeSegments(
  mode: "single" | "multi",
  segments: DocumentSegment[],
  pageCount: number,
) {
  let m = mode;
  let segs = segments;
  if (segs.length === 0) {
    m = "single";
    segs = [
      {
        pageStart: 1,
        pageEnd: pageCount > 0 ? pageCount : 1,
      },
    ];
  }
  if (segs.length === 1) {
    m = "single";
  }
  return { mode: m, segments: segs, segmentCount: segs.length };
}

async function callMistralSegmentation(
  digest: string,
  pageCount: number,
): Promise<{ mode: "single" | "multi"; segments: DocumentSegment[] } | null> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) return null;

  const pagesHint =
    pageCount > 0
      ? `Le PDF contient ${pageCount} page(s). Chaque bloc commence par "--- Page N ---".`
      : "Les pages sont marquées par --- Page N ---.";

  const prompt = `
Tu analyses un export PDF scolaire (souvent Charlemagne : bulletins de toute une classe).

${pagesHint}

Détermine les DOCUMENTS LOGIQUES DISTINCTS (souvent un bulletin par élève).

Règles :
- Un bulletin par élève = un segment (1 ou plusieurs pages consécutives).
- Pages 1-indexées. Segments sans chevauchement, couvrant toutes les pages.
- Ne devine pas : nom/prénom/ine seulement si visibles dans le résumé de page.
- En cas de doute, préfère couper entre deux pages plutôt qu'au milieu d'un bulletin.

JSON uniquement :
{"mode":"single"|"multi","segments":[{"pageStart":1,"pageEnd":1,"nom":null,"prenom":null,"ine":null,"label":"..."}]}

Résumé OCR par page :
---
${digest.slice(0, DIGEST_PROMPT_LIMIT)}
---
`;

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), MISTRAL_TIMEOUT_MS);

  try {
    const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
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
      console.error("[segment-document] Mistral HTTP", res.status, await res.text());
      return null;
    }

    const data = await res.json();
    const raw = data.choices?.[0]?.message?.content || "";
    const parsed = parseSegmentationJson(raw);
    if (!parsed) {
      console.error("[segment-document] JSON invalide", raw.slice(0, 500));
      return null;
    }
    return parsed;
  } catch (err) {
    console.error("[segment-document] Mistral", err);
    return null;
  } finally {
    clearTimeout(timer);
  }
}

export async function POST(req: Request) {
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const session = await resolveSession();
    const userId = session?.userId;
    if (!userId) {
      return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
    }

    const body = await req.json();
    const pageTexts =
      body.pageTexts && typeof body.pageTexts === "object"
        ? (body.pageTexts as Record<string, string>)
        : null;
    const pageCount =
      typeof body.pageCount === "number" && body.pageCount > 0
        ? body.pageCount
        : 0;
    const text = typeof body.text === "string" ? body.text : "";

    let digest = "";
    let resolvedPageCount = pageCount;

    if (pageTexts && Object.keys(pageTexts).length > 0) {
      const built = buildPageDigestForSegmentation(pageTexts, pageCount);
      digest = built.digest;
      resolvedPageCount = built.pageCount || pageCount;
    } else if (text) {
      digest = text.slice(0, DIGEST_PROMPT_LIMIT);
      if (!resolvedPageCount) {
        const matches = text.match(/--- Page (\d+) ---/g) || [];
        resolvedPageCount = matches.length;
      }
    } else {
      return NextResponse.json(
        { error: "pageTexts ou text requis" },
        { status: 400 },
      );
    }

    if (!digest.trim()) {
      return NextResponse.json({ error: "Texte OCR vide" }, { status: 400 });
    }

    let usedFallback = false;
    let coverageFixed = false;
    const digestTooLongForMistral = digest.length > DIGEST_PROMPT_LIMIT;
    const tooManyPagesForMistral =
      resolvedPageCount > MISTRAL_MAX_RELIABLE_PAGES;

    let result: {
      mode: "single" | "multi";
      segments: DocumentSegment[];
    } | null = null;

    if (
      pageTexts &&
      resolvedPageCount > 0 &&
      (digestTooLongForMistral || tooManyPagesForMistral)
    ) {
      result = heuristicClassSegments(pageTexts, resolvedPageCount);
      usedFallback = true;
      console.warn(
        "[segment-document] heuristique (PDF trop long pour Mistral)",
        result.segments.length,
        "segments,",
        resolvedPageCount,
        "pages",
      );
    } else {
      result = (await callMistralSegmentation(digest, resolvedPageCount)) ?? null;

      if (result && resolvedPageCount > 0) {
        const before = maxSegmentPageEnd(result.segments);
        const fixed = ensureFullPageCoverage(result.segments, resolvedPageCount);
        result = {
          ...result,
          mode: fixed.segments.length > 1 ? "multi" : result.mode,
          segments: fixed.segments,
        };
        if (fixed.coverageFixed) {
          coverageFixed = true;
          console.warn(
            "[segment-document] couverture complétée",
            before,
            "→",
            resolvedPageCount,
            "pages,",
            fixed.segments.length,
            "segments",
          );
        }
      }

      if (!result && pageTexts && resolvedPageCount > 0) {
        result = heuristicClassSegments(pageTexts, resolvedPageCount);
        usedFallback = true;
        console.warn(
          "[segment-document] repli heuristique",
          result.segments.length,
          "segments",
        );
      }
    }

    if (!result) {
      return NextResponse.json(
        { error: "Segmentation impossible (timeout ou erreur IA)" },
        { status: 503 },
      );
    }

    const finalized = finalizeSegments(
      result.mode,
      result.segments,
      resolvedPageCount,
    );

    return NextResponse.json({
      ...finalized,
      usedFallback,
      coverageFixed,
      pageCount: resolvedPageCount,
    });
  } catch (error) {
    console.error("[segment-document]", error);
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
