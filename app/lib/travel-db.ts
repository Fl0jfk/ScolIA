import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  travel,
  travelAttr,
  travelHistory,
  travelMessage,
  travelParticipant,
} from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import type { TravelsTrip } from "@/app/lib/travels-types";

const SKIP_ROOT = new Set([
  "id",
  "type",
  "status",
  "ownerName",
  "ownerEmail",
  "ownerId",
  "createdAt",
  "updatedAt",
  "imageUrl",
  "imageConfigId",
  "history",
  "messages",
  "data",
]);

const SKIP_DATA = new Set([
  "title",
  "destination",
  "etablissement",
  "classes",
  "startDate",
  "endDate",
  "startTime",
  "endTime",
  "nbEleves",
  "nbAccompagnateurs",
  "listeElevesStatus",
  "participantEleves",
]);

function parseTs(raw: string | undefined | null): Date | null {
  if (!raw) return null;
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? null : d;
}

export async function travelsDbReady(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}

export async function listTravelsFromDb(etablissementId: string): Promise<TravelsTrip[]> {
  const db = getDb();
  const mains = await db.select().from(travel).where(eq(travel.etablissementId, etablissementId));
  const result: TravelsTrip[] = [];
  for (const m of mains) {
    const trip = await hydrateTravel(etablissementId, m);
    result.push(trip);
  }
  return result;
}

export async function getTravelFromDb(
  etablissementId: string,
  id: string,
): Promise<TravelsTrip | null> {
  const db = getDb();
  const [m] = await db
    .select()
    .from(travel)
    .where(and(eq(travel.etablissementId, etablissementId), eq(travel.id, id)))
    .limit(1);
  if (!m) return null;
  return hydrateTravel(etablissementId, m);
}

async function hydrateTravel(
  etablissementId: string,
  m: typeof travel.$inferSelect,
): Promise<TravelsTrip> {
  const db = getDb();
  const [attrs, participants, history, messages] = await Promise.all([
    db
      .select()
      .from(travelAttr)
      .where(and(eq(travelAttr.etablissementId, etablissementId), eq(travelAttr.travelId, m.id))),
    db
      .select()
      .from(travelParticipant)
      .where(
        and(
          eq(travelParticipant.etablissementId, etablissementId),
          eq(travelParticipant.travelId, m.id),
        ),
      ),
    db
      .select()
      .from(travelHistory)
      .where(
        and(eq(travelHistory.etablissementId, etablissementId), eq(travelHistory.travelId, m.id)),
      ),
    db
      .select()
      .from(travelMessage)
      .where(
        and(eq(travelMessage.etablissementId, etablissementId), eq(travelMessage.travelId, m.id)),
      ),
  ]);

  const inflated = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  const dataFromAttrs =
    inflated.data && typeof inflated.data === "object"
      ? (inflated.data as Record<string, unknown>)
      : {};
  const rootExtras = { ...inflated };
  delete rootExtras.data;

  const participantEleves = [...participants]
    .sort((a, b) => a.sortOrder - b.sortOrder)
    .map((p) => ({
      ine: p.eleveKey,
      nom: p.nom,
      prenom: p.prenom,
      droitImageOk: true,
      ...(p.classe ? { classe: p.classe } : {}),
    }));

  const data = {
    ...dataFromAttrs,
    title: m.title ?? undefined,
    destination: m.destination ?? undefined,
    etablissement: m.siteLabel ?? undefined,
    classes: m.classes ?? undefined,
    startDate: m.startDate ?? undefined,
    endDate: m.endDate ?? undefined,
    startTime: m.startTime ?? undefined,
    endTime: m.endTime ?? undefined,
    nbEleves: m.nbEleves ?? undefined,
    nbAccompagnateurs: m.nbAccompagnateurs ?? undefined,
    listeElevesStatus: (m.listeElevesStatus as "draft" | "confirmed" | undefined) ?? undefined,
    participantEleves,
  };

  return {
    id: m.id,
    type: m.type as TravelsTrip["type"],
    status: m.status,
    ownerName: m.ownerName ?? undefined,
    ownerEmail: m.ownerEmail ?? undefined,
    ownerId: m.ownerId ?? undefined,
    createdAt: m.createdAt?.toISOString(),
    updatedAt: m.updatedAt?.toISOString(),
    imageUrl: m.imageUrl ?? undefined,
    imageConfigId: m.imageConfigId ?? undefined,
    data,
    history: [...history]
      .sort((a, b) => a.sortOrder - b.sortOrder)
      .map((h) => ({
        date: h.at,
        user: h.by,
        action: h.action,
        ...(h.note ? { note: h.note } : {}),
      })),
    messages: [...messages]
      .sort((a, b) => a.sortOrder - b.sortOrder)
      .map((msg) => ({
        id: msg.id,
        user: msg.userLabel,
        role: msg.role,
        text: msg.body,
        date: msg.at,
      })),
    ...(rootExtras.receivedDevis
      ? { receivedDevis: rootExtras.receivedDevis as TravelsTrip["receivedDevis"] }
      : {}),
  };
}

