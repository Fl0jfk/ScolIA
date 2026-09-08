import "server-only";

import { and, asc, count, eq, isNotNull, isNull, lte } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  portesOuvertesConfig,
  portesOuvertesRegistration,
  portesOuvertesSlot,
  portesOuvertesSlotStaff,
} from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import type { PortesOuvertesSlot } from "@/app/lib/toolbox-types";
import type {
  PortesOuvertesCycle,
  PortesOuvertesRegistration,
  PortesOuvertesRegistrationSource,
  PortesOuvertesStaffRole,
  PortesOuvertesStaffRow,
} from "@/app/lib/portes-ouvertes-types";
import { isPortesOuvertesCycle } from "@/app/lib/portes-ouvertes-types";
import {
  PORTES_OUVERTES_MAX_AMBASSADEURS,
  PORTES_OUVERTES_MAX_ENCADRANTS,
} from "@/app/lib/portes-ouvertes-types";

export type { PortesOuvertesStaffRole, PortesOuvertesStaffRow };

export type PortesOuvertesConfigRow = {
  title: string;
  intro: string;
  address: string;
  mapsUrl?: string;
  notifyEmail?: string;
  preinscriptionUrl?: string;
  followUpDelayMinutes: number;
  consentLabel: string;
};

const DEFAULT_CONFIG: PortesOuvertesConfigRow = {
  title: "Portes ouvertes",
  intro: "Inscrivez-vous pour visiter l'établissement et rencontrer l'équipe.",
  address: "",
  mapsUrl: undefined,
  notifyEmail: undefined,
  preinscriptionUrl: undefined,
  followUpDelayMinutes: 60,
  consentLabel:
    "J'accepte que mes coordonnées soient utilisées pour organiser ma visite et me recontacter si besoin.",
};

async function requireEtabId(explicit?: string): Promise<string> {
  const id = explicit || (await resolveCurrentEtablissementId());
  if (!id) throw new Error("Établissement introuvable (tenant).");
  return id;
}

function toIso(d: Date | null | undefined): string | undefined {
  if (!d) return undefined;
  return d.toISOString();
}

function parseStaffRole(v: string): PortesOuvertesStaffRole | null {
  if (v === "ambassadeur" || v === "enseignant" || v === "personnel") return v;
  return null;
}

function mapRegistration(row: typeof portesOuvertesRegistration.$inferSelect): PortesOuvertesRegistration {
  return {
    id: row.id,
    slotId: row.slotId,
    slotLabel: row.slotLabel || undefined,
    slotStartAt: toIso(row.slotStartAt),
    slotEndAt: toIso(row.slotEndAt),
    firstName: row.firstName,
    lastName: row.lastName,
    email: row.email,
    phone: row.phone || undefined,
    childrenInfo: row.childrenInfo || undefined,
    childFirstName: row.childFirstName || undefined,
    childLastName: row.childLastName || undefined,
    cycle: isPortesOuvertesCycle(row.cycle) ? row.cycle : undefined,
    classeSouhaitee: row.classeSouhaitee || undefined,
    consent: row.consent,
    source:
      row.source === "public" || row.source === "accueil"
        ? (row.source as PortesOuvertesRegistrationSource)
        : undefined,
    recordedBy:
      row.recordedByUserId
        ? { userId: row.recordedByUserId, name: row.recordedByName || "Accueil" }
        : undefined,
    lastModifiedBy:
      row.lastModifiedByUserId
        ? {
            userId: row.lastModifiedByUserId,
            name: row.lastModifiedByName || "Accueil",
          }
        : undefined,
    visitedAt: toIso(row.visitedAt),
    followUpDueAt: toIso(row.followUpDueAt),
    followUpEmailSentAt: toIso(row.followUpEmailSentAt),
    createdAt: row.createdAt.toISOString(),
    updatedAt: toIso(row.updatedAt),
  };
}

