import "server-only";

import { and, desc, eq, inArray, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { invitationPage, invitationRsvp } from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  diplomaLabel,
  isInvitationDiploma,
  isInvitationDiplomaMode,
  isInvitationResponse,
  isInvitationTheme,
  normalizeEleveName,
  slugifyInvitation,
  type InvitationDashboardStats,
  type InvitationDiploma,
  type InvitationDiplomaMode,
  type InvitationDuplicateSuspect,
  type InvitationPageRecord,
  type InvitationResponse,
  type InvitationRsvpRecord,
  type InvitationTheme,
} from "@/app/lib/invitation-types";

async function requireEtabId(explicit?: string): Promise<string> {
  const id = explicit || (await resolveCurrentEtablissementId());
  if (!id) throw new Error("Établissement introuvable (tenant).");
  return id;
}

function toIso(d: Date | null | undefined): string | null {
  if (!d) return null;
  return d.toISOString();
}

function mapPage(row: typeof invitationPage.$inferSelect): InvitationPageRecord {
  const theme: InvitationTheme = isInvitationTheme(row.theme) ? row.theme : "remise_diplome";
  const diplomaMode: InvitationDiplomaMode = isInvitationDiplomaMode(row.diplomaMode)
    ? row.diplomaMode
    : "both";
  return {
    id: row.id,
    etablissementId: row.etablissementId,
    slug: row.slug,
    title: row.title,
    intro: row.intro,
    theme,
    enabled: row.enabled === 1,
    startsAt: toIso(row.startsAt),
    endsAt: toIso(row.endsAt),
    location: row.location || "",
    diplomaMode,
    maxTotalPersons: row.maxTotalPersons,
    maxPersonsPerEleve: row.maxPersonsPerEleve,
    notifyEmail: row.notifyEmail?.trim() || null,
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
    updatedAt: toIso(row.updatedAt) || new Date().toISOString(),
  };
}

function mapRsvp(row: typeof invitationRsvp.$inferSelect): InvitationRsvpRecord {
  const response: InvitationResponse = isInvitationResponse(row.response) ? row.response : "non";
  const diploma =
    row.diploma && isInvitationDiploma(row.diploma) ? row.diploma : null;
  return {
    id: row.id,
    etablissementId: row.etablissementId,
    pageId: row.pageId,
    eleveFirstName: row.eleveFirstName,
    eleveLastName: row.eleveLastName,
    eleveNameNorm: row.eleveNameNorm,
    response,
    presentCount: row.presentCount,
    parentEmail: row.parentEmail,
    diploma,
    duplicateGroupId: row.duplicateGroupId,
    duplicateDismissedAt: toIso(row.duplicateDismissedAt),
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
    updatedAt: toIso(row.updatedAt) || new Date().toISOString(),
  };
}

export async function listInvitationPages(
  etablissementId?: string,
): Promise<InvitationPageRecord[]> {
  if (!isDatabaseConfigured()) return [];
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(invitationPage)
    .where(eq(invitationPage.etablissementId, etabId))
    .orderBy(desc(invitationPage.createdAt));
  return rows.map(mapPage);
}

export async function getInvitationPageById(
  pageId: string,
  etablissementId?: string,
): Promise<InvitationPageRecord | null> {
  if (!isDatabaseConfigured()) return null;
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(invitationPage)
    .where(and(eq(invitationPage.etablissementId, etabId), eq(invitationPage.id, pageId)))
    .limit(1);
  return rows[0] ? mapPage(rows[0]) : null;
}

export async function getInvitationPageBySlug(
  slug: string,
  etablissementId?: string,
): Promise<InvitationPageRecord | null> {
  if (!isDatabaseConfigured()) return null;
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(invitationPage)
    .where(and(eq(invitationPage.etablissementId, etabId), eq(invitationPage.slug, slug)))
    .limit(1);
  return rows[0] ? mapPage(rows[0]) : null;
}

export type CreateInvitationPageInput = {
  title: string;
  slug?: string;
  intro?: string;
  theme?: InvitationTheme;
  enabled?: boolean;
  startsAt?: string | null;
  endsAt?: string | null;
  location?: string;
  diplomaMode?: InvitationDiplomaMode;
  maxTotalPersons?: number;
  maxPersonsPerEleve?: number;
  notifyEmail?: string | null;
};

async function ensureUniqueSlug(
  etabId: string,
  baseSlug: string,
  excludeId?: string,
): Promise<string> {
  const db = getDb();
  let candidate = slugifyInvitation(baseSlug);
  for (let i = 0; i < 50; i++) {
    const trySlug = i === 0 ? candidate : `${candidate}-${i + 1}`;
    const rows = await db
      .select({ id: invitationPage.id })
      .from(invitationPage)
      .where(
        and(eq(invitationPage.etablissementId, etabId), eq(invitationPage.slug, trySlug)),
      )
      .limit(1);
    if (!rows[0] || (excludeId && rows[0].id === excludeId)) return trySlug;
  }
  return `${candidate}-${Date.now().toString(36)}`;
}

