import "server-only";

import { eq, inArray, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { authUserMapping, user as userTable } from "@/db/schema";
import { loadAppConfig } from "@/app/lib/app-config";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import {
  capabilitiesFromFluxes,
  fluxesAssignedToUser,
  migrateLegacyUserSecteursToOcrFlux,
  type OcrResolvedFlux,
  type OcrUserCapabilities,
  OCR_FLUX_META,
} from "@/app/lib/ocr-flux";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import {
  DEFAULT_ONEDRIVE_BASE_BY_SECTEUR,
  type OneDriveUserProfile,
} from "@/app/lib/onedrive-user-profiles";

function defaultBaseBySecteur(
  establishments: Awaited<ReturnType<typeof loadAppConfig>>["establishments"],
): Record<Secteur, { basePath: string; label: string }> {
  const fallback: Record<Secteur, { basePath: string; label: string }> = {
    ecole: { ...DEFAULT_ONEDRIVE_BASE_BY_SECTEUR.ecole },
    college: { ...DEFAULT_ONEDRIVE_BASE_BY_SECTEUR.college },
    lycee: { ...DEFAULT_ONEDRIVE_BASE_BY_SECTEUR.lycee },
  };
  for (const e of getActiveEstablishments(establishments)) {
    const kind = inferEstablishmentKind(e);
    if (kind === "ecole" || kind === "college" || kind === "lycee") {
      fallback[kind] = { basePath: `Dossier élèves/${e.label}`, label: e.label };
    }
  }
  return fallback;
}

function normalizeMatch(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function collectUserIdentifiers(user: SessionLikeUser): string[] {
  const out: string[] = [];
  if (user.lastName?.trim()) out.push(normalizeMatch(user.lastName));
  if (user.fullName?.trim()) out.push(normalizeMatch(user.fullName));
  if (user.primaryEmailAddress?.emailAddress) {
    out.push(normalizeMatch(user.primaryEmailAddress.emailAddress));
  }
  for (const e of user.emailAddresses ?? []) {
    if (e.emailAddress) out.push(normalizeMatch(e.emailAddress));
  }
  return out;
}

function collectEmails(user: SessionLikeUser): string[] {
  return [
    user.primaryEmailAddress?.emailAddress,
    ...(user.emailAddresses?.map((e) => e.emailAddress) ?? []),
  ].filter((e): e is string => Boolean(e?.trim()));
}

/**
 * Élargit les ids utilisables pour rattacher un flux OCR :
 * id session (métier), id auth Better-Auth, external_user_id, mapping Clerk historique.
 * Sans ça, Paramètres peut afficher la personne via e-mail alors que /api/onedrive/profile
 * ne matche que sur un id obsolète.
 */
export async function expandOcrMatchIdsForUser(user: SessionLikeUser): Promise<string[]> {
  const ids = new Set<string>();
  const push = (value: string | null | undefined) => {
    const v = value?.trim();
    if (v) ids.add(v);
  };
  push(user.id);

  if (!isDatabaseConfigured()) return [...ids];

  try {
    const db = getDb();
    const emails = collectEmails(user)
      .map((e) => e.trim().toLowerCase())
      .filter(Boolean);

    const rows =
      emails.length > 0
        ? await db
            .select({
              id: userTable.id,
              externalUserId: userTable.externalUserId,
              etablissementId: userTable.etablissementId,
            })
            .from(userTable)
            .where(inArray(sql`lower(${userTable.email})`, emails))
        : [];

    // Repli : résoudre la ligne via l'id déjà connu (auth ou métier).
    if (rows.length === 0 && user.id?.trim()) {
      const sid = user.id.trim();
      const byId = await db
        .select({
          id: userTable.id,
          externalUserId: userTable.externalUserId,
          etablissementId: userTable.etablissementId,
        })
        .from(userTable)
        .where(eq(userTable.id, sid))
        .limit(1);
      if (byId[0]) rows.push(byId[0]);
      else {
        const byExternal = await db
          .select({
            id: userTable.id,
            externalUserId: userTable.externalUserId,
            etablissementId: userTable.etablissementId,
          })
          .from(userTable)
          .where(eq(userTable.externalUserId, sid))
          .limit(1);
        if (byExternal[0]) rows.push(byExternal[0]);
      }
    }

    const authIds: string[] = [];
    for (const row of rows) {
      push(row.id);
      push(row.externalUserId);
      authIds.push(row.id);
    }

    if (authIds.length > 0) {
      const mappings = await db
        .select({
          externalUserId: authUserMapping.externalUserId,
          userId: authUserMapping.userId,
        })
        .from(authUserMapping)
        .where(inArray(authUserMapping.userId, authIds));
      for (const m of mappings) {
        push(m.externalUserId);
        push(m.userId);
      }
    }

    // Mapping inverse : id session = ancien Clerk stocké comme external dans auth_user_mapping
    if (user.id?.trim()) {
      const reverse = await db
        .select({
          externalUserId: authUserMapping.externalUserId,
          userId: authUserMapping.userId,
        })
        .from(authUserMapping)
        .where(eq(authUserMapping.externalUserId, user.id.trim()));
      for (const m of reverse) {
        push(m.externalUserId);
        push(m.userId);
      }
    }
  } catch (error) {
    console.error("[expandOcrMatchIdsForUser]", error);
  }

  return [...ids];
}

async function loadOneDriveConfig() {
  try {
    const config = await loadAppConfig();
    return {
      od: config.integrations.microsoftOneDrive,
      establishments: config.establishments,
    };
  } catch {
    return { od: undefined, establishments: [] as Awaited<ReturnType<typeof loadAppConfig>>["establishments"] };
  }
}

function applyElevesPathDefaults(
  flux: OcrResolvedFlux,
  defaults: Record<Secteur, { basePath: string; label: string }>,
  basesBySecteur?: Partial<Record<Secteur, { basePath?: string; label?: string }>>,
): OcrResolvedFlux {
  if (flux.kind !== "eleves" || !flux.secteur) return flux;
  const assignedPath = flux.basePath?.trim();
  const override = basesBySecteur?.[flux.secteur]?.basePath?.trim();
  const establishmentDefault = defaults[flux.secteur]?.basePath;
  const metaDefault = OCR_FLUX_META[flux.id].defaultBasePath;
  const usedDefault = !assignedPath || assignedPath === metaDefault;
  return {
    ...flux,
    basePath: override || (usedDefault ? establishmentDefault || metaDefault : assignedPath),
    label: basesBySecteur?.[flux.secteur]?.label?.trim() || defaults[flux.secteur]?.label || flux.label,
  };
}

/**
 * Résout tous les flux OCR rattachés à l'utilisateur (ocrFlux, avec repli userSecteurs).
 */
export async function resolveOcrCapabilitiesForUserServer(
  user: SessionLikeUser,
): Promise<OcrUserCapabilities> {
  const { od, establishments } = await loadOneDriveConfig();
  const defaults = defaultBaseBySecteur(establishments);
  const grid = migrateLegacyUserSecteursToOcrFlux({
    ocrFlux: od?.ocrFlux,
    userSecteurs: od?.userSecteurs,
    basesBySecteur: od?.basesBySecteur,
    personnelBasePath: od?.rhDrive?.basePath,
  });
  const matchIds = await expandOcrMatchIdsForUser(user);
  const assigned = fluxesAssignedToUser(grid, {
    id: user.id,
    ids: matchIds,
    lastName: user.lastName,
    fullName: user.fullName,
    emails: collectEmails(user),
  }).map((flux) => applyElevesPathDefaults(flux, defaults, od?.basesBySecteur));
  return capabilitiesFromFluxes(assigned);
}

/**
 * Résout le profil OneDrive (dossier racine + cycle) d'un utilisateur.
 * Inchangé pour un secrétariat qui n'a qu'un flux élèves : même chemin, même secteur.
 */
export async function resolveOneDriveProfileForUserServer(
  user: SessionLikeUser,
): Promise<OneDriveUserProfile | null> {
  const caps = await resolveOcrCapabilitiesForUserServer(user);
  if (caps.primaryEleves) return caps.primaryEleves;

  let profile: OneDriveUserProfile | null = null;

  const { od, establishments } = await loadOneDriveConfig();
  const defaults = defaultBaseBySecteur(establishments);
  const matchIds = await expandOcrMatchIdsForUser(user);
  const idSet = new Set(matchIds);

  if (!profile && od?.userSecteurs?.length) {
    const byId = od.userSecteurs.find((m) => {
      const ext = m.externalUserId?.trim();
      return Boolean(ext && idSet.has(ext));
    });
    if (byId) {
      const def = defaults[byId.secteur];
      profile = { key: byId.secteur, secteur: byId.secteur, basePath: def.basePath, label: def.label };
    }
  }

  if (!profile && od?.userSecteurs?.length) {
    const identifiers = collectUserIdentifiers(user);
    const hit = od.userSecteurs.find((m) => {
      const target = normalizeMatch(m.match);
      if (!target) return false;
      if (idSet.has(target)) return true;
      return identifiers.some((id) => id === target || id.includes(target) || target.includes(id));
    });
    if (hit) {
      const def = defaults[hit.secteur];
      profile = { key: hit.secteur, secteur: hit.secteur, basePath: def.basePath, label: def.label };
    }
  }

  if (!profile) return null;

  const override = od?.basesBySecteur?.[profile.secteur];
  if (override?.basePath?.trim()) {
    profile = {
      ...profile,
      basePath: override.basePath.trim(),
      label: override.label?.trim() || profile.label,
    };
  }

  return profile;
}
