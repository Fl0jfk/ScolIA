import "server-only";

import { and, eq, inArray } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  travel,
  travelAttr,
  travelHistory,
  travelMessage,
  travelParticipant,
} from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  normalizeParticipantIneKey,
  resolveEleveIdsByIneKeys,
} from "@/app/lib/travel-participant-resolve";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { chunkArray } from "@/app/lib/db-in-chunks";
import {
  listTravelParticipantsForTripIds,
  queryTravelParticipants,
  type TravelParticipantReadRow,
} from "@/app/lib/travel-db-participant-read";

type TravelMain = typeof travel.$inferSelect;
type TravelAttrRow = typeof travelAttr.$inferSelect;
type TravelParticipantRow = typeof travelParticipant.$inferSelect;
type TravelParticipantAssemblyRow = TravelParticipantReadRow | TravelParticipantRow;
type TravelHistoryRow = typeof travelHistory.$inferSelect;
type TravelMessageRow = typeof travelMessage.$inferSelect;

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

/** Postgres travel table — aligné sur la liste élèves (pas de garde ENT_CORE_DB). */
export async function travelsDbReady(): Promise<string | null> {
  if (!isDatabaseConfigured()) return null;
  const fromTenant = await resolveCurrentEtablissementId();
  if (fromTenant) return fromTenant;
  try {
    const { getAppSession } = await import("@/app/lib/app-session");
    const session = await getAppSession();
    const id = session?.user?.etablissementId?.trim();
    return id || null;
  } catch {
    return null;
  }
}

/**
 * Liste complète des séjours.
 * Important perf : 1 requête mains + 4 requêtes enfants en batch (pas de N+1).
 * Avec Postgres distant, l’ancien hydrate séquentiel par dossier coûtait souvent 10–30s.
 */
export type ListTravelsFromDbOptions = {
  /** Liste module / rappels : pas besoin de messages ni historique (perf + taille JSON). */
  forListIndex?: boolean;
};