function mapSlot(row: typeof portesOuvertesSlot.$inferSelect): PortesOuvertesSlot {
  return {
    id: row.id,
    label: row.label,
    startAt: row.startAt.toISOString(),
    endAt: row.endAt.toISOString(),
    maxPlaces: row.maxPlaces ?? undefined,
    cycle: isPortesOuvertesCycle(row.cycle) ? row.cycle : undefined,
  };
}

export async function getPortesOuvertesConfig(
  etablissementId?: string,
): Promise<PortesOuvertesConfigRow> {
  if (!isDatabaseConfigured()) return { ...DEFAULT_CONFIG };
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(portesOuvertesConfig)
    .where(eq(portesOuvertesConfig.etablissementId, etabId))
    .limit(1);
  const row = rows[0];
  if (!row) return { ...DEFAULT_CONFIG };
  return {
    title: row.title,
    intro: row.intro,
    address: row.address,
    mapsUrl: row.mapsUrl || undefined,
    notifyEmail: row.notifyEmail || undefined,
    preinscriptionUrl: row.preinscriptionUrl || undefined,
    followUpDelayMinutes:
      typeof row.followUpDelayMinutes === "number" && row.followUpDelayMinutes > 0
        ? row.followUpDelayMinutes
        : 60,
    consentLabel: row.consentLabel,
  };
}

export async function upsertPortesOuvertesConfig(
  patch: Partial<PortesOuvertesConfigRow>,
  etablissementId?: string,
): Promise<PortesOuvertesConfigRow> {
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const current = await getPortesOuvertesConfig(etabId);
  const next: PortesOuvertesConfigRow = {
    title: patch.title?.trim() || current.title,
    intro: patch.intro ?? current.intro,
    address: patch.address ?? current.address,
    mapsUrl: patch.mapsUrl !== undefined ? patch.mapsUrl || undefined : current.mapsUrl,
    notifyEmail:
      patch.notifyEmail !== undefined ? patch.notifyEmail || undefined : current.notifyEmail,
    preinscriptionUrl:
      patch.preinscriptionUrl !== undefined
        ? patch.preinscriptionUrl || undefined
        : current.preinscriptionUrl,
    followUpDelayMinutes:
      typeof patch.followUpDelayMinutes === "number" && patch.followUpDelayMinutes > 0
        ? Math.min(24 * 60, patch.followUpDelayMinutes)
        : current.followUpDelayMinutes,
    consentLabel: patch.consentLabel?.trim() || current.consentLabel,
  };
  await db
    .insert(portesOuvertesConfig)
    .values({
      etablissementId: etabId,
      title: next.title,
      intro: next.intro,
      address: next.address,
      mapsUrl: next.mapsUrl || null,
      notifyEmail: next.notifyEmail || null,
      preinscriptionUrl: next.preinscriptionUrl || null,
      followUpDelayMinutes: next.followUpDelayMinutes,
      consentLabel: next.consentLabel,
      updatedAt: new Date(),
    })
    .onConflictDoUpdate({
      target: portesOuvertesConfig.etablissementId,
      set: {
        title: next.title,
        intro: next.intro,
        address: next.address,
        mapsUrl: next.mapsUrl || null,
        notifyEmail: next.notifyEmail || null,
        preinscriptionUrl: next.preinscriptionUrl || null,
        followUpDelayMinutes: next.followUpDelayMinutes,
        consentLabel: next.consentLabel,
        updatedAt: new Date(),
      },
    });
  return next;
}

export async function listPortesOuvertesSlots(
  etablissementId?: string,
  cycle?: PortesOuvertesCycle,
): Promise<PortesOuvertesSlot[]> {
  if (!isDatabaseConfigured()) return [];
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = cycle
    ? await db
        .select()
        .from(portesOuvertesSlot)
        .where(
          and(
            eq(portesOuvertesSlot.etablissementId, etabId),
            eq(portesOuvertesSlot.cycle, cycle),
          ),
        )
        .orderBy(asc(portesOuvertesSlot.startAt))
    : await db
        .select()
        .from(portesOuvertesSlot)
        .where(eq(portesOuvertesSlot.etablissementId, etabId))
        .orderBy(asc(portesOuvertesSlot.startAt));
  return rows.map(mapSlot);
}

