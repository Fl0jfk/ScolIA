import "server-only";

import { and, desc, eq, inArray, or, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { invitationEligible, invitationPage, invitationRsvp } from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  diplomaLabel,
  formatInvitationEleveLabel,
  formatInvitationFirstName,
  formatInvitationLastName,
  isInvitationAskSituation,
  isInvitationDiploma,
  isInvitationDiplomaMode,
  isInvitationResponse,
  isInvitationSituationStatus,
  isInvitationTheme,
  isInvitationRsvpOpen,
  normalizeEleveName,
  parseInvitationBirthDate,
  pickBestIdentityMatch,
  shouldAskSituation,
  slugifyInvitation,
  type InvitationAskSituation,
  type InvitationDashboardStats,
  type InvitationDiploma,
  type InvitationDiplomaMode,
  type InvitationDuplicateSuspect,
  type InvitationEligibleRecord,
  type InvitationPageRecord,
  type InvitationResponse,
  type InvitationRsvpRecord,
  type InvitationSituationStatus,
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

function birthDateToIso(value: string | Date | null | undefined): string | null {
  if (value == null || value === "") return null;
  if (value instanceof Date) {
    if (Number.isNaN(value.getTime())) return null;
    const y = value.getUTCFullYear();
    const m = String(value.getUTCMonth() + 1).padStart(2, "0");
    const d = String(value.getUTCDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }
  const raw = String(value).slice(0, 10);
  return parseInvitationBirthDate(raw) || (/^\d{4}-\d{2}-\d{2}$/.test(raw) ? raw : null);
}

function mapPage(row: typeof invitationPage.$inferSelect): InvitationPageRecord {
  const theme: InvitationTheme = isInvitationTheme(row.theme) ? row.theme : "remise_diplome";
  const diplomaMode: InvitationDiplomaMode = isInvitationDiplomaMode(row.diplomaMode)
    ? row.diplomaMode
    : "both";
  const askSituation: InvitationAskSituation = isInvitationAskSituation(row.askSituation)
    ? row.askSituation
    : "off";
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
    rsvpClosesAt: toIso(row.rsvpClosesAt),
    location: row.location || "",
    diplomaMode,
    maxTotalPersons: row.maxTotalPersons,
    maxPersonsPerEleve: row.maxPersonsPerEleve,
    notifyEmail: row.notifyEmail?.trim() || null,
    requireEligible: row.requireEligible === 1,
    askSituation,
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
    updatedAt: toIso(row.updatedAt) || new Date().toISOString(),
  };
}

function mapRsvp(row: typeof invitationRsvp.$inferSelect): InvitationRsvpRecord {
  const response: InvitationResponse = isInvitationResponse(row.response) ? row.response : "non";
  const diploma =
    row.diploma && isInvitationDiploma(row.diploma) ? row.diploma : null;
  const situationStatus =
    row.situationStatus && isInvitationSituationStatus(row.situationStatus)
      ? row.situationStatus
      : null;
  return {
    id: row.id,
    etablissementId: row.etablissementId,
    pageId: row.pageId,
    eligibleId: row.eligibleId || null,
    eleveFirstName: formatInvitationFirstName(row.eleveFirstName),
    eleveLastName: formatInvitationLastName(row.eleveLastName),
    eleveNameNorm: row.eleveNameNorm,
    birthDate: birthDateToIso(row.birthDate),
    response,
    presentCount: row.presentCount,
    parentEmail: row.parentEmail,
    diploma,
    situationStatus,
    situationDetail: row.situationDetail || "",
    situationEstablishment: row.situationEstablishment || "",
    duplicateGroupId: row.duplicateGroupId,
    duplicateDismissedAt: toIso(row.duplicateDismissedAt),
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
    updatedAt: toIso(row.updatedAt) || new Date().toISOString(),
  };
}

type InvitationQueryDb = {
  select: ReturnType<typeof getDb>["select"];
};

function identityMatchFilters(input: {
  nameNorm: string;
  birthDate: string;
}) {
  return or(
    eq(invitationEligible.eleveNameNorm, input.nameNorm),
    eq(invitationEligible.birthDate, input.birthDate),
  );
}

function rsvpIdentityMatchFilters(input: {
  nameNorm: string;
  birthDate: string;
}) {
  return or(
    eq(invitationRsvp.eleveNameNorm, input.nameNorm),
    eq(invitationRsvp.birthDate, input.birthDate),
  );
}

async function loadEligibleCandidates(
  db: InvitationQueryDb,
  etabId: string,
  pageId: string,
  identity: { firstName: string; lastName: string; birthDate: string },
): Promise<InvitationEligibleRecord[]> {
  const nameNorm = normalizeEleveName(identity.firstName, identity.lastName);
  const rows = await db
    .select()
    .from(invitationEligible)
    .where(
      and(
        eq(invitationEligible.etablissementId, etabId),
        eq(invitationEligible.pageId, pageId),
        identityMatchFilters({ nameNorm, birthDate: identity.birthDate }),
      ),
    );
  return rows.map(mapEligible);
}

async function loadRsvpIdentityCandidates(
  db: InvitationQueryDb,
  etabId: string,
  pageId: string,
  identity: { firstName: string; lastName: string; birthDate: string },
): Promise<(typeof invitationRsvp.$inferSelect)[]> {
  const nameNorm = normalizeEleveName(identity.firstName, identity.lastName);
  return db
    .select()
    .from(invitationRsvp)
    .where(
      and(
        eq(invitationRsvp.etablissementId, etabId),
        eq(invitationRsvp.pageId, pageId),
        rsvpIdentityMatchFilters({ nameNorm, birthDate: identity.birthDate }),
      ),
    );
}

function mapEligible(row: typeof invitationEligible.$inferSelect): InvitationEligibleRecord {
  const diploma =
    row.diploma && isInvitationDiploma(row.diploma) ? row.diploma : null;
  return {
    id: row.id,
    etablissementId: row.etablissementId,
    pageId: row.pageId,
    eleveFirstName: formatInvitationFirstName(row.eleveFirstName),
    eleveLastName: formatInvitationLastName(row.eleveLastName),
    eleveNameNorm: row.eleveNameNorm,
    birthDate: birthDateToIso(row.birthDate),
    diploma,
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
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
  requireEligible?: boolean;
  askSituation?: InvitationAskSituation;
  rsvpClosesAt?: string | null;
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
      requireEligible: input.requireEligible ? 1 : 0,
      askSituation:
        input.askSituation && isInvitationAskSituation(input.askSituation)
          ? input.askSituation
          : "off",
      rsvpClosesAt: parseOptionalDate(input.rsvpClosesAt),
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
  if (input.requireEligible !== undefined) {
    patch.requireEligible = input.requireEligible ? 1 : 0;
  }
  if (input.askSituation !== undefined && isInvitationAskSituation(input.askSituation)) {
    patch.askSituation = input.askSituation;
  }
  if (input.rsvpClosesAt !== undefined) {
    patch.rsvpClosesAt = parseOptionalDate(input.rsvpClosesAt);
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
  eligible: InvitationEligibleRecord[] = [],
): InvitationDashboardStats {
  let ouiCount = 0;
  let nonCount = 0;
  let totalPersons = 0;
  const rsvpByEligibleId = new Set<string>();
  const rsvpByNormBirth = new Set<string>();
  for (const r of rsvps) {
    if (r.response === "oui") {
      ouiCount += 1;
      totalPersons += r.presentCount;
    } else {
      nonCount += 1;
    }
    if (r.eligibleId) rsvpByEligibleId.add(r.eligibleId);
    rsvpByNormBirth.add(`${r.eleveNameNorm}|${r.birthDate || ""}`);
  }
  let respondedFromList = 0;
  for (const e of eligible) {
    if (
      rsvpByEligibleId.has(e.id) ||
      rsvpByNormBirth.has(`${e.eleveNameNorm}|${e.birthDate || ""}`)
    ) {
      respondedFromList += 1;
    }
  }
  const eligibleCount = eligible.length;
  const respondedCount = eligibleCount > 0 ? respondedFromList : rsvps.length;
  const pendingCount = eligibleCount > 0 ? Math.max(0, eligibleCount - respondedFromList) : 0;
  const placesRemaining = Math.max(0, page.maxTotalPersons - totalPersons);
  return {
    ouiCount,
    nonCount,
    totalPersons,
    placesRemaining,
    maxTotalPersons: page.maxTotalPersons,
    eligibleCount,
    respondedCount,
    pendingCount,
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
      eleveLabel: formatInvitationEleveLabel(sample.eleveFirstName, sample.eleveLastName),
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
  birthDate?: string | null;
  response: InvitationResponse;
  presentCount?: number;
  parentEmail: string;
  diploma?: InvitationDiploma | null;
  situationStatus?: InvitationSituationStatus | null;
  situationDetail?: string;
  situationEstablishment?: string;
};

export type RegisterInvitationResult =
  | {
      ok: true;
      rsvp: InvitationRsvpRecord;
      page: InvitationPageRecord;
      updated: boolean;
    }
  | { ok: false; error: string; status: number };

export type LookupInvitationResult =
  | {
      ok: true;
      page: InvitationPageRecord;
      eligibleId: string | null;
      existing: InvitationRsvpRecord | null;
      matchScore: number | null;
      eleveFirstName: string;
      eleveLastName: string;
    }
  | { ok: false; error: string; status: number };

function resolveDiplomaForPage(
  page: InvitationPageRecord,
  inputDiploma: InvitationDiploma | null | undefined,
  eligibleDiploma: InvitationDiploma | null,
): { diploma: InvitationDiploma | null; error?: string } {
  let diploma: InvitationDiploma | null = null;
  if (page.diplomaMode === "bac") diploma = "bac";
  else if (page.diplomaMode === "brevet") diploma = "brevet";
  else if (page.diplomaMode === "both") {
    if (!inputDiploma || !isInvitationDiploma(inputDiploma)) {
      return { diploma: null, error: "Veuillez indiquer le diplôme (bac ou brevet)." };
    }
    diploma = inputDiploma;
  }
  if (eligibleDiploma && page.diplomaMode === "both" && diploma && diploma !== eligibleDiploma) {
    return {
      diploma: null,
      error: "Pour cet élève, le diplôme attendu est : " + diplomaLabel(eligibleDiploma) + ".",
    };
  }
  if (eligibleDiploma && (page.diplomaMode === "none" || !diploma)) {
    diploma = eligibleDiploma;
  }
  return { diploma };
}

export async function lookupInvitationIdentity(
  input: {
    slug: string;
    eleveFirstName: string;
    eleveLastName: string;
    birthDate?: string | null;
  },
  etablissementId?: string,
): Promise<LookupInvitationResult> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) {
    return { ok: false, error: "Service indisponible.", status: 503 };
  }
  const eleveFirstName = formatInvitationFirstName(input.eleveFirstName).slice(0, 80);
  const eleveLastName = formatInvitationLastName(input.eleveLastName).slice(0, 80);
  const birthDate = parseInvitationBirthDate(String(input.birthDate || "")) ||
    birthDateToIso(input.birthDate) ||
    null;
  if (!eleveFirstName || !eleveLastName) {
    return { ok: false, error: "Nom et prénom de l'élève requis.", status: 400 };
  }
  if (!birthDate) {
    return {
      ok: false,
      error: "Indiquez la date de naissance de l'élève.",
      status: 400,
    };
  }

  const page = await getInvitationPageBySlug(input.slug, etabId);
  if (!page || !page.enabled) {
    return { ok: false, error: "Cette invitation n'est pas disponible.", status: 404 };
  }
  if (!isInvitationRsvpOpen(page.rsvpClosesAt)) {
    return {
      ok: false,
      error: "La date limite de réponse est dépassée.",
      status: 403,
    };
  }

  const identity = { firstName: eleveFirstName, lastName: eleveLastName, birthDate };

  const db = getDb();

  if (page.requireEligible) {
    const eligibleList = await loadEligibleCandidates(db, etabId, page.id, {
      firstName: eleveFirstName,
      lastName: eleveLastName,
      birthDate,
    });
    const picked = pickBestIdentityMatch(identity, eligibleList, 2);
    if (!picked) {
      return {
        ok: false,
        error:
          "Aucun élève correspondant (il faut au moins 2 critères exacts parmi prénom, nom et date de naissance). Vérifiez la saisie.",
        status: 403,
      };
    }
    const existingRows = await db
      .select()
      .from(invitationRsvp)
      .where(
        and(
          eq(invitationRsvp.etablissementId, etabId),
          eq(invitationRsvp.pageId, page.id),
          eq(invitationRsvp.eligibleId, picked.match.id),
        ),
      )
      .limit(1);
    return {
      ok: true,
      page,
      eligibleId: picked.match.id,
      existing: existingRows[0] ? mapRsvp(existingRows[0]) : null,
      matchScore: picked.score,
      eleveFirstName: picked.match.eleveFirstName,
      eleveLastName: picked.match.eleveLastName,
    };
  }

  const rsvpRows = await loadRsvpIdentityCandidates(db, etabId, page.id, {
    firstName: eleveFirstName,
    lastName: eleveLastName,
    birthDate,
  });
  const picked = pickBestIdentityMatch(
    identity,
    rsvpRows.map((r) => ({
      id: r.id,
      eleveFirstName: r.eleveFirstName,
      eleveLastName: r.eleveLastName,
      birthDate: birthDateToIso(r.birthDate),
    })),
    2,
  );
  const existingRow = picked ? rsvpRows.find((r) => r.id === picked.match.id) : undefined;
  const existing = existingRow ? mapRsvp(existingRow) : null;
  return {
    ok: true,
    page,
    eligibleId: null,
    existing,
    matchScore: picked?.score ?? null,
    eleveFirstName: existing?.eleveFirstName || eleveFirstName,
    eleveLastName: existing?.eleveLastName || eleveLastName,
  };
}

export async function registerInvitationRsvp(
  input: RegisterInvitationInput,
  etablissementId?: string,
): Promise<RegisterInvitationResult> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) {
    return { ok: false, error: "Service indisponible.", status: 503 };
  }

  let eleveFirstName = formatInvitationFirstName(input.eleveFirstName).slice(0, 80);
  let eleveLastName = formatInvitationLastName(input.eleveLastName).slice(0, 80);
  const parentEmail = input.parentEmail.trim().toLowerCase().slice(0, 200);
  const birthDate = parseInvitationBirthDate(String(input.birthDate || "")) ||
    birthDateToIso(input.birthDate) ||
    null;
  if (!eleveFirstName || !eleveLastName) {
    return { ok: false, error: "Nom et prénom de l'élève requis.", status: 400 };
  }
  if (!birthDate) {
    return {
      ok: false,
      error: "Indiquez la date de naissance de l'élève.",
      status: 400,
    };
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
      return { ok: false as const, error: "Cette invitation n'est pas disponible.", status: 404 };
    }
    const page = mapPage(pageRow);
    if (!isInvitationRsvpOpen(page.rsvpClosesAt)) {
      return {
        ok: false as const,
        error: "La date limite de réponse est dépassée.",
        status: 403,
      };
    }

    const identity = { firstName: eleveFirstName, lastName: eleveLastName, birthDate };
    let eligibleId: string | null = null;
    let eligibleDiploma: InvitationDiploma | null = null;

    if (page.requireEligible) {
      const eligibleList = await loadEligibleCandidates(tx, etabId, page.id, identity);
      const picked = pickBestIdentityMatch(identity, eligibleList, 2);
      if (!picked) {
        return {
          ok: false as const,
          error:
            "Aucun élève correspondant (2 critères sur 3 : prénom, nom, date de naissance). Vérifiez la saisie.",
          status: 403,
        };
      }
      eligibleId = picked.match.id;
      eligibleDiploma = picked.match.diploma;
      eleveFirstName = picked.match.eleveFirstName;
      eleveLastName = picked.match.eleveLastName;
    }

    const diplomaResolved = resolveDiplomaForPage(page, input.diploma ?? null, eligibleDiploma);
    if (diplomaResolved.error) {
      return { ok: false as const, error: diplomaResolved.error, status: 400 };
    }
    const diploma = diplomaResolved.diploma;

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
          error: "Maximum " + page.maxPersonsPerEleve + " personne(s) par élève.",
          status: 400,
        };
      }
      presentCount = raw;
    }

    let situationStatus: InvitationSituationStatus | null = null;
    let situationDetail = "";
    let situationEstablishment = "";
    if (shouldAskSituation(page.askSituation, diploma, page.diplomaMode)) {
      if (input.situationStatus && isInvitationSituationStatus(input.situationStatus)) {
        situationStatus = input.situationStatus;
      }
      situationDetail = (input.situationDetail || "").trim().slice(0, 300);
      situationEstablishment = (input.situationEstablishment || "").trim().slice(0, 200);
    }

    const eleveNameNorm = normalizeEleveName(eleveFirstName, eleveLastName);

    // RSVP existante ?
    let existingRow: typeof invitationRsvp.$inferSelect | undefined;
    if (eligibleId) {
      const found = await tx
        .select()
        .from(invitationRsvp)
        .where(
          and(
            eq(invitationRsvp.etablissementId, etabId),
            eq(invitationRsvp.pageId, page.id),
            eq(invitationRsvp.eligibleId, eligibleId),
          ),
        )
        .limit(1);
      existingRow = found[0];
    } else {
      const all = await loadRsvpIdentityCandidates(tx, etabId, page.id, identity);
      const picked = pickBestIdentityMatch(
        identity,
        all.map((r) => ({
          id: r.id,
          eleveFirstName: r.eleveFirstName,
          eleveLastName: r.eleveLastName,
          birthDate: birthDateToIso(r.birthDate),
        })),
        2,
      );
      if (picked) existingRow = all.find((r) => r.id === picked.match.id);
    }

    // Capacité : si update d'un Oui existant, retirer l'ancien effectif du compteur
    if (input.response === "oui") {
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
      let used = Number(sumRows[0]?.total ?? 0);
      if (existingRow && existingRow.response === "oui") {
        used -= existingRow.presentCount;
      }
      if (used + presentCount > page.maxTotalPersons) {
        const left = Math.max(0, page.maxTotalPersons - used);
        return {
          ok: false as const,
          error:
            left === 0
              ? "Il n'y a plus de places disponibles."
              : "Il ne reste que " + left + " place(s).",
          status: 409,
        };
      }
    }

    const now = new Date();
    const rsvpValues = {
      eleveFirstName,
      eleveLastName,
      eleveNameNorm,
      birthDate,
      eligibleId: eligibleId || existingRow?.eligibleId || null,
      response: input.response,
      presentCount,
      parentEmail,
      diploma,
      situationStatus,
      situationDetail,
      situationEstablishment,
      updatedAt: now,
    };
    if (existingRow) {
      const [updated] = await tx
        .update(invitationRsvp)
        .set(rsvpValues)
        .where(
          and(
            eq(invitationRsvp.etablissementId, etabId),
            eq(invitationRsvp.id, existingRow.id),
          ),
        )
        .returning();
      if (!updated) {
        return { ok: false as const, error: "Impossible de mettre à jour la réponse.", status: 500 };
      }
      return {
        ok: true as const,
        rsvp: mapRsvp(updated),
        page,
        updated: true,
      };
    }

    if (eligibleId) {
      const [upserted] = await tx
        .insert(invitationRsvp)
        .values({
          etablissementId: etabId,
          pageId: page.id,
          ...rsvpValues,
          eligibleId,
        })
        .onConflictDoUpdate({
          target: [invitationRsvp.pageId, invitationRsvp.eligibleId],
          set: rsvpValues,
        })
        .returning();
      return {
        ok: true as const,
        rsvp: mapRsvp(upserted),
        page,
        updated: true,
      };
    }

    const [inserted] = await tx
      .insert(invitationRsvp)
      .values({
        etablissementId: etabId,
        pageId: page.id,
        ...rsvpValues,
      })
      .returning();

    return {
      ok: true as const,
      rsvp: mapRsvp(inserted),
      page,
      updated: false,
    };
  });
}

