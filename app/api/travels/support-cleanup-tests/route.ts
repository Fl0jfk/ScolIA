import { NextResponse } from "next/server";
import { z } from "zod";
import { deleteTravelFromDb, travelsDbReady } from "@/app/lib/travel-db";
import { listTravelsIndex } from "@/app/lib/travels-storage";
import { normalizeTravelsSearchText } from "@/app/lib/travels-trip-helpers";
import type { TravelsTrip } from "@/app/lib/travels-types";

const BodySchema = z.object({
  /** false = suppression réelle (défaut : dry-run). */
  dryRun: z.boolean().optional().default(true),
});

/** Titres / destinations clairement identifiés comme jeux de test. */
const TEST_TITLE_NEEDLES = ["parc asterix", "asterix", "disneyland", "disney land"] as const;

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

function isTravelTestJunk(trip: TravelsTrip): boolean {
  const hay = normalizeTravelsSearchText(
    [trip.data?.title, trip.data?.destination].filter(Boolean).join(" "),
  );
  if (!hay) return false;
  return TEST_TITLE_NEEDLES.some((needle) => hay.includes(needle));
}

function summarize(trip: TravelsTrip) {
  return {
    id: trip.id,
    title: trip.data?.title || "",
    destination: trip.data?.destination || "",
    status: trip.status,
    etablissement: trip.data?.etablissement || "",
    startDate: trip.data?.startDate || trip.data?.date || "",
    endDate: trip.data?.endDate || "",
  };
}

/**
 * Nettoyage ciblé des dossiers voyage de test (Parc Astérix / Disneyland).
 * Auth : header `x-scola-support-secret` (= OCR_WORKER_SECRET).
 * GET = liste candidats ; POST { dryRun: false } = suppression unitaire (cascade attrs).
 */
export async function GET(req: Request) {
  if (!supportSecretOk(req)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  const trips = await listTravelsIndex();
  const matches = trips.filter(isTravelTestJunk).map(summarize);
  return NextResponse.json({ count: matches.length, matches });
}

export async function POST(req: Request) {
  if (!supportSecretOk(req)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  const parsed = BodySchema.safeParse(await req.json().catch(() => ({})));
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  const trips = await listTravelsIndex();
  const matches = trips.filter(isTravelTestJunk);
  const summary = matches.map(summarize);

  if (parsed.data.dryRun !== false) {
    return NextResponse.json({
      dryRun: true,
      deleted: 0,
      count: summary.length,
      matches: summary,
    });
  }

  const etabId = await travelsDbReady();
  if (!etabId) {
    return NextResponse.json({ error: "Base voyages indisponible." }, { status: 503 });
  }

  const deleted: string[] = [];
  const errors: Array<{ id: string; error: string }> = [];
  for (const trip of matches) {
    try {
      await deleteTravelFromDb(etabId, String(trip.id));
      deleted.push(String(trip.id));
    } catch (e) {
      errors.push({
        id: String(trip.id),
        error: e instanceof Error ? e.message : String(e),
      });
    }
  }

  return NextResponse.json({
    dryRun: false,
    deleted: deleted.length,
    deletedIds: deleted,
    errors,
    matches: summary,
  });
}