function parseOptionalDate(iso: string | null | undefined): Date | null {
  if (iso == null || iso === "") return null;
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? null : d;
}

export async function createInvitationPage(
  input: CreateInvitationPageInput,
  etablissementId?: string,
): Promise<InvitationPageRecord> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const db = getDb();
  const title = input.title.trim().slice(0, 200) || "Invitation";
  const slug = await ensureUniqueSlug(etabId, input.slug?.trim() || title);
  const theme: InvitationTheme =
    input.theme && isInvitationTheme(input.theme) ? input.theme : "remise_diplome";
  const diplomaMode: InvitationDiplomaMode =
    input.diplomaMode && isInvitationDiplomaMode(input.diplomaMode)
      ? input.diplomaMode
      : "both";
  const [row] = await db
    .insert(invitationPage)
    .values({
      etablissementId: etabId,
      slug,
      title,
      intro: (input.intro || "").slice(0, 4000),
      theme,
      enabled: input.enabled ? 1 : 0,
      startsAt: parseOptionalDate(input.startsAt),
      endsAt: parseOptionalDate(input.endsAt),
      location: (input.location || "").slice(0, 500),
      diplomaMode,
      maxTotalPersons: Math.max(1, Math.min(50000, input.maxTotalPersons ?? 200)),
      maxPersonsPerEleve: Math.max(1, Math.min(50, input.maxPersonsPerEleve ?? 4)),
      notifyEmail: input.notifyEmail?.trim() || null,
      updatedAt: new Date(),
    })
    .returning();
  return mapPage(row);
}

export type UpdateInvitationPageInput = Partial<CreateInvitationPageInput> & {
  slug?: string;
};

export async function updateInvitationPage(
  pageId: string,
  input: UpdateInvitationPageInput,
  etablissementId?: string,
): Promise<InvitationPageRecord | null> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return null;
  const existing = await getInvitationPageById(pageId, etabId);
  if (!existing) return null;

  const db = getDb();
  const patch: Partial<typeof invitationPage.$inferInsert> = {
    updatedAt: new Date(),
  };
  if (input.title !== undefined) patch.title = input.title.trim().slice(0, 200) || existing.title;
  if (input.intro !== undefined) patch.intro = input.intro.slice(0, 4000);
  if (input.theme !== undefined && isInvitationTheme(input.theme)) patch.theme = input.theme;
  if (input.enabled !== undefined) patch.enabled = input.enabled ? 1 : 0;
  if (input.startsAt !== undefined) patch.startsAt = parseOptionalDate(input.startsAt);
  if (input.endsAt !== undefined) patch.endsAt = parseOptionalDate(input.endsAt);
  if (input.location !== undefined) patch.location = input.location.slice(0, 500);
  if (input.diplomaMode !== undefined && isInvitationDiplomaMode(input.diplomaMode)) {
    patch.diplomaMode = input.diplomaMode;
  }
  if (input.maxTotalPersons !== undefined) {
    patch.maxTotalPersons = Math.max(1, Math.min(50000, input.maxTotalPersons));
  }
  if (input.maxPersonsPerEleve !== undefined) {
    patch.maxPersonsPerEleve = Math.max(1, Math.min(50, input.maxPersonsPerEleve));
  }
  if (input.notifyEmail !== undefined) {
    patch.notifyEmail = input.notifyEmail?.trim() || null;
  }
  if (input.slug !== undefined) {
    patch.slug = await ensureUniqueSlug(etabId, input.slug, pageId);
  }

  const [row] = await db
    .update(invitationPage)
    .set(patch)
    .where(and(eq(invitationPage.etablissementId, etabId), eq(invitationPage.id, pageId)))
    .returning();
  return row ? mapPage(row) : null;
}

export async function deleteInvitationPage(
  pageId: string,
  etablissementId?: string,
): Promise<boolean> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return false;
  const db = getDb();
  const deleted = await db
    .delete(invitationPage)
    .where(and(eq(invitationPage.etablissementId, etabId), eq(invitationPage.id, pageId)))
    .returning({ id: invitationPage.id });
  return deleted.length > 0;
}

export async function sumOuiPresentCount(pageId: string, etablissementId?: string): Promise<number> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return 0;
  const db = getDb();
  const rows = await db
    .select({
      total: sql<number>`coalesce(sum(${invitationRsvp.presentCount}), 0)::int`,
    })
    .from(invitationRsvp)
    .where(
      and(
        eq(invitationRsvp.etablissementId, etabId),
        eq(invitationRsvp.pageId, pageId),
        eq(invitationRsvp.response, "oui"),
      ),
    );
  return Number(rows[0]?.total ?? 0);
}

