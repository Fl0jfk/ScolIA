import "server-only";

import { and, asc, count, desc, eq, inArray, ne, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  partenariatEvenement,
  partenariatInscription,
  partenariatOffre,
} from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { putObject, getSignedReadUrl } from "@/app/lib/s3-storage";
import { sanitizeS3FileName } from "@/app/lib/s3-path";
import {
  DEFAULT_ENGAGEMENT_TEXT,
  isInscriptionWindowOpen,
  isPartenariatInscriptionStatus,
  isPartenariatKind,
  sanitizeCtaLinks,
  sanitizeCycles,
  sanitizeNiveaux,
  sanitizeTarifs,
  slugifyPartenariat,
  type PartenariatCtaLink,
  type PartenariatEvenementRecord,
  type PartenariatInscriptionRecord,
  type PartenariatInscriptionStatus,
  type PartenariatKind,
  type PartenariatOffrePublicCard,
  type PartenariatOffrePublicDetail,
  type PartenariatOffreRecord,
  type PartenariatTarif,
} from "@/app/lib/partenariats-types";
import { loadAppConfig } from "@/app/lib/app-config";

async function requireEtabId(explicit?: string): Promise<string> {
  const id = explicit || (await resolveCurrentEtablissementId());
  if (!id) throw new Error("Établissement introuvable (tenant).");
  return id;
}

function toIso(d: Date | null | undefined): string | null {
  if (!d) return null;
  return d.toISOString();
}

function parseOptionalDate(raw: string | null | undefined): Date | null {
  if (raw == null || raw === "") return null;
  const t = Date.parse(raw);
  if (!Number.isFinite(t)) return null;
  return new Date(t);
}

async function logoUrlForKey(key: string | null | undefined): Promise<string | null> {
  if (!key?.trim()) return null;
  try {
    return (await getSignedReadUrl(key, 3600)) || null;
  } catch {
    return null;
  }
}

function mapOffre(
  row: typeof partenariatOffre.$inferSelect,
  logoUrl: string | null = null,
): PartenariatOffreRecord {
  const kind: PartenariatKind = isPartenariatKind(row.kind) ? row.kind : "info";
  return {
    id: row.id,
    etablissementId: row.etablissementId,
    slug: row.slug,
    title: row.title,
    shortDescription: row.shortDescription || "",
    body: row.body || "",
    logoS3Key: row.logoS3Key || null,
    logoUrl,
    kind,
    cycles: sanitizeCycles(row.cycles),
    niveaux: sanitizeNiveaux(row.niveaux),
    categoryLabel: row.categoryLabel || "",
    contactName: row.contactName || "",
    contactRole: row.contactRole || "",
    contactEmail: row.contactEmail || "",
    contactPhone: row.contactPhone || "",
    partnerContactName: row.partnerContactName || "",
    partnerContactRole: row.partnerContactRole || "",
    partnerContactEmail: row.partnerContactEmail || "",
    partnerContactPhone: row.partnerContactPhone || "",
    ctaLinks: sanitizeCtaLinks(row.ctaLinks),
    tarifs: sanitizeTarifs(row.tarifs),
    demarche: row.demarche || "",
    engagementText: row.engagementText || DEFAULT_ENGAGEMENT_TEXT,
    requireSignature: row.requireSignature === 1,
    notifyEmail: row.notifyEmail?.trim() || null,
    maxPlaces: row.maxPlaces ?? null,
    inscriptionOpensAt: toIso(row.inscriptionOpensAt),
    inscriptionClosesAt: toIso(row.inscriptionClosesAt),
    enabled: row.enabled === 1,
    sortOrder: row.sortOrder,
    publishedAt: toIso(row.publishedAt),
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
    updatedAt: toIso(row.updatedAt) || new Date().toISOString(),
  };
}