export async function upsertTravelInDb(
  etablissementId: string,
  trip: TravelsTrip,
): Promise<void> {
  const db = getDb();
  const data = trip.data ?? {};
  const main = {
    id: String(trip.id),
    etablissementId,
    type: String(trip.type ?? "SIMPLE"),
    status: String(trip.status ?? ""),
    ownerName: trip.ownerName ?? null,
    ownerEmail: trip.ownerEmail ?? null,
    ownerId: trip.ownerId ?? null,
    createdAt: parseTs(trip.createdAt),
    updatedAt: parseTs(trip.updatedAt) ?? new Date(),
    imageUrl: trip.imageUrl ?? null,
    imageConfigId: trip.imageConfigId ?? null,
    title: data.title ? String(data.title) : null,
    destination: data.destination ? String(data.destination) : null,
    siteLabel: data.etablissement ? String(data.etablissement) : null,
    classes: data.classes ? String(data.classes) : null,
    startDate: data.startDate ? String(data.startDate) : null,
    endDate: data.endDate ? String(data.endDate) : null,
    startTime: data.startTime ? String(data.startTime) : null,
    endTime: data.endTime ? String(data.endTime) : null,
    nbEleves: data.nbEleves != null ? String(data.nbEleves) : null,
    nbAccompagnateurs: data.nbAccompagnateurs != null ? String(data.nbAccompagnateurs) : null,
    listeElevesStatus: data.listeElevesStatus ? String(data.listeElevesStatus) : null,
  };

  await db
    .insert(travel)
    .values(main)
    .onConflictDoUpdate({
      target: travel.id,
      set: {
        etablissementId: main.etablissementId,
        type: main.type,
        status: main.status,
        ownerName: main.ownerName,
        ownerEmail: main.ownerEmail,
        ownerId: main.ownerId,
        createdAt: main.createdAt,
        updatedAt: main.updatedAt,
        imageUrl: main.imageUrl,
        imageConfigId: main.imageConfigId,
        title: main.title,
        destination: main.destination,
        siteLabel: main.siteLabel,
        classes: main.classes,
        startDate: main.startDate,
        endDate: main.endDate,
        startTime: main.startTime,
        endTime: main.endTime,
        nbEleves: main.nbEleves,
        nbAccompagnateurs: main.nbAccompagnateurs,
        listeElevesStatus: main.listeElevesStatus,
      },
    });

  const dataRest = { ...data } as Record<string, unknown>;
  for (const k of SKIP_DATA) delete dataRest[k];
  const rootRest: Record<string, unknown> = { data: dataRest };
  if (trip.receivedDevis) rootRest.receivedDevis = trip.receivedDevis;
  for (const [k, v] of Object.entries(trip as unknown as Record<string, unknown>)) {
    if (SKIP_ROOT.has(k)) continue;
    rootRest[k] = v;
  }

  await db
    .delete(travelAttr)
    .where(and(eq(travelAttr.etablissementId, etablissementId), eq(travelAttr.travelId, main.id)));
  const attrs = flattenToAttrs(rootRest);
  if (attrs.length > 0) {
    const chunk = 80;
    for (let i = 0; i < attrs.length; i += chunk) {
      await db.insert(travelAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          travelId: main.id,
          path: a.path,
          value: a.value,
        })),
      );
    }
  }

  await db
    .delete(travelParticipant)
    .where(
      and(
        eq(travelParticipant.etablissementId, etablissementId),
        eq(travelParticipant.travelId, main.id),
      ),
    );
  const eleves = Array.isArray(data.participantEleves) ? data.participantEleves : [];
  if (eleves.length > 0) {
    await db.insert(travelParticipant).values(
      eleves.map((p, i) => ({
        etablissementId,
        travelId: main.id,
        eleveKey: String(p.ine ?? ""),
        nom: String(p.nom ?? ""),
        prenom: String(p.prenom ?? ""),
        classe: p.classe ? String(p.classe) : null,
        sortOrder: i,
      })),
    );
  }

  await db
    .delete(travelHistory)
    .where(
      and(eq(travelHistory.etablissementId, etablissementId), eq(travelHistory.travelId, main.id)),
    );
  const hist = Array.isArray(trip.history) ? trip.history : [];
  if (hist.length > 0) {
    await db.insert(travelHistory).values(
      hist.map((h, i) => ({
        etablissementId,
        travelId: main.id,
        at: String(h.date ?? ""),
        by: String(h.user ?? ""),
        action: String(h.action ?? ""),
        note: h.note ? String(h.note) : null,
        sortOrder: i,
      })),
    );
  }

  await db
    .delete(travelMessage)
    .where(
      and(eq(travelMessage.etablissementId, etablissementId), eq(travelMessage.travelId, main.id)),
    );
  const msgs = Array.isArray(trip.messages) ? trip.messages : [];
  if (msgs.length > 0) {
    await db.insert(travelMessage).values(
      msgs.map((msg, i) => ({
        id: String(msg.id || `${main.id}_msg_${i}`),
        etablissementId,
        travelId: main.id,
        userLabel: String(msg.user ?? ""),
        role: String(msg.role ?? ""),
        body: String(msg.text ?? ""),
        at: String(msg.date ?? ""),
        sortOrder: i,
      })),
    );
  }
}

export async function replaceTravelsInDb(
  etablissementId: string,
  trips: TravelsTrip[],
): Promise<number> {
  const db = getDb();
  await db.delete(travel).where(eq(travel.etablissementId, etablissementId));
  for (const t of trips) await upsertTravelInDb(etablissementId, t);
  return trips.length;
}

export async function deleteTravelFromDb(etablissementId: string, id: string): Promise<void> {
  const db = getDb();
  await db
    .delete(travel)
    .where(and(eq(travel.etablissementId, etablissementId), eq(travel.id, id)));
}
