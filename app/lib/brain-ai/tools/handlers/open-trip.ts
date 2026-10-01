import "server-only";

import { getJson } from "@/app/lib/s3-storage";
import type { TravelsTrip } from "@/app/lib/travels-types";
import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

async function loadTripsIndex(): Promise<TravelsTrip[]> {
  const hit = await getJson<TravelsTrip[]>("travels/index.json");
  return Array.isArray(hit?.data) ? hit.data : [];
}

function tripLabel(t: TravelsTrip): string {
  const title = t.data?.title || "(sans titre)";
  const dest = t.data?.destination ? ` — ${t.data.destination}` : "";
  const dates =
    t.data?.startDate && t.data?.endDate && t.data.startDate !== t.data.endDate
      ? ` (${t.data.startDate} → ${t.data.endDate})`
      : t.data?.date || t.data?.startDate
        ? ` (${t.data.date || t.data.startDate})`
        : "";
  return `${title}${dest}${dates}`;
}

function scoreTrip(t: TravelsTrip, query: string): number {
  const q = query.toLowerCase().trim();
  if (!q) return 0;
  const hay = `${t.data?.title || ""} ${t.data?.destination || ""} ${t.data?.classes || ""} ${t.id}`.toLowerCase();
  let score = 0;
  if (hay === q) score += 20;
  if (hay.includes(q)) score += 10;
  for (const tok of q.split(/\s+/).filter(Boolean)) {
    if (hay.includes(tok)) score += 3;
  }
  return score;
}

/**
 * Ouvre un séjour existant. Si plusieurs matchent, propose un choix + liens CTA.
 */
export async function handleOpenTrip(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const tripId = String(args.tripId || args.id || "").trim();
  const query = String(args.query || args.q || args.title || "").trim();
  const trips = await loadTripsIndex();

  if (tripId) {
    const exists = trips.some((t) => t.id === tripId);
    if (!exists) {
      const full = await getJson<TravelsTrip>(`travels/${tripId}.json`);
      if (!full?.data) {
        return { ok: false, error: "Séjour introuvable.", code: "NOT_FOUND" };
      }
    }
    const href = `/travels/${tripId}`;
    const action: BrainClientAction = { type: "open_route", href, label: "Ouvrir le séjour" };
    return {
      ok: true,
      data: {
        tripId,
        href,
        clientActions: [action],
        ctas: [{ label: "Ouvrir le séjour", href }],
      },
      summaryFr: `J’ouvre le séjour ${tripId}.`,
    };
  }

  if (!query) {
    const recent = [...trips]
      .sort((a, b) =>
        String(b.updatedAt || b.createdAt || "").localeCompare(String(a.updatedAt || a.createdAt || "")),
      )
      .slice(0, 12);
    if (recent.length === 0) {
      return { ok: false, error: "Aucun séjour trouvé." };
    }
    return {
      ok: false,
      needsChoices: true,
      tool: "open_trip",
      field: "tripId",
      promptFr: "Quel séjour voulez-vous ouvrir ?",
      options: recent.map((t) => ({ value: t.id, label: tripLabel(t) })),
      draftArgs: {},
      selectionType: "single",
    };
  }

  const scored = trips
    .map((t) => ({ t, score: scoreTrip(t, query) }))
    .filter((x) => x.score > 0)
    .sort((a, b) => b.score - a.score);

  if (scored.length === 0) {
    return { ok: false, error: `Aucun séjour pour « ${query} ».` };
  }

  if (scored.length > 1) {
    const top = scored.slice(0, 10);
    return {
      ok: false,
      needsChoices: true,
      tool: "open_trip",
      field: "tripId",
      promptFr: `Plusieurs séjours correspondent à « ${query} ». Lequel ouvrir ?`,
      options: top.map((x) => ({ value: x.t.id, label: tripLabel(x.t) })),
      draftArgs: { query },
      selectionType: "single",
    };
  }

  const trip = scored[0]!.t;
  const href = `/travels/${trip.id}`;
  const action: BrainClientAction = {
    type: "open_route",
    href,
    label: tripLabel(trip),
  };
  return {
    ok: true,
    data: {
      tripId: trip.id,
      title: trip.data?.title,
      href,
      clientActions: [action],
      ctas: [{ label: `Ouvrir « ${trip.data?.title || trip.id} »`, href }],
    },
    summaryFr: `J’ouvre « ${trip.data?.title || trip.id} ».`,
  };
}