function mapEvenement(
  row: typeof partenariatEvenement.$inferSelect,
): PartenariatEvenementRecord {
  return {
    id: row.id,
    etablissementId: row.etablissementId,
    offreId: row.offreId,
    title: row.title,
    startsAt: toIso(row.startsAt) || new Date().toISOString(),
    endsAt: toIso(row.endsAt),
    location: row.location || "",
    notes: row.notes || "",
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
    updatedAt: toIso(row.updatedAt) || new Date().toISOString(),
  };
}

function mapInscription(
  row: typeof partenariatInscription.$inferSelect,
): PartenariatInscriptionRecord {
  const status: PartenariatInscriptionStatus = isPartenariatInscriptionStatus(row.status)
    ? row.status
    : "soumise";
  return {
    id: row.id,
    etablissementId: row.etablissementId,
    offreId: row.offreId,
    eleveFirstName: row.eleveFirstName,
    eleveLastName: row.eleveLastName,
    eleveNiveau: row.eleveNiveau || "",
    eleveClasse: row.eleveClasse || "",
    eleveBirthDate: row.eleveBirthDate || null,
    parentFirstName: row.parentFirstName,
    parentLastName: row.parentLastName,
    parentEmail: row.parentEmail,
    parentPhone: row.parentPhone || "",
    engagementAcceptedAt: toIso(row.engagementAcceptedAt) || new Date().toISOString(),
    signatureS3Key: row.signatureS3Key || null,
    status,
    adminNote: row.adminNote || "",
    createdAt: toIso(row.createdAt) || new Date().toISOString(),
    updatedAt: toIso(row.updatedAt) || new Date().toISOString(),
  };
}

async function uniqueSlug(etabId: string, base: string, excludeId?: string): Promise<string> {
  const db = getDb();
  let candidate = slugifyPartenariat(base);
  for (let i = 0; i < 40; i++) {
    const trySlug = i === 0 ? candidate : `${candidate}-${i + 1}`;
    const rows = await db
      .select({ id: partenariatOffre.id })
      .from(partenariatOffre)
      .where(
        and(
          eq(partenariatOffre.etablissementId, etabId),
          eq(partenariatOffre.slug, trySlug),
        ),
      )
      .limit(1);
    if (!rows[0] || (excludeId && rows[0].id === excludeId)) return trySlug;
  }
  return `${candidate}-${Date.now().toString(36)}`;
}

export async function countActiveInscriptions(offreId: string): Promise<number> {
  if (!isDatabaseConfigured()) return 0;
  const db = getDb();
  const rows = await db
    .select({ n: count() })
    .from(partenariatInscription)
    .where(
      and(
        eq(partenariatInscription.offreId, offreId),
        inArray(partenariatInscription.status, ["soumise", "validee"]),
      ),
    );
  return Number(rows[0]?.n || 0);
}

export async function listPartenariatOffres(
  etablissementId?: string,
): Promise<PartenariatOffreRecord[]> {
  if (!isDatabaseConfigured()) return [];
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(partenariatOffre)
    .where(eq(partenariatOffre.etablissementId, etabId))
    .orderBy(asc(partenariatOffre.sortOrder), desc(partenariatOffre.createdAt));

  const ids = rows.map((r) => r.id);
  const inscCounts = new Map<string, number>();
  if (ids.length) {
    const counts = await db
      .select({
        offreId: partenariatInscription.offreId,
        n: count(),
      })
      .from(partenariatInscription)
      .where(
        and(
          eq(partenariatInscription.etablissementId, etabId),
          inArray(partenariatInscription.offreId, ids),
          inArray(partenariatInscription.status, ["soumise", "validee"]),
        ),
      )
      .groupBy(partenariatInscription.offreId);
    for (const c of counts) inscCounts.set(c.offreId, Number(c.n));
  }

  const out: PartenariatOffreRecord[] = [];
  for (const row of rows) {
    const logoUrl = await logoUrlForKey(row.logoS3Key);
    const mapped = mapOffre(row, logoUrl);
    const n = inscCounts.get(row.id) || 0;
    mapped.inscriptionsCount = n;
    mapped.placesRemaining =
      mapped.maxPlaces != null ? Math.max(0, mapped.maxPlaces - n) : null;
    out.push(mapped);
  }
  return out;
}