export async function listTravelsFromDb(
  etablissementId: string,
  opts?: ListTravelsFromDbOptions,
): Promise<TravelsTrip[]> {
  const db = getDb();
  const mains = await db.select().from(travel).where(eq(travel.etablissementId, etablissementId));
  if (mains.length === 0) return [];

  const ids = mains.map((m) => m.id);
  const skipHeavy = opts?.forListIndex === true;

  const allAttrs: TravelAttrRow[] = [];
  for (const batch of chunkArray(ids)) {
    const part = await db
      .select()
      .from(travelAttr)
      .where(
        and(eq(travelAttr.etablissementId, etablissementId), inArray(travelAttr.travelId, batch)),
      );
    allAttrs.push(...part);
  }

  const participantsPromise = listTravelParticipantsForTripIds(db, etablissementId, ids);

  const [allParticipants, allHistory, allMessages] = await Promise.all([
    participantsPromise,
    skipHeavy
      ? Promise.resolve([] as TravelHistoryRow[])
      : db
          .select()
          .from(travelHistory)
          .where(
            and(
              eq(travelHistory.etablissementId, etablissementId),
              inArray(travelHistory.travelId, ids),
            ),
          ),
    skipHeavy
      ? Promise.resolve([] as TravelMessageRow[])
      : db
          .select()
          .from(travelMessage)
          .where(
            and(
              eq(travelMessage.etablissementId, etablissementId),
              inArray(travelMessage.travelId, ids),
            ),
          ),
  ]);

  const attrsByTrip = groupByTravelId(allAttrs);
  const participantsByTrip = groupByTravelId(allParticipants);
  const historyByTrip = groupByTravelId(allHistory);
  const messagesByTrip = groupByTravelId(allMessages);

  const trips: TravelsTrip[] = [];
  for (const m of mains) {
    try {
      trips.push(
        assembleTravel(m, {
          attrs: attrsByTrip.get(m.id) ?? [],
          participants: participantsByTrip.get(m.id) ?? [],
          history: historyByTrip.get(m.id) ?? [],
          messages: messagesByTrip.get(m.id) ?? [],
        }),
      );
    } catch (assembleErr) {
      console.error("[travel-db] assembleTravel list", m.id, assembleErr);
    }
  }
  return trips;
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

function groupByTravelId<T extends { travelId: string }>(rows: T[]): Map<string, T[]> {
  const map = new Map<string, T[]>();
  for (const row of rows) {
    const list = map.get(row.travelId);
    if (list) list.push(row);
    else map.set(row.travelId, [row]);
  }
  return map;
}

function assembleTravel(
  m: TravelMain,
  parts: {
    attrs: TravelAttrRow[];
    participants: TravelParticipantAssemblyRow[];
    history: TravelHistoryRow[];
    messages: TravelMessageRow[];
  },
): TravelsTrip {
  const inflated = inflateFromAttrs(parts.attrs.map((a) => ({ path: a.path, value: a.value })));
  const dataFromAttrs =
    inflated.data && typeof inflated.data === "object"
      ? (inflated.data as Record<string, unknown>)
      : {};
  const rootExtras = { ...inflated };
  delete rootExtras.data;

  const participantEleves = [...parts.participants]
    .sort((a, b) => a.sortOrder - b.sortOrder)
    .map((p) => ({
      ine: p.eleveKey,
      nom: p.nom,
      prenom: p.prenom,
      droitImageOk: p.droitImageOk !== false,
      panierRepas: p.panierRepas === true,
      ...(p.classe ? { classe: p.classe } : {}),
      ...("eleveId" in p && p.eleveId ? { eleveId: p.eleveId } : {}),
    }));

  const attrStart =
    typeof dataFromAttrs.startDate === "string" ? dataFromAttrs.startDate : undefined;
  const attrEnd = typeof dataFromAttrs.endDate === "string" ? dataFromAttrs.endDate : undefined;
  const attrDate = typeof dataFromAttrs.date === "string" ? dataFromAttrs.date : undefined;

  const data = {
    ...dataFromAttrs,
    title: m.title ?? undefined,
    destination: m.destination ?? undefined,
    etablissement: m.siteLabel ?? undefined,
    classes: m.classes ?? undefined,
    startDate: m.startDate ?? attrStart ?? attrDate ?? undefined,
    endDate: m.endDate ?? attrEnd ?? undefined,
    date: attrDate ?? m.startDate ?? undefined,
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
    ...(typeof rootExtras.imageAttribution === "string"
      ? { imageAttribution: rootExtras.imageAttribution }
      : {}),
    ...(typeof rootExtras.imageAuthor === "string" || rootExtras.imageAuthor === null
      ? { imageAuthor: rootExtras.imageAuthor as string | null }
      : {}),
    ...(typeof rootExtras.imageLicense === "string" || rootExtras.imageLicense === null
      ? { imageLicense: rootExtras.imageLicense as string | null }
      : {}),
    ...(typeof rootExtras.imageAttributionUrl === "string" ||
    rootExtras.imageAttributionUrl === null
      ? { imageAttributionUrl: rootExtras.imageAttributionUrl as string | null }
      : {}),
    data,
    history: [...parts.history]
      .sort((a, b) => a.sortOrder - b.sortOrder)
      .map((h) => ({
        date: String(h.at ?? ""),
        user: h.by,
        action: h.action,
        ...(h.note ? { note: h.note } : {}),
      })),
    messages: [...parts.messages]
      .sort((a, b) => a.sortOrder - b.sortOrder)
      .map((msg) => ({
        id: msg.id,
        user: msg.userLabel,
        role: msg.role,
        text: msg.body,
        date: String(msg.at ?? ""),
      })),
    ...(rootExtras.receivedDevis
      ? { receivedDevis: rootExtras.receivedDevis as TravelsTrip["receivedDevis"] }
      : {}),
  };
}

async function hydrateTravel(etablissementId: string, m: TravelMain): Promise<TravelsTrip> {
  const db = getDb();
  const [attrs, participants, history, messages] = await Promise.all([
    db
      .select()
      .from(travelAttr)
      .where(and(eq(travelAttr.etablissementId, etablissementId), eq(travelAttr.travelId, m.id))),
    queryTravelParticipants(
      db,
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

  return assembleTravel(m, { attrs, participants, history, messages });
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

  await syncTravelParticipants({
    etablissementId,
    travelId: main.id,
    eleves: Array.isArray(data.participantEleves) ? data.participantEleves : [],
  });

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

  const msgs = Array.isArray(trip.messages) ? trip.messages : [];
  if (msgs.length > 0) {
    for (let i = 0; i < msgs.length; i++) {
      const msg = msgs[i]!;
      const row = {
        id: String(msg.id || `${main.id}_msg_${i}`),
        etablissementId,
        travelId: main.id,
        userLabel: String(msg.user ?? ""),
        role: String(msg.role ?? ""),
        body: String(msg.text ?? ""),
        at: String(msg.date ?? ""),
        sortOrder: i,
      };
      await db
        .insert(travelMessage)
        .values(row)
        .onConflictDoUpdate({
          target: travelMessage.id,
          set: {
            userLabel: row.userLabel,
            role: row.role,
            body: row.body,
            at: row.at,
            sortOrder: row.sortOrder,
          },
        });
    }
  }

  void import("@/app/lib/valkey")
    .then(async ({ valkeyDel }) => {
      const { valkeyKeyTravelsIndex, valkeyKeyTravelTrip } = await import(
        "@/app/lib/valkey-keys"
      );
      await valkeyDel(
        valkeyKeyTravelsIndex(etablissementId),
        valkeyKeyTravelTrip(etablissementId, main.id),
      );
    })
    .catch(() => undefined);
}

/**
 * Sync participants d'un voyage : upsert unitaire par clé INE, DELETE ciblé des absents.
 * Remplit `eleve_id` via matching INE strict (orphelins restent null).
 */
async function syncTravelParticipants(opts: {
  etablissementId: string;
  travelId: string;
  eleves: Array<{
    ine?: string | null;
    nom?: string | null;
    prenom?: string | null;
    classe?: string | null;
    droitImageOk?: boolean;
    panierRepas?: boolean;
    eleveId?: string | null;
  }>;
}): Promise<void> {
  const db = getDb();
  const { etablissementId, travelId, eleves } = opts;

  const existing = await db
    .select()
    .from(travelParticipant)
    .where(
      and(
        eq(travelParticipant.etablissementId, etablissementId),
        eq(travelParticipant.travelId, travelId),
      ),
    );

  const byKey = new Map<string, (typeof existing)[number]>();
  for (const row of existing) {
    const key = normalizeParticipantIneKey(row.eleveKey);
    if (key && !byKey.has(key)) byKey.set(key, row);
  }

  const resolved = await resolveEleveIdsByIneKeys({
    etablissementId,
    keys: eleves.map((p) => String(p.ine ?? "")),
  });

  const keptIds = new Set<string>();
  const toInsert: Array<typeof travelParticipant.$inferInsert> = [];

  for (let i = 0; i < eleves.length; i++) {
    const p = eleves[i]!;
    const eleveKey = String(p.ine ?? "");
    const keyNorm = normalizeParticipantIneKey(eleveKey);
    const eleveId =
      (p.eleveId && String(p.eleveId).trim()) ||
      (keyNorm ? resolved.get(keyNorm) : undefined) ||
      null;
    const values = {
      eleveKey,
      eleveId,
      nom: String(p.nom ?? ""),
      prenom: String(p.prenom ?? ""),
      classe: p.classe ? String(p.classe) : null,
      droitImageOk: p.droitImageOk !== false,
      panierRepas: p.panierRepas === true,
      sortOrder: i,
    };

    const prev = keyNorm ? byKey.get(keyNorm) : undefined;
    if (prev) {
      keptIds.add(prev.id);
      await db
        .update(travelParticipant)
        .set(values)
        .where(
          and(
            eq(travelParticipant.etablissementId, etablissementId),
            eq(travelParticipant.id, prev.id),
          ),
        );
    } else {
      toInsert.push({
        etablissementId,
        travelId,
        ...values,
      });
    }
  }

  const orphanIds = existing.filter((row) => !keptIds.has(row.id)).map((row) => row.id);
  if (orphanIds.length > 0) {
    await db
      .delete(travelParticipant)
      .where(
        and(
          eq(travelParticipant.etablissementId, etablissementId),
          eq(travelParticipant.travelId, travelId),
          inArray(travelParticipant.id, orphanIds),
        ),
      );
  }

  if (toInsert.length > 0) {
    await db.insert(travelParticipant).values(toInsert);
  }
}

/** Remplacement complet — script de migration uniquement (efface les dossiers absents de la liste). */
export async function replaceTravelsInDb(
  etablissementId: string,
  trips: TravelsTrip[],
): Promise<number> {
  const db = getDb();
  await db.delete(travel).where(eq(travel.etablissementId, etablissementId));
  for (const t of trips) await upsertTravelInDb(etablissementId, t);
  void import("@/app/lib/valkey")
    .then(async ({ valkeyDel }) => {
      const { valkeyKeyTravelsIndex } = await import("@/app/lib/valkey-keys");
      await valkeyDel(valkeyKeyTravelsIndex(etablissementId));
    })
    .catch(() => undefined);
  return trips.length;
}

export async function deleteTravelFromDb(etablissementId: string, id: string): Promise<void> {
  const db = getDb();
  await db
    .delete(travel)
    .where(and(eq(travel.etablissementId, etablissementId), eq(travel.id, id)));
  void import("@/app/lib/valkey")
    .then(async ({ valkeyDel }) => {
      const { valkeyKeyTravelsIndex, valkeyKeyTravelTrip } = await import(
        "@/app/lib/valkey-keys"
      );
      await valkeyDel(
        valkeyKeyTravelsIndex(etablissementId),
        valkeyKeyTravelTrip(etablissementId, id),
      );
    })
    .catch(() => undefined);
}