export async function deleteInvitationRsvp(
  pageId: string,
  rsvpId: string,
  etablissementId?: string,
): Promise<boolean> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return false;
  const db = getDb();
  const deleted = await db
    .delete(invitationRsvp)
    .where(
      and(
        eq(invitationRsvp.etablissementId, etabId),
        eq(invitationRsvp.pageId, pageId),
        eq(invitationRsvp.id, rsvpId),
      ),
    )
    .returning({ id: invitationRsvp.id });
  return deleted.length > 0;
}

export async function countInvitationEligible(
  pageId: string,
  etablissementId?: string,
): Promise<number> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return 0;
  const db = getDb();
  const rows = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(invitationEligible)
    .where(
      and(eq(invitationEligible.etablissementId, etabId), eq(invitationEligible.pageId, pageId)),
    );
  return Number(rows[0]?.n ?? 0);
}

export async function listInvitationEligible(
  pageId: string,
  etablissementId?: string,
): Promise<InvitationEligibleRecord[]> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const rows = await db
    .select()
    .from(invitationEligible)
    .where(
      and(eq(invitationEligible.etablissementId, etabId), eq(invitationEligible.pageId, pageId)),
    )
    .orderBy(invitationEligible.eleveLastName, invitationEligible.eleveFirstName);
  return rows.map(mapEligible);
}

