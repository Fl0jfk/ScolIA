import { NextResponse } from "next/server";
import { z } from "zod";
import { and, eq, isNotNull, ne, sql } from "drizzle-orm";

import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve } from "@/db/schema";
import {
  isEstablishmentDirectionEmail,
  normalizeEmailAddress,
} from "@/app/lib/eleve-direction-email";
import { invalidateElevesRegistryCache } from "@/app/lib/eleves-registry";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

const BodySchema = z.object({
  /** false = écriture réelle (défaut : dry-run). */
  dryRun: z.boolean().optional().default(true),
});

function supportSecretOk(req: Request): boolean {
  const supportSecret =
    req.headers.get("x-scola-support-secret")?.trim() ||
    req.headers.get("x-ocr-worker-secret")?.trim() ||
    new URL(req.url).searchParams.get("support_secret")?.trim() ||
    "";
  const expected =
    process.env.OCR_WORKER_SECRET?.trim() ||
    process.env.TRAVEL_EMAIL_INGEST_SECRET?.trim() ||
    "";
  return Boolean(expected && supportSecret && supportSecret === expected);
}

/**
 * Vide le champ `email` élève lorsqu’il contient un mail CE / RNE institutionnel
 * (ex. ce.0762565a@ac-normandie.fr). N’effectue pas de DELETE de lignes.
 *
 * Auth : header `x-scola-support-secret` (= OCR_WORKER_SECRET).
 * GET = inventaire ; POST { dryRun: false } = UPDATE ciblé + invalidation cache.
 */
export async function GET(req: Request) {
  if (!supportSecretOk(req)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!isDatabaseConfigured()) {
    return NextResponse.json({ error: "Base indisponible." }, { status: 503 });
  }

  const db = getDb();
  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      email: eleve.email,
      etablissementId: eleve.etablissementId,
    })
    .from(eleve)
    .where(and(isNotNull(eleve.email), ne(sql`trim(${eleve.email})`, "")));

  const hits = rows.filter((r) => isEstablishmentDirectionEmail(r.email));
  const byEmail: Record<string, number> = {};
  for (const h of hits) {
    const k = normalizeEmailAddress(h.email);
    byEmail[k] = (byEmail[k] || 0) + 1;
  }

  return NextResponse.json({
    count: hits.length,
    byEmail,
    sample: hits.slice(0, 20).map((h) => ({
      nom: h.nom,
      prenom: h.prenom,
      classe: h.classe,
      email: h.email,
    })),
  });
}

export async function POST(req: Request) {
  if (!supportSecretOk(req)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!isDatabaseConfigured()) {
    return NextResponse.json({ error: "Base indisponible." }, { status: 503 });
  }

  const parsed = BodySchema.safeParse(await req.json().catch(() => ({})));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  const db = getDb();
  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      email: eleve.email,
      etablissementId: eleve.etablissementId,
    })
    .from(eleve)
    .where(and(isNotNull(eleve.email), ne(sql`trim(${eleve.email})`, "")));

  const hits = rows.filter((r) => isEstablishmentDirectionEmail(r.email));
  const byEmail: Record<string, number> = {};
  for (const h of hits) {
    const k = normalizeEmailAddress(h.email);
    byEmail[k] = (byEmail[k] || 0) + 1;
  }

  if (parsed.data.dryRun !== false) {
    return NextResponse.json({
      dryRun: true,
      cleared: 0,
      count: hits.length,
      byEmail,
      sample: hits.slice(0, 10).map((h) => ({
        nom: h.nom,
        prenom: h.prenom,
        classe: h.classe,
        email: h.email,
      })),
    });
  }

  const ids = hits.map((h) => h.id);
  let cleared = 0;
  // Updates unitaires par id — jamais de wipe table.
  for (const id of ids) {
    const updated = await db
      .update(eleve)
      .set({ email: null })
      .where(and(eq(eleve.id, id), isNotNull(eleve.email)))
      .returning({ id: eleve.id });
    cleared += updated.length;
  }

  const etabIds = [...new Set(hits.map((h) => h.etablissementId).filter(Boolean))];
  for (const etabId of etabIds) {
    await invalidateElevesRegistryCache(etabId);
  }
  // Aussi invalider le contexte courant (tenant de la requête).
  const currentEtab = await resolveCurrentEtablissementId();
  if (currentEtab && !etabIds.includes(currentEtab)) {
    await invalidateElevesRegistryCache(currentEtab);
  }

  const remaining = await db
    .select({ id: eleve.id, email: eleve.email })
    .from(eleve)
    .where(and(isNotNull(eleve.email), ne(sql`trim(${eleve.email})`, "")));
  const stillBad = remaining.filter((r) => isEstablishmentDirectionEmail(r.email)).length;

  return NextResponse.json({
    dryRun: false,
    cleared,
    remainingDirectionEmails: stillBad,
    byEmail,
  });
}
