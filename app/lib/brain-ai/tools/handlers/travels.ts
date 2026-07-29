import { getJson } from "@/app/lib/s3-storage";
import {
  TRAVELS_STATUS_LABELS,
  type TravelsTrip,
} from "@/app/lib/travels-types";
import { putJson } from "@/app/lib/s3-storage";
import { syncTripActualite } from "@/app/lib/brain-ai/sync/knowledge-writer";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

function tripDates(t: TravelsTrip): string {
  const d = t.data || {};
  if (d.startDate && d.endDate && d.startDate !== d.endDate) {
    return `${d.startDate} → ${d.endDate}`;
  }
  return String(d.date || d.startDate || d.endDate || "");
}

function tripBrief(t: TravelsTrip) {
  const status = String(t.status || "");
  return {
    id: t.id,
    title: t.data?.title || "(sans titre)",
    destination: t.data?.destination || null,
    dates: tripDates(t) || null,
    classes: t.data?.classes || null,
    type: t.type,
    status,
    statusLabel: TRAVELS_STATUS_LABELS[status] || status,
    ownerName: t.ownerName || null,
  };
}

async function loadTripsIndex(): Promise<TravelsTrip[]> {
  const hit = await getJson<TravelsTrip[]>("travels/index.json");
  return Array.isArray(hit?.data) ? hit.data : [];
}

export async function handleListTripsBrief(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const limit = Math.min(Math.max(Number(args.limit) || 12, 1), 40);
  const trips = await loadTripsIndex();
  const sorted = [...trips].sort((a, b) => {
    const da = tripDates(a) || a.updatedAt || a.createdAt || "";
    const db = tripDates(b) || b.updatedAt || b.createdAt || "";
    return db.localeCompare(da);
  });
  const items = sorted.slice(0, limit).map(tripBrief);
  return {
    ok: true,
    data: { trips: items, total: trips.length },
    summaryFr:
      items.length === 0
        ? "Aucun séjour trouvé."
        : `${items.length} séjour(s) : ${items
            .slice(0, 5)
            .map((t) => `${t.title} (${t.statusLabel})`)
            .join(" · ")}.`,
  };
}

export async function handleGetTripStatus(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const tripId = typeof args.tripId === "string" ? args.tripId.trim() : "";
  const query = typeof args.query === "string" ? args.query.trim().toLowerCase() : "";
  const trips = await loadTripsIndex();

  let trip: TravelsTrip | undefined;
  if (tripId) {
    trip = trips.find((t) => t.id === tripId);
    if (!trip) {
      const full = await getJson<TravelsTrip>(`travels/${tripId}.json`);
      trip = full?.data;
    }
  } else if (query) {
    trip = trips.find((t) => {
      const hay = `${t.data?.title || ""} ${t.data?.destination || ""} ${t.data?.classes || ""}`.toLowerCase();
      return hay.includes(query);
    });
  } else {
    return { ok: false, error: "Indiquez tripId ou query (titre / destination)." };
  }

  if (!trip) {
    return { ok: false, error: "Séjour introuvable." };
  }

  const brief = tripBrief(trip);
  return {
    ok: true,
    data: brief,
    summaryFr: `« ${brief.title} » — ${brief.dates || "dates non renseignées"} — statut : ${brief.statusLabel}.`,
  };
}

export async function handleCreateTrip(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const title = String(args.title || "").trim();
  const destination = String(args.destination || "").trim();
  const date = String(args.date || args.startDate || "").trim();
  const startDate = String(args.startDate || date || "").trim();
  const endDate = String(args.endDate || startDate || "").trim();
  const classes = String(args.classes || "").trim();
  const etablissement = String(args.etablissement || "").trim();
  const nbEleves = args.nbEleves != null ? Number(args.nbEleves) : undefined;
  const type = String(args.type || "SIMPLE").toUpperCase() === "COMPLEX" ? "COMPLEX" : "SIMPLE";

  if (!title) return { ok: false, error: "Le titre du séjour est obligatoire." };
  if (!startDate || !/^\d{4}-\d{2}-\d{2}$/.test(startDate)) {
    return { ok: false, error: "Une date de départ (YYYY-MM-DD) est obligatoire." };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_trip",
      args: {
        title,
        destination,
        date: startDate,
        startDate,
        endDate: endDate || startDate,
        classes,
        etablissement,
        nbEleves,
        type,
      },
      summaryFr:
        `Créer le séjour « ${title} »` +
        (destination ? ` à ${destination}` : "") +
        ` le ${startDate}` +
        (classes ? ` (${classes})` : "") +
        ` ?`,
    };
  }

  const now = new Date().toISOString();
  const id = crypto.randomUUID();
  const trip: TravelsTrip = {
    id,
    type,
    status: "EN_ATTENTE_DIR_INITIAL",
    ownerId: ctx.userId || undefined,
    ownerName: [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || undefined,
    ownerEmail: ctx.email || undefined,
    createdAt: now,
    updatedAt: now,
    data: {
      title,
      destination: destination || undefined,
      date: startDate,
      startDate,
      endDate: endDate || startDate,
      classes: classes || undefined,
      etablissement: etablissement || undefined,
      nbEleves: Number.isFinite(nbEleves) ? nbEleves : undefined,
    },
    history: [
      {
        date: now,
        user: [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || "Assistant IA",
        action: "CREE",
        note: "Créé via ScolIA",
      },
    ],
  };

  await putJson(`travels/${id}.json`, trip);
  const indexHit = await getJson<unknown[]>("travels/index.json");
  const currentIndex = Array.isArray(indexHit?.data) ? [...indexHit.data] : [];
  const summary = {
    id: trip.id,
    type: trip.type,
    status: trip.status,
    ownerName: trip.ownerName,
    ownerEmail: trip.ownerEmail,
    ownerId: trip.ownerId,
    createdAt: trip.createdAt,
    updatedAt: trip.updatedAt,
    data: {
      title: trip.data.title,
      destination: trip.data.destination,
      date: trip.data.date,
      startDate: trip.data.startDate,
      endDate: trip.data.endDate,
      classes: trip.data.classes,
      etablissement: trip.data.etablissement,
      nbEleves: trip.data.nbEleves,
    },
  };
  currentIndex.unshift(summary);
  await putJson("travels/index.json", currentIndex);

  void syncTripActualite({
    id,
    title,
    dates: startDate === endDate ? startDate : `${startDate} → ${endDate}`,
    classes,
    statusLabel: TRAVELS_STATUS_LABELS.EN_ATTENTE_DIR_INITIAL,
  });

  return {
    ok: true,
    data: {
      id,
      followUrl: `/travels/${id}`,
      status: trip.status,
      statusLabel: TRAVELS_STATUS_LABELS.EN_ATTENTE_DIR_INITIAL,
    },
    summaryFr: `Séjour « ${title} » créé. Complétez le dossier sur /travels/${id}.`,
  };
}