export async function getPartenariatOffreById(
  offreId: string,
  etablissementId?: string,
): Promise<PartenariatOffreRecord | null> {
  if (!isDatabaseConfigured()) return null;
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(partenariatOffre)
    .where(
      and(eq(partenariatOffre.id, offreId), eq(partenariatOffre.etablissementId, etabId)),
    )
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  const logoUrl = await logoUrlForKey(row.logoS3Key);
  const mapped = mapOffre(row, logoUrl);
  const n = await countActiveInscriptions(row.id);
  mapped.inscriptionsCount = n;
  mapped.placesRemaining =
    mapped.maxPlaces != null ? Math.max(0, mapped.maxPlaces - n) : null;
  return mapped;
}

export async function getPartenariatOffreBySlug(
  slug: string,
  etablissementId?: string,
): Promise<PartenariatOffreRecord | null> {
  if (!isDatabaseConfigured()) return null;
  const etabId = await requireEtabId(etablissementId);
  const db = getDb();
  const rows = await db
    .select()
    .from(partenariatOffre)
    .where(
      and(
        eq(partenariatOffre.etablissementId, etabId),
        eq(partenariatOffre.slug, slugifyPartenariat(slug)),
      ),
    )
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  const logoUrl = await logoUrlForKey(row.logoS3Key);
  const mapped = mapOffre(row, logoUrl);
  const n = await countActiveInscriptions(row.id);
  mapped.inscriptionsCount = n;
  mapped.placesRemaining =
    mapped.maxPlaces != null ? Math.max(0, mapped.maxPlaces - n) : null;
  return mapped;
}

export type CreatePartenariatOffreInput = {
  title: string;
  slug?: string;
  shortDescription?: string;
  body?: string;
  kind?: PartenariatKind;
  cycles?: string[];
  niveaux?: string[];
  categoryLabel?: string;
  enabled?: boolean;
};

export async function createPartenariatOffre(
  input: CreatePartenariatOffreInput,
): Promise<PartenariatOffreRecord> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const etabId = await requireEtabId();
  const db = getDb();
  const title = input.title.trim().slice(0, 200) || "Nouvelle offre";
  const slug = await uniqueSlug(etabId, input.slug?.trim() || title);
  const kind: PartenariatKind = isPartenariatKind(input.kind) ? input.kind : "info";
  const cycles = sanitizeCycles(input.cycles?.length ? input.cycles : ["college", "lycee"]);
  const niveaux = sanitizeNiveaux(input.niveaux || []);
  const enabled = input.enabled === true ? 1 : 0;

  const maxSort = await db
    .select({ m: sql<number>`coalesce(max(${partenariatOffre.sortOrder}), 0)` })
    .from(partenariatOffre)
    .where(eq(partenariatOffre.etablissementId, etabId));
  const sortOrder = Number(maxSort[0]?.m || 0) + 10;

  const [row] = await db
    .insert(partenariatOffre)
    .values({
      etablissementId: etabId,
      slug,
      title,
      shortDescription: (input.shortDescription || "").trim().slice(0, 500),
      body: (input.body || "").trim().slice(0, 20000),
      kind,
      cycles,
      niveaux,
      categoryLabel: (input.categoryLabel || "").trim().slice(0, 80),
      engagementText: DEFAULT_ENGAGEMENT_TEXT,
      requireSignature: 1,
      enabled,
      sortOrder,
      publishedAt: enabled ? new Date() : null,
      updatedAt: new Date(),
    })
    .returning();

  return mapOffre(row, null);
}