export async function listInvitationRsvps(
  pageId: string,
  etablissementId?: string,
): Promise<InvitationRsvpRecord[]> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const rows = await db
    .select()
    .from(invitationRsvp)
    .where(and(eq(invitationRsvp.etablissementId, etabId), eq(invitationRsvp.pageId, pageId)))
    .orderBy(desc(invitationRsvp.createdAt));
  return rows.map(mapRsvp);
}

export function computeDashboardStats(
  page: InvitationPageRecord,
  rsvps: InvitationRsvpRecord[],
): InvitationDashboardStats {
  let ouiCount = 0;
  let nonCount = 0;
  let totalPersons = 0;
  for (const r of rsvps) {
    if (r.response === "oui") {
      ouiCount += 1;
      totalPersons += r.presentCount;
    } else {
      nonCount += 1;
    }
  }
  const placesRemaining = Math.max(0, page.maxTotalPersons - totalPersons);
  return {
    ouiCount,
    nonCount,
    totalPersons,
    placesRemaining,
    maxTotalPersons: page.maxTotalPersons,
  };
}

/** Doublons suspects : même nom normalisé, ≥2 RSVP, pas tous reliés au même groupe, non dismiss. */
export function findDuplicateSuspects(rsvps: InvitationRsvpRecord[]): InvitationDuplicateSuspect[] {
  const byNorm = new Map<string, InvitationRsvpRecord[]>();
  for (const r of rsvps) {
    if (r.duplicateDismissedAt) continue;
    const list = byNorm.get(r.eleveNameNorm) || [];
    list.push(r);
    byNorm.set(r.eleveNameNorm, list);
  }
  const out: InvitationDuplicateSuspect[] = [];
  for (const [norm, group] of byNorm) {
    if (group.length < 2) continue;
    const linkedIds = new Set(
      group.map((g) => g.duplicateGroupId).filter((id): id is string => Boolean(id)),
    );
    const allSameGroup =
      linkedIds.size === 1 && group.every((g) => g.duplicateGroupId === [...linkedIds][0]);
    if (allSameGroup) continue;
    const sample = group[0];
    out.push({
      eleveNameNorm: norm,
      eleveLabel: `${sample.eleveFirstName} ${sample.eleveLastName}`.trim(),
      rsvpIds: group.map((g) => g.id),
    });
  }
  return out.sort((a, b) => a.eleveLabel.localeCompare(b.eleveLabel, "fr"));
}

export async function linkInvitationDuplicates(
  pageId: string,
  rsvpIds: string[],
  etablissementId?: string,
): Promise<{ ok: true; groupId: string } | { ok: false; error: string }> {
  const etabId = await requireEtabId(etablissementId);
  if (rsvpIds.length < 2) return { ok: false, error: "Sélectionnez au moins deux réponses." };
  if (!isDatabaseConfigured()) return { ok: false, error: "Base de données non configurée." };
  const db = getDb();
  const rows = await db
    .select()
    .from(invitationRsvp)
    .where(
      and(
        eq(invitationRsvp.etablissementId, etabId),
        eq(invitationRsvp.pageId, pageId),
        inArray(invitationRsvp.id, rsvpIds),
      ),
    );
  if (rows.length !== rsvpIds.length) {
    return { ok: false, error: "Certaines réponses sont introuvables." };
  }
  const groupId = crypto.randomUUID();
  const now = new Date();
  await db
    .update(invitationRsvp)
    .set({ duplicateGroupId: groupId, duplicateDismissedAt: null, updatedAt: now })
    .where(
      and(
        eq(invitationRsvp.etablissementId, etabId),
        eq(invitationRsvp.pageId, pageId),
        inArray(invitationRsvp.id, rsvpIds),
      ),
    );
  return { ok: true, groupId };
}

export async function dismissInvitationDuplicates(
  pageId: string,
  rsvpIds: string[],
  etablissementId?: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const etabId = await requireEtabId(etablissementId);
  if (rsvpIds.length < 1) return { ok: false, error: "Aucune réponse sélectionnée." };
  if (!isDatabaseConfigured()) return { ok: false, error: "Base de données non configurée." };
  const db = getDb();
  const now = new Date();
  await db
    .update(invitationRsvp)
    .set({ duplicateDismissedAt: now, updatedAt: now })
    .where(
      and(
        eq(invitationRsvp.etablissementId, etabId),
        eq(invitationRsvp.pageId, pageId),
        inArray(invitationRsvp.id, rsvpIds),
      ),
    );
  return { ok: true };
}