export type ImportEligibleRow = {
  eleveFirstName: string;
  eleveLastName: string;
  birthDate?: string | null;
  diploma?: InvitationDiploma | null;
};

export async function replaceInvitationEligibleBatch(
  pageId: string,
  rows: ImportEligibleRow[],
  mode: "append" | "replace",
  etablissementId?: string,
): Promise<{ inserted: number; skipped: number; total: number }> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const page = await getInvitationPageById(pageId, etabId);
  if (!page) throw new Error("Page introuvable.");
  const db = getDb();

  if (mode === "replace") {
    await db
      .delete(invitationEligible)
      .where(
        and(eq(invitationEligible.etablissementId, etabId), eq(invitationEligible.pageId, pageId)),
      );
  }

  const values: (typeof invitationEligible.$inferInsert)[] = [];
  const seen = new Set<string>();
  let skipped = 0;
  for (const raw of rows) {
    const eleveFirstName = formatInvitationFirstName(raw.eleveFirstName).slice(0, 80);
    const eleveLastName = formatInvitationLastName(raw.eleveLastName).slice(0, 80);
    if (!eleveFirstName || !eleveLastName) {
      skipped += 1;
      continue;
    }
    const birthDate = parseInvitationBirthDate(String(raw.birthDate || "")) ||
      birthDateToIso(raw.birthDate) ||
      null;
    const eleveNameNorm = normalizeEleveName(eleveFirstName, eleveLastName);
    const dedupeKey = `${eleveNameNorm}|${birthDate || ""}`;
    if (seen.has(dedupeKey)) {
      skipped += 1;
      continue;
    }
    seen.add(dedupeKey);
    values.push({
      etablissementId: etabId,
      pageId,
      eleveFirstName,
      eleveLastName,
      eleveNameNorm,
      birthDate,
      diploma: raw.diploma && isInvitationDiploma(raw.diploma) ? raw.diploma : null,
    });
  }

  if (values.length > 0) {
    await db.insert(invitationEligible).values(values);
  }

  const total = await countInvitationEligible(pageId, etabId);
  return { inserted: values.length, skipped, total };
}

export async function deleteInvitationEligible(
  pageId: string,
  eligibleId: string,
  etablissementId?: string,
): Promise<boolean> {
  const etabId = await requireEtabId(etablissementId);
  if (!isDatabaseConfigured()) return false;
  const db = getDb();
  const deleted = await db
    .delete(invitationEligible)
    .where(
      and(
        eq(invitationEligible.etablissementId, etabId),
        eq(invitationEligible.pageId, pageId),
        eq(invitationEligible.id, eligibleId),
      ),
    )
    .returning({ id: invitationEligible.id });
  return deleted.length > 0;
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
  return day + " à " + time;
}

export function invitationDiplomaDisplay(
  page: InvitationPageRecord,
  diploma: InvitationDiploma | null,
): string {
  if (page.diplomaMode === "none") return "";
  return diplomaLabel(diploma);
}

/** Réexport utilitaire pour l'UI admin. */
export { diplomaLabel, normalizeEleveName, slugifyInvitation };