export type UpdatePartenariatOffreInput = {
  title?: string;
  slug?: string;
  shortDescription?: string;
  body?: string;
  logoS3Key?: string | null;
  kind?: PartenariatKind;
  cycles?: string[];
  niveaux?: string[];
  categoryLabel?: string;
  contactName?: string;
  contactRole?: string;
  contactEmail?: string;
  contactPhone?: string;
  partnerContactName?: string;
  partnerContactRole?: string;
  partnerContactEmail?: string;
  partnerContactPhone?: string;
  ctaLinks?: PartenariatCtaLink[];
  tarifs?: PartenariatTarif[];
  demarche?: string;
  engagementText?: string;
  requireSignature?: boolean;
  notifyEmail?: string | null;
  maxPlaces?: number | null;
  inscriptionOpensAt?: string | null;
  inscriptionClosesAt?: string | null;
  enabled?: boolean;
  sortOrder?: number;
};

export async function updatePartenariatOffre(
  offreId: string,
  input: UpdatePartenariatOffreInput,
): Promise<PartenariatOffreRecord> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const etabId = await requireEtabId();
  const existing = await getPartenariatOffreById(offreId, etabId);
  if (!existing) throw new Error("Offre introuvable.");

  const db = getDb();
  const patch: Partial<typeof partenariatOffre.$inferInsert> = {
    updatedAt: new Date(),
  };

  if (input.title !== undefined) patch.title = input.title.trim().slice(0, 200) || existing.title;
  if (input.slug !== undefined) {
    patch.slug = await uniqueSlug(etabId, input.slug.trim() || existing.title, offreId);
  }
  if (input.shortDescription !== undefined) {
    patch.shortDescription = input.shortDescription.trim().slice(0, 500);
  }
  if (input.body !== undefined) patch.body = input.body.trim().slice(0, 20000);
  if (input.logoS3Key !== undefined) patch.logoS3Key = input.logoS3Key?.trim() || null;
  if (input.kind !== undefined && isPartenariatKind(input.kind)) patch.kind = input.kind;
  if (input.cycles !== undefined) patch.cycles = sanitizeCycles(input.cycles);
  if (input.niveaux !== undefined) patch.niveaux = sanitizeNiveaux(input.niveaux);
  if (input.categoryLabel !== undefined) {
    patch.categoryLabel = input.categoryLabel.trim().slice(0, 80);
  }
  if (input.contactName !== undefined) patch.contactName = input.contactName.trim().slice(0, 120);
  if (input.contactRole !== undefined) patch.contactRole = input.contactRole.trim().slice(0, 120);
  if (input.contactEmail !== undefined) {
    patch.contactEmail = input.contactEmail.trim().slice(0, 200);
  }
  if (input.contactPhone !== undefined) {
    patch.contactPhone = input.contactPhone.trim().slice(0, 40);
  }
  if (input.partnerContactName !== undefined) {
    patch.partnerContactName = input.partnerContactName.trim().slice(0, 120);
  }
  if (input.partnerContactRole !== undefined) {
    patch.partnerContactRole = input.partnerContactRole.trim().slice(0, 120);
  }
  if (input.partnerContactEmail !== undefined) {
    patch.partnerContactEmail = input.partnerContactEmail.trim().slice(0, 200);
  }
  if (input.partnerContactPhone !== undefined) {
    patch.partnerContactPhone = input.partnerContactPhone.trim().slice(0, 40);
  }
  if (input.ctaLinks !== undefined) patch.ctaLinks = sanitizeCtaLinks(input.ctaLinks);
  if (input.tarifs !== undefined) patch.tarifs = sanitizeTarifs(input.tarifs);
  if (input.demarche !== undefined) patch.demarche = input.demarche.trim().slice(0, 8000);
  if (input.engagementText !== undefined) {
    patch.engagementText =
      input.engagementText.trim().slice(0, 2000) || DEFAULT_ENGAGEMENT_TEXT;
  }
  if (input.requireSignature !== undefined) {
    patch.requireSignature = input.requireSignature ? 1 : 0;
  }
  if (input.notifyEmail !== undefined) {
    patch.notifyEmail = input.notifyEmail?.trim().slice(0, 200) || null;
  }
  if (input.maxPlaces !== undefined) {
    patch.maxPlaces =
      input.maxPlaces == null || input.maxPlaces <= 0 ? null : Math.min(50000, Math.round(input.maxPlaces));
  }
  if (input.inscriptionOpensAt !== undefined) {
    patch.inscriptionOpensAt = parseOptionalDate(input.inscriptionOpensAt);
  }
  if (input.inscriptionClosesAt !== undefined) {
    patch.inscriptionClosesAt = parseOptionalDate(input.inscriptionClosesAt);
  }
  if (input.sortOrder !== undefined && Number.isFinite(input.sortOrder)) {
    patch.sortOrder = Math.round(input.sortOrder);
  }
  if (input.enabled !== undefined) {
    patch.enabled = input.enabled ? 1 : 0;
    if (input.enabled && !existing.enabled) patch.publishedAt = new Date();
  }

  await db
    .update(partenariatOffre)
    .set(patch)
    .where(
      and(eq(partenariatOffre.id, offreId), eq(partenariatOffre.etablissementId, etabId)),
    );

  const updated = await getPartenariatOffreById(offreId, etabId);
  if (!updated) throw new Error("Offre introuvable après mise à jour.");
  return updated;
}

