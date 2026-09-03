import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { etablissement } from "@/db/schema";
import { searchAccueilPersonnes } from "@/app/lib/accueil-absences-search";

/**
 * Smoke test prod (header secret) — à retirer après validation.
 * GET /api/internal/accueil-search-smoke?q=xxx&slug=la-providence-nicolas-barre
 */
export async function GET(req: Request) {
  const secret = process.env.OCR_WORKER_SECRET?.trim();
  if (!secret || req.headers.get("x-ocr-worker-secret") !== secret) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim() || "a";
  const slug =
    url.searchParams.get("slug")?.trim() ||
    process.env.DEFAULT_TENANT_SLUG?.trim() ||
    "la-providence-nicolas-barre";

  const db = getDb();
  const [etab] = await db
    .select({ id: etablissement.id, slug: etablissement.slug, name: etablissement.name })
    .from(etablissement)
    .where(eq(etablissement.slug, slug))
    .limit(1);

  if (!etab) {
    return NextResponse.json({ error: "etablissement introuvable", slug }, { status: 404 });
  }

  const needle = q.length >= 3 ? q : "dup";
  const hits = await searchAccueilPersonnes(etab.id, needle);
  const byKind: Record<string, number> = {};
  for (const h of hits) {
    byKind[h.kind] = (byKind[h.kind] || 0) + 1;
  }

  return NextResponse.json({
    ok: true,
    etab: { id: etab.id, slug: etab.slug, name: etab.name },
    query: needle,
    total: hits.length,
    byKind,
    sample: hits.slice(0, 12).map((h) => ({
      kind: h.kind,
      scope: h.scope ?? null,
      displayName: h.displayName,
      subtitle: h.subtitle,
    })),
  });
}