export async function upsertPortesOuvertesSlot(
  slot: PortesOuvertesSlot & { cycle: PortesOuvertesCycle },
  etablissementId?: string,
): Promise<PortesOuvertesSlot> {
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const startAt = new Date(slot.startAt);
  const endAt = new Date(slot.endAt);
  if (!Number.isFinite(startAt.getTime()) || !Number.isFinite(endAt.getTime())) {
    throw new Error("Dates de créneau invalides.");
  }
  await db
    .insert(portesOuvertesSlot)
    .values({
      etablissementId: etabId,
      id: slot.id,
      cycle: slot.cycle,
      label: slot.label,
      startAt,
      endAt,
      maxPlaces: slot.maxPlaces ?? null,
      updatedAt: new Date(),
    })
    .onConflictDoUpdate({
      target: [portesOuvertesSlot.etablissementId, portesOuvertesSlot.id],
      set: {
        cycle: slot.cycle,
        label: slot.label,
        startAt,
        endAt,
        maxPlaces: slot.maxPlaces ?? null,
        updatedAt: new Date(),
      },
    });
  return { ...slot, startAt: startAt.toISOString(), endAt: endAt.toISOString() };
}

export async function insertPortesOuvertesSlots(
  slots: Array<PortesOuvertesSlot & { cycle: PortesOuvertesCycle }>,
  etablissementId?: string,
): Promise<number> {
  let n = 0;
  for (const s of slots) {
    await upsertPortesOuvertesSlot(s, etablissementId);
    n += 1;
  }
  return n;
}

/** Suppression ciblée d’un créneau (id + etablissement) + staffing associé. */
export async function deletePortesOuvertesSlot(
  slotId: string,
  etablissementId?: string,
): Promise<boolean> {
  const etabId = await requireEtabId(etablissementId);
  const id = slotId.trim();
  if (!id) return false;
  const db = getDb();
  await db
    .delete(portesOuvertesSlotStaff)
    .where(
      and(
        eq(portesOuvertesSlotStaff.etablissementId, etabId),
        eq(portesOuvertesSlotStaff.slotId, id),
      ),
    );
  const deleted = await db
    .delete(portesOuvertesSlot)
    .where(
      and(eq(portesOuvertesSlot.etablissementId, etabId), eq(portesOuvertesSlot.id, id)),
    )
    .returning({ id: portesOuvertesSlot.id });
  return deleted.length > 0;
}

/** Remplace uniquement les créneaux d’un cycle donné (pas les autres). Nettoie le staffing des anciens ids. */
export async function replacePortesOuvertesSlotsForCycle(
  cycle: PortesOuvertesCycle,
  slots: Array<PortesOuvertesSlot & { cycle: PortesOuvertesCycle }>,
  etablissementId?: string,
): Promise<void> {
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const existing = await db
    .select({ id: portesOuvertesSlot.id })
    .from(portesOuvertesSlot)
    .where(
      and(eq(portesOuvertesSlot.etablissementId, etabId), eq(portesOuvertesSlot.cycle, cycle)),
    );
  for (const row of existing) {
    await db
      .delete(portesOuvertesSlotStaff)
      .where(
        and(
          eq(portesOuvertesSlotStaff.etablissementId, etabId),
          eq(portesOuvertesSlotStaff.slotId, row.id),
        ),
      );
    await db
      .delete(portesOuvertesSlot)
      .where(
        and(
          eq(portesOuvertesSlot.etablissementId, etabId),
          eq(portesOuvertesSlot.id, row.id),
        ),
      );
  }
  for (const s of slots) {
    await upsertPortesOuvertesSlot({ ...s, cycle }, etabId);
  }
}