export async function deletePartenariatOffre(offreId: string): Promise<void> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const etabId = await requireEtabId();
  const db = getDb();
  await db
    .delete(partenariatOffre)
    .where(
      and(eq(partenariatOffre.id, offreId), eq(partenariatOffre.etablissementId, etabId)),
    );
}

export async function listPartenariatEvenements(
  offreId: string,
): Promise<PartenariatEvenementRecord[]> {
  if (!isDatabaseConfigured()) return [];
  const etabId = await requireEtabId();
  const db = getDb();
  const rows = await db
    .select()
    .from(partenariatEvenement)
    .where(
      and(
        eq(partenariatEvenement.etablissementId, etabId),
        eq(partenariatEvenement.offreId, offreId),
      ),
    )
    .orderBy(asc(partenariatEvenement.startsAt));
  return rows.map(mapEvenement);
}

export async function createPartenariatEvenement(input: {
  offreId: string;
  title: string;
  startsAt: string;
  endsAt?: string | null;
  location?: string;
  notes?: string;
}): Promise<PartenariatEvenementRecord> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const etabId = await requireEtabId();
  const offre = await getPartenariatOffreById(input.offreId, etabId);
  if (!offre) throw new Error("Offre introuvable.");
  const startsAt = parseOptionalDate(input.startsAt);
  if (!startsAt) throw new Error("Date de début invalide.");
  const db = getDb();
  const [row] = await db
    .insert(partenariatEvenement)
    .values({
      etablissementId: etabId,
      offreId: input.offreId,
      title: input.title.trim().slice(0, 200) || "Réunion",
      startsAt,
      endsAt: parseOptionalDate(input.endsAt),
      location: (input.location || "").trim().slice(0, 300),
      notes: (input.notes || "").trim().slice(0, 2000),
      updatedAt: new Date(),
    })
    .returning();
  return mapEvenement(row);
}

export async function updatePartenariatEvenement(
  eventId: string,
  input: {
    title?: string;
    startsAt?: string;
    endsAt?: string | null;
    location?: string;
    notes?: string;
  },
): Promise<PartenariatEvenementRecord> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const etabId = await requireEtabId();
  const db = getDb();
  const existing = await db
    .select()
    .from(partenariatEvenement)
    .where(
      and(
        eq(partenariatEvenement.id, eventId),
        eq(partenariatEvenement.etablissementId, etabId),
      ),
    )
    .limit(1);
  if (!existing[0]) throw new Error("Événement introuvable.");

  const patch: Partial<typeof partenariatEvenement.$inferInsert> = {
    updatedAt: new Date(),
  };
  if (input.title !== undefined) patch.title = input.title.trim().slice(0, 200) || existing[0].title;
  if (input.startsAt !== undefined) {
    const d = parseOptionalDate(input.startsAt);
    if (!d) throw new Error("Date de début invalide.");
    patch.startsAt = d;
  }
  if (input.endsAt !== undefined) patch.endsAt = parseOptionalDate(input.endsAt);
  if (input.location !== undefined) patch.location = input.location.trim().slice(0, 300);
  if (input.notes !== undefined) patch.notes = input.notes.trim().slice(0, 2000);

  const [row] = await db
    .update(partenariatEvenement)
    .set(patch)
    .where(
      and(
        eq(partenariatEvenement.id, eventId),
        eq(partenariatEvenement.etablissementId, etabId),
      ),
    )
    .returning();
  return mapEvenement(row);
}

