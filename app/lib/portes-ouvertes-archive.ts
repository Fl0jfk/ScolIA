import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  portesOuvertesArchive,
  portesOuvertesConfig,
  portesOuvertesRegistration,
  portesOuvertesSlot,
  portesOuvertesSlotStaff,
} from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

const TZ = "Europe/Paris";

type WallParts = { y: number; m: number; d: number; h: number; mi: number };

function parisParts(ms: number): WallParts {
  const f = new Intl.DateTimeFormat("en-GB", {
    timeZone: TZ,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  });
  const get = (t: string) => Number(f.formatToParts(new Date(ms)).find((p) => p.type === t)?.value);
  return { y: get("year"), m: get("month"), d: get("day"), h: get("hour"), mi: get("minute") };
}

/** Heure murale Paris → instant (gère l'heure d'été / d'hiver). */
function parisWallToMs(p: WallParts): number {
  const guess = Date.UTC(p.y, p.m - 1, p.d, p.h, p.mi);
  const off1 = Date.UTC(...wallArgs(parisParts(guess))) - guess;
  let ms = guess - off1;
  const off2 = Date.UTC(...wallArgs(parisParts(ms))) - ms;
  if (off2 !== off1) ms = guess - off2;
  return ms;
}

function wallArgs(p: WallParts): [number, number, number, number, number] {
  return [p.y, p.m - 1, p.d, p.h, p.mi];
}

/** Jour calendaire Paris (en jours depuis l'époque) d'un instant. */
function parisDayNumber(ms: number): number {
  const p = parisParts(ms);
  return Math.round(Date.UTC(p.y, p.m - 1, p.d) / 86_400_000);
}

/** Décale un instant de N jours calendaires en gardant la même heure à Paris. */
export function shiftParisDays(ms: number, days: number): number {
  const p = parisParts(ms);
  const base = new Date(Date.UTC(p.y, p.m - 1, p.d + days));
  return parisWallToMs({
    y: base.getUTCFullYear(),
    m: base.getUTCMonth() + 1,
    d: base.getUTCDate(),
    h: p.h,
    mi: p.mi,
  });
}

export type PortesOuvertesArchiveSummary = {
  id: string;
  title: string;
  sessionStartAt: string | null;
  sessionEndAt: string | null;
  registrationsCount: number;
  staffCount: number;
  archivedAt: string;
  archivedByName: string | null;
};

async function requireEtabId(): Promise<string> {
  const id = await resolveCurrentEtablissementId();
  if (!id) throw new Error("Établissement introuvable (tenant).");
  return id;
}

export async function listPortesOuvertesArchives(): Promise<PortesOuvertesArchiveSummary[]> {
  const etabId = await requireEtabId();
  const rows = await getDb()
    .select({
      id: portesOuvertesArchive.id,
      title: portesOuvertesArchive.title,
      sessionStartAt: portesOuvertesArchive.sessionStartAt,
      sessionEndAt: portesOuvertesArchive.sessionEndAt,
      registrationsCount: portesOuvertesArchive.registrationsCount,
      staffCount: portesOuvertesArchive.staffCount,
      archivedAt: portesOuvertesArchive.archivedAt,
      archivedByName: portesOuvertesArchive.archivedByName,
    })
    .from(portesOuvertesArchive)
    .where(eq(portesOuvertesArchive.etablissementId, etabId))
    .orderBy(desc(portesOuvertesArchive.archivedAt));
  return rows.map((r) => ({
    ...r,
    sessionStartAt: r.sessionStartAt?.toISOString() ?? null,
    sessionEndAt: r.sessionEndAt?.toISOString() ?? null,
    archivedAt: r.archivedAt.toISOString(),
  }));
}

export async function getPortesOuvertesArchive(id: string) {
  const etabId = await requireEtabId();
  const rows = await getDb()
    .select()
    .from(portesOuvertesArchive)
    .where(and(eq(portesOuvertesArchive.etablissementId, etabId), eq(portesOuvertesArchive.id, id)))
    .limit(1);
  return rows[0] ?? null;
}

export type ArchiveAndResetResult = {
  archiveId: string;
  registrationsArchived: number;
  staffArchived: number;
  slotsShifted: number;
  dayShift: number;
};

/**
 * Archive la session en cours (inscrits + équipes + photo des créneaux), puis la remet à zéro :
 * - les créneaux (heures, cycle, libellé, places max) sont conservés, décalés sur la nouvelle date ;
 * - les inscriptions et les équipes sont supprimées de la session active (elles restent dans l'archive).
 * Tout se passe dans une seule transaction : tout ou rien.
 */
export async function archiveAndResetPortesOuvertes(params: {
  /** Nouvelle date du premier jour (AAAA-MM-JJ, heure de Paris). */
  newFirstDay: string;
  actor: { userId: string; name: string };
}): Promise<ArchiveAndResetResult> {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(params.newFirstDay);
  if (!m) throw new Error("Date invalide.");
  const target = Math.round(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3])) / 86_400_000);

  const etabId = await requireEtabId();
  const db = getDb();

  return db.transaction(async (tx) => {
    const slots = await tx
      .select()
      .from(portesOuvertesSlot)
      .where(eq(portesOuvertesSlot.etablissementId, etabId));
    if (slots.length === 0) throw new Error("Aucun créneau à décaler.");
    const registrations = await tx
      .select()
      .from(portesOuvertesRegistration)
      .where(eq(portesOuvertesRegistration.etablissementId, etabId));
    const staff = await tx
      .select()
      .from(portesOuvertesSlotStaff)
      .where(eq(portesOuvertesSlotStaff.etablissementId, etabId));
    const cfg = await tx
      .select({ title: portesOuvertesConfig.title })
      .from(portesOuvertesConfig)
      .where(eq(portesOuvertesConfig.etablissementId, etabId))
      .limit(1);

    const firstStart = Math.min(...slots.map((s) => s.startAt.getTime()));
    const lastEnd = Math.max(...slots.map((s) => s.endAt.getTime()));
    const dayShift = target - parisDayNumber(firstStart);

    const [archive] = await tx
      .insert(portesOuvertesArchive)
      .values({
        etablissementId: etabId,
        title: cfg[0]?.title ?? "Portes ouvertes",
        sessionStartAt: new Date(firstStart),
        sessionEndAt: new Date(lastEnd),
        registrationsCount: registrations.length,
        staffCount: staff.length,
        slots: JSON.parse(JSON.stringify(slots)),
        registrations: JSON.parse(JSON.stringify(registrations)),
        staff: JSON.parse(JSON.stringify(staff)),
        archivedByUserId: params.actor.userId,
        archivedByName: params.actor.name,
      })
      .returning({ id: portesOuvertesArchive.id });

    await tx
      .delete(portesOuvertesRegistration)
      .where(eq(portesOuvertesRegistration.etablissementId, etabId));
    await tx
      .delete(portesOuvertesSlotStaff)
      .where(eq(portesOuvertesSlotStaff.etablissementId, etabId));

    for (const s of slots) {
      await tx
        .update(portesOuvertesSlot)
        .set({
          startAt: new Date(shiftParisDays(s.startAt.getTime(), dayShift)),
          endAt: new Date(shiftParisDays(s.endAt.getTime(), dayShift)),
          updatedAt: new Date(),
        })
        .where(
          and(eq(portesOuvertesSlot.etablissementId, etabId), eq(portesOuvertesSlot.id, s.id)),
        );
    }

    return {
      archiveId: archive.id,
      registrationsArchived: registrations.length,
      staffArchived: staff.length,
      slotsShifted: slots.length,
      dayShift,
    };
  });
}
