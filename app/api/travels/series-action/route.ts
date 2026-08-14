import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { isTripOwner } from "@/app/lib/travels-direction-permissions";
import { notifyComptaTravelsPhase, type TravelsTripForNotify } from "@/app/lib/travels-notify";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { canSignTravelsDirectionForEtab } from "@/app/lib/establishments";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function tripSummaryFromFullTrip(tripId: string, t: any) {
  const inner = t.data || {};
  return {
    id: tripId,
    ownerName: t.ownerName,
    status: t.status,
    type: t.type,
    createdAt: t.createdAt || new Date().toISOString(),
    data: {
      title: inner.title || "Titre introuvable",
      destination: inner.destination || "Destination introuvable",
      imageUrl: t.imageUrl,
      nbEleves: inner.nbEleves,
      nbAccompagnateurs: inner.nbAccompagnateurs,
      nomsAccompagnateurs: inner.nomsAccompagnateurs || [],
      classes: inner.classes || [],
      piqueNique: inner.piqueNique || false,
      piqueNiqueDetails: inner.piqueNiqueDetails || null,
      date: inner.date || null,
      startDate: inner.startDate || inner.date || null,
      endDate: inner.endDate || inner.date || null,
      startTime: inner.startTime || null,
      endTime: inner.endTime || null,
      needsBus: inner.needsBus || false,
      transportRequest: inner.transportRequest || null,
      objectifs: inner.objectifs || inner.pedagogicalObjectives || null,
      coutTotal: inner.coutTotal,
      etablissement: inner.etablissement || null,
      recurrenceSeriesId: inner.recurrenceSeriesId || null,
      recurrenceIndex: inner.recurrenceIndex ?? null,
      recurrenceTotal: inner.recurrenceTotal ?? null,
    },
  };
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
    let body: { action?: string; seriesId?: string; tripId?: string; actorName?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide" }, { status: 400 });
  }
  const actor =
    typeof body.actorName === "string" && body.actorName.trim() ? body.actorName.trim() : "Utilisateur";

  try {
    const indexHit = await getJson<unknown[]>("travels/index.json");
    let currentIndex: unknown[] = Array.isArray(indexHit?.data) ? indexHit.data : [];

    const syncIndexEntry = (tripId: string, summary: ReturnType<typeof tripSummaryFromFullTrip>) => {
      const idx = currentIndex.findIndex((e: unknown) => (e as { id?: string }).id === tripId);
      if (idx > -1) (currentIndex as unknown[])[idx] = summary;
    };

    if (body.action === "validate_pedagogy_series" && body.seriesId) {
      const seriesId = body.seriesId;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const rows = currentIndex as any[];
      const ids: string[] = [];
      for (const row of rows) {
        if (row.type !== "SIMPLE" || row.status !== "EN_ATTENTE_DIR_INITIAL" || !row.id) continue;
        const data = row.data || {};
        let sid: string | null | undefined = data.recurrenceSeriesId;
        if (sid === undefined && !("recurrenceSeriesId" in data)) {
          const fullHit = await getJson<{ data?: { recurrenceSeriesId?: string } }>(
            `travels/${row.id}.json`,
          );
          sid = fullHit?.data?.data?.recurrenceSeriesId;
          if (!fullHit) continue;
        }
        if (sid === seriesId) ids.push(row.id);
      }
      if (ids.length === 0) {
        return NextResponse.json({ error: "Aucun dossier de la série en attente pédagogie.", updated: 0 }, { status: 404 });
      }
      const me = await safeCurrentUser();
      if (!me?.id) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
      const firstHit = await getJson<{ data?: { etablissement?: string } }>( `travels/${ids[0]}.json`);
      if (!firstHit?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
      if (!(await canSignTravelsDirectionForEtab(me,  firstHit.data?.data?.etablissement))) {
        return NextResponse.json({ error: "Droits insuffisants" }, { status: 403 });
      }
      const now = new Date().toISOString();
      let updated = 0;
      for (const tripId of ids) {
        const hit = await getJson<Record<string, unknown>>(`travels/${tripId}.json`);
        if (!hit?.data) continue;
        const trip = hit.data as TravelsTripForNotify & {
          type?: string;
          status?: string;
          updatedAt?: string;
          data?: TravelsTripForNotify["data"] & { recurrenceSeriesId?: string };
          history?: unknown[];
        };
        if (trip.type !== "SIMPLE" || trip.status !== "EN_ATTENTE_DIR_INITIAL") continue;
        if (trip.data?.recurrenceSeriesId !== seriesId) continue;
        const statusBefore: string = String(trip.status);
        trip.status = "EN_ATTENTE_COMPTA";
        trip.updatedAt = now;
        trip.history = [
          ...(Array.isArray(trip.history) ? trip.history : []),
          {
            date: now,
            action: "EN_ATTENTE_COMPTA",
            note: "Pédagogie validée (toute la série)",
            user: actor,
          },
        ];
        await putJson(`travels/${tripId}.json`, trip);
        syncIndexEntry(tripId, tripSummaryFromFullTrip(tripId, trip));
        if (statusBefore !== "EN_ATTENTE_COMPTA") {
          try {
            await notifyComptaTravelsPhase({ tripId,
              trip,
              previousStatus: statusBefore,
            });
          } catch (mailErr) {
            console.error("[series-action] notification compta:", mailErr);
          }
        }
        updated++;
      }
      await putJson("travels/index.json", currentIndex);
      return NextResponse.json({ success: true, updated });
    }

    if (body.action === "cancel_session" && body.tripId) {
      const tripId = body.tripId;
      const hit = await getJson<Record<string, unknown>>(`travels/${tripId}.json`);
      if (!hit?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
      const trip = hit.data as Record<string, unknown> & {
        ownerId?: string;
        status?: string;
        data?: { recurrenceSeriesId?: string; etablissement?: string };
        history?: unknown[];
      };
      const me = await safeCurrentUser();
      if (!me?.id) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
      if (
        !isTripOwner(trip.ownerId as string, me.id) &&
        !(await canSignTravelsDirectionForEtab(me,  trip.data?.etablissement))
      ) {
        return NextResponse.json({ error: "Non autorisé" }, { status: 403 });
      }
      if (!trip.data?.recurrenceSeriesId) {
        return NextResponse.json({ error: "Ce dossier ne fait pas partie d'une série récurrente." }, { status: 400 });
      }
      if (trip.status === "SEANCE_ANNULEE") {
        return NextResponse.json({ error: "Séance déjà annulée." }, { status: 400 });
      }
      if (trip.status === "VALIDE" || trip.status === "REJETE") {
        return NextResponse.json({ error: "Impossible d'annuler ce dossier à ce stade." }, { status: 400 });
      }
      const now = new Date().toISOString();
      trip.status = "SEANCE_ANNULEE";
      trip.updatedAt = now;
      trip.history = [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          action: "SEANCE_ANNULEE",
          note: "Séance annulée (reste de la série inchangé)",
          user: actor,
        },
      ];
      await putJson(`travels/${tripId}.json`, trip);
      syncIndexEntry(tripId, tripSummaryFromFullTrip(tripId, trip));
      await putJson("travels/index.json", currentIndex);
      return NextResponse.json({ success: true });
    }

    return NextResponse.json({ error: "Action inconnue" }, { status: 400 });
  } catch (e) {
    console.error("series-action:", e);
    return NextResponse.json({ error: "Échec" }, { status: 500 });
  }
}