export async function deletePartenariatEvenement(eventId: string): Promise<void> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const etabId = await requireEtabId();
  const db = getDb();
  await db
    .delete(partenariatEvenement)
    .where(
      and(
        eq(partenariatEvenement.id, eventId),
        eq(partenariatEvenement.etablissementId, etabId),
      ),
    );
}

export async function listPartenariatInscriptions(
  offreId: string,
): Promise<PartenariatInscriptionRecord[]> {
  if (!isDatabaseConfigured()) return [];
  const etabId = await requireEtabId();
  const db = getDb();
  const rows = await db
    .select()
    .from(partenariatInscription)
    .where(
      and(
        eq(partenariatInscription.etablissementId, etabId),
        eq(partenariatInscription.offreId, offreId),
        ne(partenariatInscription.status, "annulee"),
      ),
    )
    .orderBy(desc(partenariatInscription.createdAt));
  return rows.map(mapInscription);
}

export async function updatePartenariatInscriptionStatus(
  inscriptionId: string,
  status: PartenariatInscriptionStatus,
  adminNote?: string,
): Promise<PartenariatInscriptionRecord> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  if (!isPartenariatInscriptionStatus(status)) throw new Error("Statut invalide.");
  const etabId = await requireEtabId();
  const db = getDb();
  const patch: Partial<typeof partenariatInscription.$inferInsert> = {
    status,
    updatedAt: new Date(),
  };
  if (adminNote !== undefined) patch.adminNote = adminNote.trim().slice(0, 2000);

  const [row] = await db
    .update(partenariatInscription)
    .set(patch)
    .where(
      and(
        eq(partenariatInscription.id, inscriptionId),
        eq(partenariatInscription.etablissementId, etabId),
      ),
    )
    .returning();
  if (!row) throw new Error("Inscription introuvable.");
  return mapInscription(row);
}

export async function savePartenariatLogo(
  offreId: string,
  fileName: string,
  buffer: Buffer,
  contentType: string,
): Promise<{ key: string; url: string | null }> {
  const etabId = await requireEtabId();
  const offre = await getPartenariatOffreById(offreId, etabId);
  if (!offre) throw new Error("Offre introuvable.");
  const safe = sanitizeS3FileName(fileName);
  const key = `partenariats/logos/${offreId}/${Date.now()}-${safe}`;
  await putObject(key, buffer, contentType);
  await updatePartenariatOffre(offreId, { logoS3Key: key });
  const url = await logoUrlForKey(key);
  return { key, url };
}

export async function savePartenariatSignature(
  offreId: string,
  inscriptionId: string,
  dataUrl: string,
): Promise<string> {
  const match = /^data:image\/(png|jpeg|jpg);base64,(.+)$/i.exec(dataUrl.trim());
  if (!match) throw new Error("Signature invalide.");
  const ext = match[1].toLowerCase() === "png" ? "png" : "jpg";
  const contentType = ext === "png" ? "image/png" : "image/jpeg";
  const buffer = Buffer.from(match[2], "base64");
  if (buffer.length > 2 * 1024 * 1024) throw new Error("Signature trop volumineuse.");
  const key = `partenariats/signatures/${offreId}/${inscriptionId}.${ext}`;
  await putObject(key, buffer, contentType);
  return key;
}

export type SubmitPartenariatInscriptionInput = {
  eleveFirstName: string;
  eleveLastName: string;
  eleveNiveau?: string;
  eleveClasse?: string;
  eleveBirthDate?: string | null;
  parentFirstName: string;
  parentLastName: string;
  parentEmail: string;
  parentPhone?: string;
  engagementAccepted: boolean;
  signatureDataUrl?: string | null;
};