export type RegisterInvitationInput = {
  slug: string;
  eleveFirstName: string;
  eleveLastName: string;
  response: InvitationResponse;
  presentCount?: number;
  parentEmail: string;
  diploma?: InvitationDiploma | null;
};

export type RegisterInvitationResult =
  | { ok: true; rsvp: InvitationRsvpRecord; page: InvitationPageRecord }
  | { ok: false; error: string; status: number };

export async function registerInvitationRsvp(
  input: RegisterInvitationInput,
  etablissementId?: string,
): Promise<RegisterInvitationResult> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) {
    return { ok: false, error: "Service indisponible.", status: 503 };
  }

  const eleveFirstName = input.eleveFirstName.trim().slice(0, 80);
  const eleveLastName = input.eleveLastName.trim().slice(0, 80);
  const parentEmail = input.parentEmail.trim().toLowerCase().slice(0, 200);
  if (!eleveFirstName || !eleveLastName) {
    return { ok: false, error: "Nom et prénom de l’élève requis.", status: 400 };
  }
  if (!parentEmail || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(parentEmail)) {
    return { ok: false, error: "E-mail parent invalide.", status: 400 };
  }
  if (!isInvitationResponse(input.response)) {
    return { ok: false, error: "Réponse Oui / Non requise.", status: 400 };
  }

  const db = getDb();

  return db.transaction(async (tx) => {
    const pages = await tx
      .select()
      .from(invitationPage)
      .where(
        and(eq(invitationPage.etablissementId, etabId), eq(invitationPage.slug, input.slug)),
      )
      .limit(1);
    const pageRow = pages[0];
    if (!pageRow || pageRow.enabled !== 1) {
      return { ok: false as const, error: "Cette invitation n’est pas disponible.", status: 404 };
    }
    const page = mapPage(pageRow);

    let diploma: InvitationDiploma | null = null;
    if (page.diplomaMode === "bac") diploma = "bac";
    else if (page.diplomaMode === "brevet") diploma = "brevet";
    else if (page.diplomaMode === "both") {
      if (!input.diploma || !isInvitationDiploma(input.diploma)) {
        return {
          ok: false as const,
          error: "Veuillez indiquer le diplôme (bac ou brevet).",
          status: 400,
        };
      }
      diploma = input.diploma;
    }

    let presentCount = 0;
    if (input.response === "oui") {
      const raw = Number(input.presentCount ?? 0);
      if (!Number.isInteger(raw) || raw < 1) {
        return {
          ok: false as const,
          error: "Indiquez le nombre de personnes présentes (moi compris).",
          status: 400,
        };
      }
      if (raw > page.maxPersonsPerEleve) {
        return {
          ok: false as const,
          error: `Maximum ${page.maxPersonsPerEleve} personne(s) par élève.`,
          status: 400,
        };
      }
      presentCount = raw;

      // Verrouillage optimiste : somme actuelle sous transaction
      const sumRows = await tx
        .select({
          total: sql<number>`coalesce(sum(${invitationRsvp.presentCount}), 0)::int`,
        })
        .from(invitationRsvp)
        .where(
          and(
            eq(invitationRsvp.etablissementId, etabId),
            eq(invitationRsvp.pageId, page.id),
            eq(invitationRsvp.response, "oui"),
          ),
        );
      const used = Number(sumRows[0]?.total ?? 0);
      if (used + presentCount > page.maxTotalPersons) {
        const left = Math.max(0, page.maxTotalPersons - used);
        return {
          ok: false as const,
          error:
            left === 0
              ? "Il n’y a plus de places disponibles."
              : `Il ne reste que ${left} place(s).`,
          status: 409,
        };
      }
    }

    const eleveNameNorm = normalizeEleveName(eleveFirstName, eleveLastName);
    const [inserted] = await tx
      .insert(invitationRsvp)
      .values({
        etablissementId: etabId,
        pageId: page.id,
        eleveFirstName,
        eleveLastName,
        eleveNameNorm,
        response: input.response,
        presentCount,
        parentEmail,
        diploma,
        updatedAt: new Date(),
      })
      .returning();

    return {
      ok: true as const,
      rsvp: mapRsvp(inserted),
      page,
    };
  });
}

export function formatInvitationWhen(page: InvitationPageRecord): string {
  if (!page.startsAt) return "";
  const start = new Date(page.startsAt);
  const day = start.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const time = start.toLocaleTimeString("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
  });
  return `${day} à ${time}`;
}

export function invitationDiplomaDisplay(
  page: InvitationPageRecord,
  diploma: InvitationDiploma | null,
): string {
  if (page.diplomaMode === "none") return "";
  return diplomaLabel(diploma);
}

/** Réexport utilitaire pour l’UI admin. */
export { diplomaLabel, normalizeEleveName, slugifyInvitation };
