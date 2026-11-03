import "server-only";

import {
  findActiveBookingByGoogleEvent,
  getRdvInscriptionDirectionById,
} from "@/app/lib/rdv-inscription-db";
import {
  clearOrphanInscriptionMarkers,
  listHiddenInscriptionSlots,
  type HiddenInscriptionSlot,
} from "@/app/lib/rdv-inscription-gcal";

/**
 * Créneau masqué de la liste publique par une marque ScolIA.
 * `repairable` : aucune réservation active ne l'utilise → la marque est orpheline
 * (laissée par une ancienne libération qui n'effaçait pas les marques Google).
 */
export type HiddenSlotReport = HiddenInscriptionSlot & {
  repairable: boolean;
  activeBooking: { id: string; studentLabel: string; status: string } | null;
};

type DirectionCtx = { calendarId: string; titlePattern: string; horizonDays: number };

async function loadDirection(
  directionId: string,
): Promise<{ ok: true; dir: DirectionCtx } | { ok: false; status: number; error: string }> {
  const id = directionId.trim();
  if (!id) return { ok: false, status: 400, error: "directionId requis." };
  const dir = await getRdvInscriptionDirectionById(id);
  if (!dir) return { ok: false, status: 404, error: "Direction introuvable." };
  if (!dir.googleCalendarId.trim()) {
    return { ok: false, status: 400, error: "Agenda de cette direction non configuré." };
  }
  return {
    ok: true,
    dir: {
      calendarId: dir.googleCalendarId.trim(),
      titlePattern: dir.eventTitlePattern,
      horizonDays: dir.horizonDays,
    },
  };
}

/** Lecture seule : liste les créneaux masqués et dit lesquels sont réparables. */
export async function scanHiddenInscriptionSlots(directionId: string): Promise<
  { ok: true; slots: HiddenSlotReport[] } | { ok: false; status: number; error: string }
> {
  const loaded = await loadDirection(directionId);
  if (!loaded.ok) return loaded;
  const { dir } = loaded;

  const hidden = await listHiddenInscriptionSlots(dir);
  const slots: HiddenSlotReport[] = [];
  for (const h of hidden) {
    const active = await findActiveBookingByGoogleEvent({
      calendarId: dir.calendarId,
      eventId: h.eventId,
    });
    slots.push({
      ...h,
      repairable: !active,
      activeBooking: active
        ? {
            id: active.id,
            studentLabel: `${active.studentFirstName} ${active.studentLastName}`.trim(),
            status: active.status,
          }
        : null,
    });
  }
  return { ok: true, slots };
}

/**
 * Efface les marques orphelines des créneaux demandés.
 * Chaque créneau est revérifié : refusé s'il a une réservation active.
 */
export async function repairHiddenInscriptionSlots(opts: {
  directionId: string;
  eventIds: string[];
}): Promise<
  | {
      ok: true;
      repaired: string[];
      skipped: Array<{ eventId: string; reason: string }>;
    }
  | { ok: false; status: number; error: string }
> {
  const loaded = await loadDirection(opts.directionId);
  if (!loaded.ok) return loaded;
  const { dir } = loaded;

  const eventIds = [...new Set(opts.eventIds.map((e) => e.trim()).filter(Boolean))].slice(0, 100);
  if (!eventIds.length) return { ok: false, status: 400, error: "Aucun créneau à réparer." };

  const repaired: string[] = [];
  const skipped: Array<{ eventId: string; reason: string }> = [];
  for (const eventId of eventIds) {
    const active = await findActiveBookingByGoogleEvent({ calendarId: dir.calendarId, eventId });
    if (active) {
      skipped.push({ eventId, reason: "Réservation active sur ce créneau." });
      continue;
    }
    try {
      const res = await clearOrphanInscriptionMarkers({
        calendarId: dir.calendarId,
        eventId,
        titlePattern: dir.titlePattern,
      });
      if (!res.ok) {
        skipped.push({ eventId, reason: res.message });
      } else if (res.changed) {
        repaired.push(eventId);
        console.info("[rdv-inscription] marque orpheline effacée", { eventId });
      } else {
        skipped.push({ eventId, reason: "Créneau déjà libre." });
      }
    } catch (e) {
      skipped.push({ eventId, reason: e instanceof Error ? e.message : String(e) });
    }
  }
  return { ok: true, repaired, skipped };
}