export async function submitPartenariatInscription(
  slug: string,
  input: SubmitPartenariatInscriptionInput,
): Promise<PartenariatInscriptionRecord> {
  if (!isDatabaseConfigured()) throw new Error("Base de données non configurée.");
  const etabId = await requireEtabId();
  const offre = await getPartenariatOffreBySlug(slug, etabId);
  if (!offre || !offre.enabled) throw new Error("Offre introuvable.");
  if (offre.kind !== "inscription") {
    throw new Error("Cette offre n’accepte pas d’inscription en ligne.");
  }
  if (!isInscriptionWindowOpen(offre)) {
    throw new Error("Les inscriptions sont fermées pour cette offre.");
  }
  if (!input.engagementAccepted) {
    throw new Error("Vous devez accepter l’engagement.");
  }
  if (offre.requireSignature && !input.signatureDataUrl?.trim()) {
    throw new Error("La signature est obligatoire.");
  }

  const eleveFirstName = input.eleveFirstName.trim().slice(0, 80);
  const eleveLastName = input.eleveLastName.trim().slice(0, 80);
  const parentFirstName = input.parentFirstName.trim().slice(0, 80);
  const parentLastName = input.parentLastName.trim().slice(0, 80);
  const parentEmail = input.parentEmail.trim().toLowerCase().slice(0, 200);
  if (!eleveFirstName || !eleveLastName || !parentFirstName || !parentLastName || !parentEmail) {
    throw new Error("Champs obligatoires manquants.");
  }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(parentEmail)) {
    throw new Error("E-mail parent invalide.");
  }

  const db = getDb();
  const [row] = await db
    .insert(partenariatInscription)
    .values({
      etablissementId: etabId,
      offreId: offre.id,
      eleveFirstName,
      eleveLastName,
      eleveNiveau: (input.eleveNiveau || "").trim().slice(0, 40),
      eleveClasse: (input.eleveClasse || "").trim().slice(0, 40),
      eleveBirthDate: input.eleveBirthDate?.trim().slice(0, 10) || null,
      parentFirstName,
      parentLastName,
      parentEmail,
      parentPhone: (input.parentPhone || "").trim().slice(0, 40),
      engagementAcceptedAt: new Date(),
      status: "soumise",
      updatedAt: new Date(),
    })
    .returning();

  let signatureKey: string | null = null;
  if (input.signatureDataUrl?.trim()) {
    try {
      signatureKey = await savePartenariatSignature(offre.id, row.id, input.signatureDataUrl);
      await db
        .update(partenariatInscription)
        .set({ signatureS3Key: signatureKey, updatedAt: new Date() })
        .where(eq(partenariatInscription.id, row.id));
    } catch (e) {
      await db
        .update(partenariatInscription)
        .set({ status: "annulee", updatedAt: new Date() })
        .where(eq(partenariatInscription.id, row.id));
      throw e;
    }
  }

  return mapInscription({ ...row, signatureS3Key: signatureKey });
}

export async function listPublicPartenariatCards(): Promise<{
  schoolName: string;
  offres: PartenariatOffrePublicCard[];
}> {
  let schoolName = "";
  try {
    const bundle = await loadAppConfig();
    schoolName = bundle.identity.shortName || bundle.identity.name || "";
  } catch {
    schoolName = "";
  }
  if (!isDatabaseConfigured()) return { schoolName, offres: [] };
  try {
    const etabId = await requireEtabId();
    const db = getDb();
    const rows = await db
      .select()
      .from(partenariatOffre)
      .where(
        and(eq(partenariatOffre.etablissementId, etabId), eq(partenariatOffre.enabled, 1)),
      )
      .orderBy(asc(partenariatOffre.sortOrder), desc(partenariatOffre.createdAt));

    const offres: PartenariatOffrePublicCard[] = [];
    for (const row of rows) {
      const mapped = mapOffre(row, await logoUrlForKey(row.logoS3Key));
      offres.push({
        slug: mapped.slug,
        title: mapped.title,
        shortDescription: mapped.shortDescription,
        logoUrl: mapped.logoUrl,
        kind: mapped.kind,
        cycles: mapped.cycles,
        niveaux: mapped.niveaux,
        categoryLabel: mapped.categoryLabel,
      });
    }
    return { schoolName, offres };
  } catch (e) {
    console.warn("[partenariats] listPublicPartenariatCards unavailable:", e instanceof Error ? e.message : e);
    return { schoolName, offres: [] };
  }
}