export async function listPortesOuvertesRegistrations(
  etablissementId?: string,
): Promise<PortesOuvertesRegistration[]> {
  if (!isDatabaseConfigured()) return [];
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(portesOuvertesRegistration)
    .where(eq(portesOuvertesRegistration.etablissementId, etabId))
    .orderBy(asc(portesOuvertesRegistration.createdAt));
  return rows.map(mapRegistration);
}

export async function addPortesOuvertesRegistration(
  row: Omit<PortesOuvertesRegistration, "id" | "createdAt">,
  etablissementId?: string,
): Promise<PortesOuvertesRegistration> {
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const id = `po-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const createdAt = new Date();
  await db.insert(portesOuvertesRegistration).values({
    etablissementId: etabId,
    id,
    slotId: row.slotId,
    slotLabel: row.slotLabel || null,
    slotStartAt: row.slotStartAt ? new Date(row.slotStartAt) : null,
    slotEndAt: row.slotEndAt ? new Date(row.slotEndAt) : null,
    firstName: row.firstName,
    lastName: row.lastName,
    email: row.email,
    phone: row.phone || null,
    childrenInfo: row.childrenInfo || null,
    childFirstName: row.childFirstName || null,
    childLastName: row.childLastName || null,
    cycle: row.cycle || null,
    classeSouhaitee: row.classeSouhaitee || null,
    consent: row.consent,
    source: row.source || null,
    recordedByUserId: row.recordedBy?.userId || null,
    recordedByName: row.recordedBy?.name || null,
    createdAt,
  });
  return {
    ...row,
    id,
    createdAt: createdAt.toISOString(),
  };
}

export async function updatePortesOuvertesRegistration(
  id: string,
  patch: Partial<
    Omit<PortesOuvertesRegistration, "id" | "createdAt" | "consent" | "source" | "recordedBy">
  >,
  etablissementId?: string,
): Promise<PortesOuvertesRegistration | null> {
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const existing = await db
    .select()
    .from(portesOuvertesRegistration)
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etabId),
        eq(portesOuvertesRegistration.id, id),
      ),
    )
    .limit(1);
  const current = existing[0];
  if (!current) return null;

  const next = {
    slotId: patch.slotId ?? current.slotId,
    slotLabel: patch.slotLabel !== undefined ? patch.slotLabel || null : current.slotLabel,
    slotStartAt:
      patch.slotStartAt !== undefined
        ? patch.slotStartAt
          ? new Date(patch.slotStartAt)
          : null
        : current.slotStartAt,
    slotEndAt:
      patch.slotEndAt !== undefined
        ? patch.slotEndAt
          ? new Date(patch.slotEndAt)
          : null
        : current.slotEndAt,
    firstName: patch.firstName?.trim() || current.firstName,
    lastName: patch.lastName?.trim() || current.lastName,
    email: (patch.email?.trim().toLowerCase() || current.email).toLowerCase(),
    phone: patch.phone !== undefined ? patch.phone || null : current.phone,
    childrenInfo:
      patch.childrenInfo !== undefined ? patch.childrenInfo || null : current.childrenInfo,
    childFirstName:
      patch.childFirstName !== undefined
        ? patch.childFirstName || null
        : current.childFirstName,
    childLastName:
      patch.childLastName !== undefined ? patch.childLastName || null : current.childLastName,
    cycle: patch.cycle !== undefined ? patch.cycle || null : current.cycle,
    classeSouhaitee:
      patch.classeSouhaitee !== undefined
        ? patch.classeSouhaitee || null
        : current.classeSouhaitee,
    lastModifiedByUserId: patch.lastModifiedBy?.userId || current.lastModifiedByUserId,
    lastModifiedByName: patch.lastModifiedBy?.name || current.lastModifiedByName,
    updatedAt: new Date(),
  };

  await db
    .update(portesOuvertesRegistration)
    .set(next)
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etabId),
        eq(portesOuvertesRegistration.id, id),
      ),
    );

  const refreshed = await db
    .select()
    .from(portesOuvertesRegistration)
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etabId),
        eq(portesOuvertesRegistration.id, id),
      ),
    )
    .limit(1);
  return refreshed[0] ? mapRegistration(refreshed[0]) : null;
}

/** Suppression ciblée d’une inscription (id + établissement uniquement). */
export async function deletePortesOuvertesRegistration(
  id: string,
  etablissementId?: string,
): Promise<PortesOuvertesRegistration | null> {
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const existing = await db
    .select()
    .from(portesOuvertesRegistration)
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etabId),
        eq(portesOuvertesRegistration.id, id),
      ),
    )
    .limit(1);
  const current = existing[0];
  if (!current) return null;

  await db
    .delete(portesOuvertesRegistration)
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etabId),
        eq(portesOuvertesRegistration.id, id),
      ),
    );

  return mapRegistration(current);
}

export function countRegistrationsBySlot(
  rows: PortesOuvertesRegistration[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const r of rows) {
    out[r.slotId] = (out[r.slotId] || 0) + 1;
  }
  return out;
}

export type SlotCycleCounts = Record<
  string,
  Partial<Record<PortesOuvertesCycle, number>>
>;

export function countRegistrationsBySlotAndCycle(
  rows: PortesOuvertesRegistration[],
): SlotCycleCounts {
  const out: SlotCycleCounts = {};
  for (const r of rows) {
    if (!r.cycle) continue;
    const byCycle = out[r.slotId] ?? (out[r.slotId] = {});
    byCycle[r.cycle] = (byCycle[r.cycle] || 0) + 1;
  }
  return out;
}

export function remainingPlacesForSlotCycle(
  maxPlaces: number | undefined,
  counts: SlotCycleCounts,
  slotId: string,
  cycle: PortesOuvertesCycle,
): number | null {
  if (typeof maxPlaces !== "number") return null;
  return Math.max(0, maxPlaces - (counts[slotId]?.[cycle] || 0));
}

/** Capacité d’un créneau mono-cycle (compte toutes les inscriptions du slot). */
export async function countRegistrationsForSlot(
  slotId: string,
  etablissementId?: string,
): Promise<number> {
  if (!isDatabaseConfigured()) return 0;
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select({ n: count() })
    .from(portesOuvertesRegistration)
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etabId),
        eq(portesOuvertesRegistration.slotId, slotId),
      ),
    );
  return Number(rows[0]?.n || 0);
}

export async function listPortesOuvertesStaff(
  etablissementId?: string,
  slotId?: string,
): Promise<PortesOuvertesStaffRow[]> {
  if (!isDatabaseConfigured()) return [];
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = slotId
    ? await db
        .select()
        .from(portesOuvertesSlotStaff)
        .where(
          and(
            eq(portesOuvertesSlotStaff.etablissementId, etabId),
            eq(portesOuvertesSlotStaff.slotId, slotId),
          ),
        )
    : await db
        .select()
        .from(portesOuvertesSlotStaff)
        .where(eq(portesOuvertesSlotStaff.etablissementId, etabId));
  const out: PortesOuvertesStaffRow[] = [];
  for (const r of rows) {
    const role = parseStaffRole(r.role);
    if (!role) continue;
    out.push({
      id: r.id,
      slotId: r.slotId,
      role,
      refId: r.refId,
      displayName: r.displayName,
      meta: r.meta || undefined,
      createdAt: r.createdAt.toISOString(),
    });
  }
  return out;
}

export async function addPortesOuvertesStaff(
  input: {
    slotId: string;
    role: PortesOuvertesStaffRole;
    refId: string;
    displayName: string;
    meta?: Record<string, string>;
  },
  etablissementId?: string,
): Promise<PortesOuvertesStaffRow> {
  const etabId = await requireEtabId(etablissementId);
  const slotId = input.slotId.trim();
  const refId = input.refId.trim();
  const displayName = input.displayName.trim();
  if (!slotId || !refId || !displayName) {
    throw new Error("Créneau et personne requis.");
  }

  const existing = await listPortesOuvertesStaff(etabId, slotId);
  const alreadySame = existing.some((s) => s.role === input.role && s.refId === refId);
  if (!alreadySame) {
    if (input.role === "ambassadeur") {
      const n = existing.filter((s) => s.role === "ambassadeur").length;
      if (n >= PORTES_OUVERTES_MAX_AMBASSADEURS) {
        throw new Error(
          `Maximum ${PORTES_OUVERTES_MAX_AMBASSADEURS} élèves ambassadeurs par créneau.`,
        );
      }
    } else {
      const n = existing.filter((s) => s.role === "enseignant" || s.role === "personnel").length;
      if (n >= PORTES_OUVERTES_MAX_ENCADRANTS) {
        throw new Error(
          `Maximum ${PORTES_OUVERTES_MAX_ENCADRANTS} encadrants (professeur ou personnel OGEC) par créneau.`,
        );
      }
    }
  }

  const db = getDb();
  const inserted = await db
    .insert(portesOuvertesSlotStaff)
    .values({
      etablissementId: etabId,
      slotId,
      role: input.role,
      refId,
      displayName,
      meta: input.meta || null,
    })
    .onConflictDoUpdate({
      target: [
        portesOuvertesSlotStaff.etablissementId,
        portesOuvertesSlotStaff.slotId,
        portesOuvertesSlotStaff.role,
        portesOuvertesSlotStaff.refId,
      ],
      set: {
        displayName,
        meta: input.meta || null,
      },
    })
    .returning();
  const row = inserted[0];
  return {
    id: row.id,
    slotId: row.slotId,
    role: input.role,
    refId: row.refId,
    displayName: row.displayName,
    meta: row.meta || undefined,
    createdAt: row.createdAt.toISOString(),
  };
}

export async function deletePortesOuvertesStaff(
  staffId: string,
  etablissementId?: string,
): Promise<boolean> {
  const etabId = await requireEtabId(etablissementId);
  const id = staffId.trim();
  if (!id) return false;
  const db = getDb();
  const deleted = await db
    .delete(portesOuvertesSlotStaff)
    .where(
      and(
        eq(portesOuvertesSlotStaff.etablissementId, etabId),
        eq(portesOuvertesSlotStaff.id, id),
      ),
    )
    .returning({ id: portesOuvertesSlotStaff.id });
  return deleted.length > 0;
}

/** Check-in visite : programme le mail de suivi selon le délai config. */
export async function markPortesOuvertesVisited(
  registrationId: string,
  visited: boolean,
  etablissementId?: string,
): Promise<PortesOuvertesRegistration | null> {
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const config = await getPortesOuvertesConfig(etabId);
  const now = new Date();

  if (!visited) {
    const rows = await db
      .update(portesOuvertesRegistration)
      .set({
        visitedAt: null,
        followUpDueAt: null,
        updatedAt: now,
      })
      .where(
        and(
          eq(portesOuvertesRegistration.etablissementId, etabId),
          eq(portesOuvertesRegistration.id, registrationId),
        ),
      )
      .returning();
    return rows[0] ? mapRegistration(rows[0]) : null;
  }

  const followUpDueAt = new Date(now.getTime() + config.followUpDelayMinutes * 60 * 1000);
  const rows = await db
    .update(portesOuvertesRegistration)
    .set({
      visitedAt: now,
      followUpDueAt,
      followUpEmailSentAt: null,
      updatedAt: now,
    })
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etabId),
        eq(portesOuvertesRegistration.id, registrationId),
      ),
    )
    .returning();
  return rows[0] ? mapRegistration(rows[0]) : null;
}

export async function listDuePortesOuvertesFollowUps(limit = 50): Promise<
  Array<PortesOuvertesRegistration & { etablissementId: string }>
> {
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const now = new Date();
  const rows = await db
    .select()
    .from(portesOuvertesRegistration)
    .where(
      and(
        isNotNull(portesOuvertesRegistration.followUpDueAt),
        isNull(portesOuvertesRegistration.followUpEmailSentAt),
        lte(portesOuvertesRegistration.followUpDueAt, now),
        isNotNull(portesOuvertesRegistration.visitedAt),
      ),
    )
    .limit(limit);
  return rows.map((r) => ({
    ...mapRegistration(r),
    etablissementId: r.etablissementId,
  }));
}

export async function markPortesOuvertesFollowUpSent(
  registrationId: string,
  etablissementId: string,
): Promise<void> {
  const db = getDb();
  await db
    .update(portesOuvertesRegistration)
    .set({
      followUpEmailSentAt: new Date(),
      updatedAt: new Date(),
    })
    .where(
      and(
        eq(portesOuvertesRegistration.etablissementId, etablissementId),
        eq(portesOuvertesRegistration.id, registrationId),
      ),
    );
}

/** Construit la config outil (hors enabled toolbox) depuis SQL (+ repli toolbox si SQL vide). */
export async function buildPortesOuvertesToolPayload(etablissementId?: string): Promise<{
  title: string;
  intro: string;
  address: string;
  mapsUrl?: string;
  notifyEmail?: string;
  preinscriptionUrl?: string;
  followUpDelayMinutes: number;
  consentLabel: string;
  slots: PortesOuvertesSlot[];
  staff: PortesOuvertesStaffRow[];
}> {
  const [config, slots, staff] = await Promise.all([
    getPortesOuvertesConfig(etablissementId),
    listPortesOuvertesSlots(etablissementId),
    listPortesOuvertesStaff(etablissementId),
  ]);

  let merged = { ...config };
  const needsBackfill =
    !merged.address?.trim() ||
    !merged.mapsUrl?.trim() ||
    !merged.preinscriptionUrl?.trim() ||
    !merged.notifyEmail?.trim();

  if (needsBackfill) {
    try {
      const { getToolboxConfig } = await import("@/app/lib/toolbox-config");
      const toolbox = await getToolboxConfig();
      const po = toolbox.tools["portes-ouvertes"];
      merged = {
        ...merged,
        address: merged.address?.trim() || po.address || "",
        mapsUrl: merged.mapsUrl?.trim() || po.mapsUrl || undefined,
        notifyEmail: merged.notifyEmail?.trim() || po.notifyEmail || undefined,
        preinscriptionUrl:
          merged.preinscriptionUrl?.trim() || po.preinscriptionUrl || undefined,
        followUpDelayMinutes:
          merged.followUpDelayMinutes > 0
            ? merged.followUpDelayMinutes
            : typeof po.followUpDelayMinutes === "number" && po.followUpDelayMinutes > 0
              ? po.followUpDelayMinutes
              : 60,
      };
      // Réécrit en SQL pour ne plus perdre les valeurs au prochain enregistrement.
      if (
        (merged.address && merged.address !== config.address) ||
        (merged.mapsUrl && merged.mapsUrl !== config.mapsUrl) ||
        (merged.preinscriptionUrl && merged.preinscriptionUrl !== config.preinscriptionUrl) ||
        (merged.notifyEmail && merged.notifyEmail !== config.notifyEmail)
      ) {
        merged = await upsertPortesOuvertesConfig(merged, etablissementId);
      }
    } catch {
      /* ignore repli toolbox */
    }
  }

  return {
    ...merged,
    slots,
    staff,
  };
}
