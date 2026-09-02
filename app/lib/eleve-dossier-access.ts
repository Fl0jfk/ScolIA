import "server-only";

import { and, desc, eq, gt, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  documentAccessRequest,
  eleveAccessAudit,
  eleveDocument,
  type EleveDocumentRow,
} from "@/db/schema";
import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import {
  tiroirsForCategories,
  type EleveDocCategorie,
} from "@/app/lib/eleve-doc-categories";
import { isPapDocumentTitle } from "@/app/lib/eleve-pap";

function isExactAdmin(roles: string[]): boolean {
  return roles.includes("admin") || hasGlobalAdminRole(roles);
}

function isDirection(roles: string[]): boolean {
  return INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug));
}

export type EleveDossierSection =
  | "identite"
  | "scolarite"
  | "famille"
  | "documents"
  | "notes"
  | "vie_scolaire"
  | "sante"
  | "facturation";

export type EleveDocTiroir =
  | "scolaire"
  | "inscription"
  | "facturation"
  | "voyages"
  | "sante"
  | "vie_scolaire"
  | "orientation";

export type EleveDocConfidentialite = "standard" | "restreint" | "sante";

/** Catégories documents visibles nativement pour le rôle. */
export function eleveDocCategoriesForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): Set<EleveDocCategorie> {
  if (opts?.platformAdmin || opts?.orgAdmin || isExactAdmin(roles) || isDirection(roles)) {
    return new Set<EleveDocCategorie>(["administratif", "financier", "sante"]);
  }
  const out = new Set<EleveDocCategorie>();
  if (hasRole(roles, "administratif")) {
    out.add("administratif");
    out.add("financier");
    out.add("sante"); // dépôt / consultation PAP (et docs santé standard)
  }
  if (hasRole(roles, "comptabilite")) {
    out.add("financier");
  }
  if (hasRole(roles, "infirmerie") || hasRole(roles, "psychologue")) {
    out.add("sante");
  }
  if (hasRole(roles, "cpe") || hasRole(roles, "surveillant")) {
    out.add("administratif");
  }
  if (hasRole(roles, "professeur")) {
    out.add("administratif");
    out.add("sante"); // PAP visible (filtré à la liste / ouverture)
  }
  return out;
}

/** Sections visibles sur la fiche selon les rôles intranet. */
export function eleveDossierSectionsForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): Set<EleveDossierSection> {
  const out = new Set<EleveDossierSection>(["identite", "scolarite"]);
  if (opts?.platformAdmin || opts?.orgAdmin || isExactAdmin(roles)) {
    return new Set([
      "identite",
      "scolarite",
      "famille",
      "documents",
      "notes",
      "vie_scolaire",
      "sante",
      "facturation",
    ]);
  }
  if (isDirection(roles)) {
    return new Set([
      "identite",
      "scolarite",
      "famille",
      "documents",
      "notes",
      "vie_scolaire",
      "sante",
      "facturation",
    ]);
  }
  if (hasRole(roles, "professeur")) {
    out.add("notes");
    out.add("documents");
    out.add("vie_scolaire");
  }
  if (hasRole(roles, "cpe")) {
    out.add("vie_scolaire");
    out.add("documents");
    out.add("famille");
    out.add("notes");
  }
  if (hasRole(roles, "surveillant")) {
    out.add("famille");
  }
  if (hasRole(roles, "infirmerie")) {
    out.add("sante");
    out.add("famille");
    out.add("documents");
  }
  if (hasRole(roles, "comptabilite")) {
    out.add("famille");
    out.add("documents");
    out.add("facturation");
  }
  if (hasRole(roles, "administratif")) {
    out.add("famille");
    out.add("documents");
    out.add("facturation");
    out.add("notes");
    out.add("vie_scolaire");
  }
  return out;
}

/** Tiroirs documents accessibles nativement (sans demande). */
export function eleveDocTiroirsForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): Set<EleveDocTiroir> {
  const categories = eleveDocCategoriesForRoles(roles, opts);
  const tiroirs = new Set<EleveDocTiroir>(
    tiroirsForCategories(categories) as EleveDocTiroir[],
  );

  // Affinages métier dans la catégorie administratif
  if (hasRole(roles, "professeur") && !isDirection(roles) && !opts?.orgAdmin) {
    // Prof : scolaire + voyages + santé (PAP uniquement à l’ouverture / liste).
    return new Set<EleveDocTiroir>(["scolaire", "voyages", "sante"]);
  }
  if (hasRole(roles, "cpe") || hasRole(roles, "surveillant")) {
    tiroirs.add("vie_scolaire");
    tiroirs.add("scolaire");
  }

  return tiroirs;
}

export function eleveDocCategoriesMetaForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): EleveDocCategorie[] {
  const allowed = eleveDocCategoriesForRoles(roles, opts);
  return (["administratif", "financier", "sante"] as EleveDocCategorie[]).filter((c) =>
    allowed.has(c),
  );
}

/** Enregistrement d’un document (upload) selon tiroir et confidentialité. */
export function canRegisterEleveDocument(
  tiroir: EleveDocTiroir,
  confidentialite: EleveDocConfidentialite,
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  const allowedTiroirs = eleveDocTiroirsForRoles(roles, opts);
  if (!allowedTiroirs.has(tiroir)) return false;
  if (
    opts?.orgAdmin ||
    opts?.platformAdmin ||
    isExactAdmin(roles) ||
    isDirection(roles)
  ) {
    return true;
  }
  if (confidentialite === "restreint") return false;
  if (confidentialite === "sante") {
    return hasRole(roles, "infirmerie");
  }
  // PAP pédagogique (tiroir santé, confidentialité standard) : secrétariat / direction / infirmière.
  if (tiroir === "sante") {
    return (
      hasRole(roles, "administratif") ||
      hasRole(roles, "infirmerie") ||
      isDirection(roles) ||
      isExactAdmin(roles)
    );
  }
  return true;
}

export function canOpenDocumentWithoutGrant(
  doc: Pick<EleveDocumentRow, "tiroir" | "confidentialite" | "title">,
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (
    opts?.orgAdmin ||
    opts?.platformAdmin ||
    isExactAdmin(roles) ||
    isDirection(roles)
  ) {
    return true;
  }
  const allowed = eleveDocTiroirsForRoles(roles, opts);
  if (!allowed.has(doc.tiroir as EleveDocTiroir)) return false;
  if (doc.confidentialite === "sante") {
    if (!hasRole(roles, "infirmerie") && !isDirection(roles) && !isExactAdmin(roles)) {
      return false;
    }
  }
  if (doc.confidentialite === "restreint") return false;
  // PAP : même pour un professeur, pas d’ouverture directe — demande d’accès direction requise.
  if (
    doc.tiroir === "sante" &&
    isPapDocumentTitle(doc.title) &&
    hasRole(roles, "professeur") &&
    !isDirection(roles) &&
    !opts?.orgAdmin &&
    !isExactAdmin(roles) &&
    !hasRole(roles, "infirmerie") &&
    !hasRole(roles, "administratif")
  ) {
    return false;
  }
  // Prof : dans le tiroir santé, seul le PAP est listé (accès via grant) — pas le reste médical.
  if (
    doc.tiroir === "sante" &&
    hasRole(roles, "professeur") &&
    !isDirection(roles) &&
    !opts?.orgAdmin &&
    !isExactAdmin(roles) &&
    !hasRole(roles, "infirmerie") &&
    !hasRole(roles, "administratif")
  ) {
    return isPapDocumentTitle(doc.title);
  }
  return true;
}

export async function recordEleveAccessAudit(input: {
  etablissementId: string;
  actorUserId: string | null;
  resourceType: string;
  resourceId: string;
  eleveId?: string | null;
  action: string;
  metadata?: Record<string, unknown>;
}): Promise<void> {
  const db = getDb();
  await db.insert(eleveAccessAudit).values({
    etablissementId: input.etablissementId,
    actorUserId: input.actorUserId,
    resourceType: input.resourceType,
    resourceId: input.resourceId,
    eleveId: input.eleveId ?? null,
    action: input.action,
    metadata: input.metadata ?? null,
  });
}

export async function hasActiveDocumentGrant(opts: {
  etablissementId: string;
  documentId: string;
  userId: string;
}): Promise<boolean> {
  const db = getDb();
  const now = new Date();
  const rows = await db
    .select()
    .from(documentAccessRequest)
    .where(
      and(
        eq(documentAccessRequest.etablissementId, opts.etablissementId),
        eq(documentAccessRequest.documentId, opts.documentId),
        eq(documentAccessRequest.requesterUserId, opts.userId),
        eq(documentAccessRequest.status, "approved"),
        gt(documentAccessRequest.expiresAt, now),
      ),
    )
    .limit(5);
  return rows.length > 0;
}

