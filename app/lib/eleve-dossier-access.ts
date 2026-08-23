import "server-only";

import { and, desc, eq, gt } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  documentAccessRequest,
  eleveAccessAudit,
  eleveDocument,
  type EleveDocumentRow,
} from "@/db/schema";
import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";

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
  | "vie_scolaire";

export type EleveDocConfidentialite = "standard" | "restreint" | "sante";

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
    out.add("documents");
    out.add("notes");
  }
  if (hasRole(roles, "cpe") || hasRole(roles, "education")) {
    out.add("vie_scolaire");
    out.add("documents");
    out.add("famille");
  }
  if (hasRole(roles, "infirmerie")) {
    out.add("sante");
    out.add("famille");
  }
  if (hasRole(roles, "comptabilite") || hasRole(roles, "administratif")) {
    out.add("famille");
    out.add("documents");
    out.add("facturation");
  }
  if (hasRole(roles, "administratif")) {
    out.add("famille");
    out.add("documents");
  }
  return out;
}

/** Tiroirs documents accessibles nativement (sans demande). */
export function eleveDocTiroirsForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): Set<EleveDocTiroir> {
  const sections = eleveDossierSectionsForRoles(roles, opts);
  const tiroirs = new Set<EleveDocTiroir>();
  if (sections.has("notes") || sections.has("scolarite")) tiroirs.add("scolaire");
  if (sections.has("facturation") || hasRole(roles, "administratif") || opts?.orgAdmin) {
    tiroirs.add("inscription");
    tiroirs.add("facturation");
  }
  if (sections.has("vie_scolaire")) tiroirs.add("vie_scolaire");
  if (sections.has("sante")) tiroirs.add("sante");
  if (
    opts?.orgAdmin ||
    opts?.platformAdmin ||
    isExactAdmin(roles) ||
    isDirection(roles)
  ) {
    return new Set([
      "scolaire",
      "inscription",
      "facturation",
      "voyages",
      "sante",
      "vie_scolaire",
    ]);
  }
  if (hasRole(roles, "professeur")) {
    tiroirs.add("scolaire");
    tiroirs.add("voyages");
  }
  if (hasRole(roles, "administratif")) {
    tiroirs.add("inscription");
    tiroirs.add("facturation");
    tiroirs.add("voyages");
    // PAS sante / PAP par défaut
  }
  return tiroirs;
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
  return true;
}

export function canOpenDocumentWithoutGrant(
  doc: Pick<EleveDocumentRow, "tiroir" | "confidentialite">,
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

  for (const doc of docs) {
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
      } else {
        const tiroirs = eleveDocTiroirsForRoles(opts.roles, {
          orgAdmin: opts.orgAdmin,
          platformAdmin: opts.platformAdmin,
        });
        lockedReason = tiroirs.has(doc.tiroir as EleveDocTiroir)
          ? "confidentialite"
          : "tiroir";
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