export async function getPublicPartenariatDetail(
  slug: string,
): Promise<PartenariatOffrePublicDetail | null> {
  try {
    const offre = await getPartenariatOffreBySlug(slug);
    if (!offre || !offre.enabled) return null;
    let schoolName = "";
    try {
      const bundle = await loadAppConfig();
      schoolName = bundle.identity.shortName || bundle.identity.name || "";
    } catch {
      schoolName = "";
    }
    const events = await listPartenariatEvenements(offre.id);
    const now = Date.now();
    const upcoming = events.filter((e) => Date.parse(e.startsAt) >= now - 2 * 60 * 60 * 1000);

    return {
      slug: offre.slug,
      title: offre.title,
      shortDescription: offre.shortDescription,
      logoUrl: offre.logoUrl,
      kind: offre.kind,
      cycles: offre.cycles,
      niveaux: offre.niveaux,
      categoryLabel: offre.categoryLabel,
      body: offre.body,
      contactName: offre.contactName,
      contactRole: offre.contactRole,
      contactEmail: offre.contactEmail,
      contactPhone: offre.contactPhone,
      partnerContactName: offre.partnerContactName,
      partnerContactRole: offre.partnerContactRole,
      partnerContactEmail: offre.partnerContactEmail,
      partnerContactPhone: offre.partnerContactPhone,
      ctaLinks: offre.ctaLinks,
      tarifs: offre.tarifs,
      demarche: offre.demarche,
      engagementText: offre.engagementText,
      requireSignature: offre.requireSignature,
      maxPlaces: offre.maxPlaces,
      placesRemaining: offre.placesRemaining ?? null,
      inscriptionOpen: isInscriptionWindowOpen(offre),
      inscriptionOpensAt: offre.inscriptionOpensAt,
      inscriptionClosesAt: offre.inscriptionClosesAt,
      schoolName,
      evenements: upcoming.map((e) => ({
        id: e.id,
        title: e.title,
        startsAt: e.startsAt,
        endsAt: e.endsAt,
        location: e.location,
        notes: e.notes,
      })),
    };
  } catch (e) {
    console.warn("[partenariats] getPublicPartenariatDetail unavailable:", e instanceof Error ? e.message : e);
    return null;
  }
}

export function inscriptionsToCsv(rows: PartenariatInscriptionRecord[]): string {
  const header = [
    "id",
    "statut",
    "eleve_prenom",
    "eleve_nom",
    "eleve_niveau",
    "eleve_classe",
    "eleve_naissance",
    "parent_prenom",
    "parent_nom",
    "parent_email",
    "parent_tel",
    "engagement_at",
    "created_at",
    "note_admin",
  ];
  const escape = (v: string) => `"${v.replace(/"/g, '""')}"`;
  const lines = [header.join(";")];
  for (const r of rows) {
    lines.push(
      [
        r.id,
        r.status,
        r.eleveFirstName,
        r.eleveLastName,
        r.eleveNiveau,
        r.eleveClasse,
        r.eleveBirthDate || "",
        r.parentFirstName,
        r.parentLastName,
        r.parentEmail,
        r.parentPhone,
        r.engagementAcceptedAt,
        r.createdAt,
        r.adminNote,
      ]
        .map((c) => escape(String(c)))
        .join(";"),
    );
  }
  return lines.join("\n");
}