export async function listEleveDocumentsForViewer(opts: {
  etablissementId: string;
  eleveId: string;
  userId: string;
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): Promise<
  Array<{
    id: string;
    tiroir: string;
    title: string;
    confidentialite: string;
    source: string;
    anneeLabel: string | null;
    mimeType: string | null;
    fileUrl: string | null;
    createdAt: Date;
    canOpen: boolean;
    lockedReason: "tiroir" | "confidentialite" | null;
  }>
> {
  const db = getDb();
  const docs = await db
    .select()
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, opts.etablissementId),
        eq(eleveDocument.eleveId, opts.eleveId),
      ),
    )
    .orderBy(desc(eleveDocument.createdAt));

  const out: Array<{
    id: string;
    tiroir: string;
    title: string;
    confidentialite: string;
    source: string;
    anneeLabel: string | null;
    mimeType: string | null;
    fileUrl: string | null;
    createdAt: Date;
    canOpen: boolean;
    lockedReason: "tiroir" | "confidentialite" | null;
  }> = [];

  const allowedTiroirs = eleveDocTiroirsForRoles(opts.roles, {
    orgAdmin: opts.orgAdmin,
    platformAdmin: opts.platformAdmin,
  });
  const profPapOnly =
    hasRole(opts.roles, "professeur") &&
    !isDirection(opts.roles) &&
    !opts.orgAdmin &&
    !opts.platformAdmin &&
    !isExactAdmin(opts.roles) &&
    !hasRole(opts.roles, "infirmerie") &&
    !hasRole(opts.roles, "administratif");

  for (const doc of docs) {
    if (profPapOnly && doc.tiroir === "sante" && !isPapDocumentTitle(doc.title)) {
      continue;
    }
    const tiroirAllowed = allowedTiroirs.has(doc.tiroir as EleveDocTiroir);

    let canOpen = canOpenDocumentWithoutGrant(doc, opts.roles, {
      orgAdmin: opts.orgAdmin,
      platformAdmin: opts.platformAdmin,
    });
    let lockedReason: "tiroir" | "confidentialite" | null = null;
    if (!canOpen) {
      const grant = await hasActiveDocumentGrant({
        etablissementId: opts.etablissementId,
        documentId: doc.id,
        userId: opts.userId,
      });
      if (grant) {
        canOpen = true;
      } else if (!tiroirAllowed) {
        // Hors catégorie métier (ex. santé pour la compta) : invisible, pas « demander l’accès ».
        continue;
      } else {
        lockedReason = "confidentialite";
      }
    }
    out.push({
      id: doc.id,
      tiroir: doc.tiroir,
      title: doc.title,
      confidentialite: doc.confidentialite,
      source: doc.source,
      anneeLabel: doc.anneeLabel,
      mimeType: doc.mimeType,
      fileUrl: canOpen ? doc.fileUrl : null,
      createdAt: doc.createdAt,
      canOpen,
      lockedReason,
    });
  }
  return out;
}

/** Élèves ayant au moins un PAP (tiroir santé, titre PAP, fichier présent). */
export async function listEleveIdsWithPap(opts: {
  etablissementId: string;
  eleveIds: string[];
}): Promise<Set<string>> {
  const ids = [...new Set(opts.eleveIds.filter(Boolean))];
  const out = new Set<string>();
  if (ids.length === 0) return out;

  const db = getDb();
  const docs = await db
    .select({
      eleveId: eleveDocument.eleveId,
      title: eleveDocument.title,
      fileUrl: eleveDocument.fileUrl,
      confidentialite: eleveDocument.confidentialite,
    })
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, opts.etablissementId),
        eq(eleveDocument.tiroir, "sante"),
        inArray(eleveDocument.eleveId, ids),
      ),
    );

  for (const doc of docs) {
    if (doc.confidentialite === "restreint" || doc.confidentialite === "sante") continue;
    if (!doc.fileUrl) continue;
    if (!isPapDocumentTitle(doc.title)) continue;
    out.add(doc.eleveId);
  }
  return out;
}

/** Dernier PAP déposé (tiroir santé, titre PAP) — pour le badge synthèse. */
export async function getLatestPapDocumentForEleve(opts: {
  etablissementId: string;
  eleveId: string;
}): Promise<{
  id: string;
  title: string;
  fileUrl: string | null;
  mimeType: string | null;
  createdAt: Date;
} | null> {
  const db = getDb();
  const docs = await db
    .select({
      id: eleveDocument.id,
      title: eleveDocument.title,
      fileUrl: eleveDocument.fileUrl,
      mimeType: eleveDocument.mimeType,
      createdAt: eleveDocument.createdAt,
      confidentialite: eleveDocument.confidentialite,
    })
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, opts.etablissementId),
        eq(eleveDocument.eleveId, opts.eleveId),
        eq(eleveDocument.tiroir, "sante"),
      ),
    )
    .orderBy(desc(eleveDocument.createdAt))
    .limit(40);

  for (const doc of docs) {
    if (doc.confidentialite === "restreint" || doc.confidentialite === "sante") continue;
    if (!isPapDocumentTitle(doc.title)) continue;
    if (!doc.fileUrl) continue;
    return {
      id: doc.id,
      title: doc.title,
      fileUrl: doc.fileUrl,
      mimeType: doc.mimeType,
      createdAt: doc.createdAt,
    };
  }
  return null;
}
