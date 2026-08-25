import { NextResponse } from "next/server";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissement } from "@/db/schema";
import { runOmogenWeeklySync } from "@/app/lib/nomenclature-import/omogen-sync";

function authorizeCron(req: Request, body: Record<string, unknown>): boolean {
  const secret = process.env.OMOGEN_CRON_SECRET?.trim();
  if (!secret) return false;
  const header =
    req.headers.get("x-omogen-cron-secret") ||
    req.headers.get("authorization") ||
    "";
  if (header === secret || header === `Bearer ${secret}`) return true;
  return body.cronSecret === secret;
}

/**
 * Job hebdo Omogen — sans session admin.
 * Auth : OMOGEN_CRON_SECRET (header x-omogen-cron-secret | Authorization | body.cronSecret).
 * Cible : body.etablissementId | OMOGEN_ETABLISSEMENT_ID | tous les établissements.
 */
export async function POST(req: Request) {
  if (!isDatabaseConfigured()) {
    return NextResponse.json({ error: "Base indisponible." }, { status: 503 });
  }

  const body = (await req.json().catch(() => ({}))) as Record<string, unknown>;
  if (!authorizeCron(req, body)) {
    return NextResponse.json(
      { error: "Non autorisé — OMOGEN_CRON_SECRET requis.", code: "CRON_UNAUTHORIZED" },
      { status: 401 },
    );
  }

  if (process.env.OMOGEN_ENABLED?.trim() !== "1") {
    return NextResponse.json(
      { ok: false, message: "OMOGEN_ENABLED≠1 — sync ignorée." },
      { status: 503 },
    );
  }

  const db = getDb();
  const forcedId =
    String(body.etablissementId || "").trim() ||
    process.env.OMOGEN_ETABLISSEMENT_ID?.trim() ||
    "";

  let etabIds: string[] = [];
  if (forcedId) {
    etabIds = [forcedId];
  } else {
    const rows = await db.select({ id: etablissement.id }).from(etablissement);
    etabIds = rows.map((r) => r.id);
  }

  if (!etabIds.length) {
    return NextResponse.json({ ok: false, message: "Aucun établissement cible." }, { status: 400 });
  }

  const results: Array<{
    etablissementId: string;
    ok: boolean;
    message: string;
    reports?: OmogenReport[];
  }> = [];

  for (const id of etabIds) {
    try {
      const result = await runOmogenWeeklySync(id);
      results.push({
        etablissementId: id,
        ok: result.ok,
        message: result.message,
        reports: result.reports,
      });
    } catch (e) {
      results.push({
        etablissementId: id,
        ok: false,
        message: e instanceof Error ? e.message : "Erreur sync.",
      });
    }
  }

  const okCount = results.filter((r) => r.ok).length;
  return NextResponse.json({
    ok: okCount === results.length,
    synced: okCount,
    total: results.length,
    results,
  });
}

type OmogenReport = { file: string; message?: string; error?: string; kind?: string };
