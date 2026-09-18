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
import {
  ACCOMPAGNEMENT_KINDS,
  detectAccompagnementKind,
  isAccompagnementDocumentTitle,
  type AccompagnementKind,
} from "@/app/lib/eleve-pap";
import { eleveDocumentFileProxyPath } from "@/app/lib/eleve-document-file";

function isExactAdmin(roles: string[]): boolean {
  return roles.includes("admin") || hasGlobalAdminRole(roles);
}

function isDirection(roles: string[]): boolean {
  return INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug));
}

/** Direction / admin établissement (pas platform). */
function isEstablishmentAuthority(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  return Boolean(
    opts?.orgAdmin || isExactAdmin(roles) || isDirection(roles),
  );
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
  | "orientation"
  | "psychologue";

export type EleveDocConfidentialite = "standard" | "restreint" | "sante";

export const ALL_ELEVE_DOC_TIROIRS: EleveDocTiroir[] = [
  "scolaire",
  "inscription",
  "facturation",
  "voyages",
  "sante",
  "vie_scolaire",
  "orientation",
  "psychologue",
];

/** Document santé hors PAP/PAI/PPS/GEVASCO → gate infirmerie. */
export function isOwnerGatedSanteDocument(
  doc: Pick<EleveDocumentRow, "tiroir" | "title"> | { tiroir: string; title: string },
): boolean {
  return doc.tiroir === "sante" && !isAccompagnementDocumentTitle(doc.title);
}

/** Psy ou santé médicale (hors accompagnement) : ouverture via grant propriétaire. */
export function isOwnerGatedDocument(
  doc: Pick<EleveDocumentRow, "tiroir" | "title" | "confidentialite"> | {
    tiroir: string;
    title: string;
    confidentialite?: string;
  },
): boolean {
  if (doc.tiroir === "psychologue") return true;
  if (isOwnerGatedSanteDocument(doc)) return true;
  if (doc.confidentialite === "sante" || doc.confidentialite === "restreint") return true;
  return false;
}

/** Qui peut valider une demande d’accès pour ce document. */
export function canDecideDocumentAccessGrant(
  doc: Pick<EleveDocumentRow, "tiroir" | "title" | "confidentialite">,
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (opts?.platformAdmin) return true;
  if (doc.tiroir === "psychologue") {
    return hasRole(roles, "psychologue");
  }
  if (isOwnerGatedSanteDocument(doc) || doc.confidentialite === "sante") {
    return hasRole(roles, "infirmerie");
  }
  if (doc.confidentialite === "restreint") {
    return isEstablishmentAuthority(roles, opts);
  }
  return false;
}

/** Catégories documents visibles nativement pour le rôle. */
export function eleveDocCategoriesForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): Set<EleveDocCategorie> {
  if (opts?.platformAdmin || isEstablishmentAuthority(roles, opts)) {
    return new Set<EleveDocCategorie>([
      "administratif",
      "vie_scolaire",
      "financier",
      "sante",
      "psychologue",
    ]);
  }
  const out = new Set<EleveDocCategorie>();
  if (hasRole(roles, "administratif")) {
    out.add("administratif");
    out.add("sante"); // PAP·PAI·PPS·GEVASCO uniquement (filtré à la liste)
  }
  if (hasRole(roles, "comptabilite")) {
    out.add("financier");
  }
  if (hasRole(roles, "infirmerie")) {
    out.add("sante");
  }
  if (hasRole(roles, "psychologue")) {
    out.add("psychologue");
  }
  if (hasRole(roles, "cpe")) {
    out.add("vie_scolaire");
    out.add("administratif"); // dossier scolaire partagé
  }
  if (hasRole(roles, "surveillant")) {
    out.add("vie_scolaire");
  }
  if (hasRole(roles, "professeur")) {
    out.add("sante"); // PAP·PAI·PPS·GEVASCO (synthèse / ouverture ciblée)
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
  if (hasRole(roles, "psychologue")) {
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

/** Tiroirs documents accessibles nativement (liste / dépôt selon rôle). */
export function eleveDocTiroirsForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): Set<EleveDocTiroir> {
  if (opts?.platformAdmin || isEstablishmentAuthority(roles, opts)) {
    return new Set<EleveDocTiroir>(ALL_ELEVE_DOC_TIROIRS);
  }

  const tiroirs = new Set<EleveDocTiroir>();

  if (hasRole(roles, "administratif")) {
    tiroirs.add("scolaire");
    tiroirs.add("inscription");
    tiroirs.add("voyages");
    tiroirs.add("orientation");
    tiroirs.add("sante"); // accompagnement uniquement à la liste
  }
  if (hasRole(roles, "comptabilite")) {
    tiroirs.add("facturation");
  }
  if (hasRole(roles, "infirmerie")) {
    tiroirs.add("sante");
  }
  if (hasRole(roles, "psychologue")) {
    tiroirs.add("psychologue");
  }
  if (hasRole(roles, "cpe")) {
    tiroirs.add("vie_scolaire");
    tiroirs.add("scolaire");
  }
  if (hasRole(roles, "surveillant")) {
    tiroirs.add("vie_scolaire");
  }
  if (hasRole(roles, "professeur")) {
    // Synthèse uniquement — tiroirs pour ouverture PAP ciblée.
    tiroirs.add("sante");
  }

  // Repli catégories → tiroirs si rien d’explicite (sécurité).
  if (tiroirs.size === 0) {
    const categories = eleveDocCategoriesForRoles(roles, opts);
    for (const t of tiroirsForCategories(categories)) {
      tiroirs.add(t as EleveDocTiroir);
    }
  }

  return tiroirs;
}

export function eleveDocCategoriesMetaForRoles(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): EleveDocCategorie[] {
  const allowed = eleveDocCategoriesForRoles(roles, opts);
  return DOC_CATEGORIE_ORDER_LOCAL.filter((c) => allowed.has(c));
}

const DOC_CATEGORIE_ORDER_LOCAL: EleveDocCategorie[] = [
  "administratif",
  "vie_scolaire",
  "financier",
  "sante",
  "psychologue",
];

/** Viewer pédagogique : ouvre PAP·PAI·PPS·GEVASCO sans demande. */
function isPedagogicalAccompagnementViewer(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (
    opts?.platformAdmin ||
    opts?.orgAdmin ||
    isExactAdmin(roles) ||
    isDirection(roles) ||
    hasRole(roles, "infirmerie") ||
    hasRole(roles, "administratif")
  ) {
    return false;
  }
  return (
    hasRole(roles, "professeur") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "surveillant")
  );
}

/** Administratif : voit / gère l’accompagnement, pas le médical. */
function isAdministratifAccompagnementOnly(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (opts?.platformAdmin || isEstablishmentAuthority(roles, opts)) return false;
  if (hasRole(roles, "infirmerie")) return false;
  return hasRole(roles, "administratif");
}

/** Enregistrement d’un document (upload) selon tiroir et confidentialité. */
export function canRegisterEleveDocument(
  tiroir: EleveDocTiroir,
  confidentialite: EleveDocConfidentialite,
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (opts?.platformAdmin) return true;
  if (confidentialite === "restreint") {
    return isEstablishmentAuthority(roles, opts);
  }
  if (confidentialite === "sante") {
    return hasRole(roles, "infirmerie");
  }

  if (tiroir === "psychologue") {
    return hasRole(roles, "psychologue");
  }
  if (tiroir === "sante") {
    // Dépôt PAP·PAI·PPS·GEVASCO : admin / direction / infirmerie.
    // Autres pièces santé : infirmerie uniquement.
    return (
      hasRole(roles, "infirmerie") ||
      hasRole(roles, "administratif") ||
      isEstablishmentAuthority(roles, opts)
    );
  }
  if (tiroir === "facturation") {
    return hasRole(roles, "comptabilite") || isEstablishmentAuthority(roles, opts);
  }
  if (tiroir === "vie_scolaire") {
    return (
      hasRole(roles, "cpe") ||
      hasRole(roles, "surveillant") ||
      isEstablishmentAuthority(roles, opts)
    );
  }
  // Silos admin (scolaire, inscription, voyages, orientation)
  if (
    tiroir === "scolaire" ||
    tiroir === "inscription" ||
    tiroir === "voyages" ||
    tiroir === "orientation"
  ) {
    return (
      hasRole(roles, "administratif") ||
      (tiroir === "scolaire" && hasRole(roles, "cpe")) ||
      isEstablishmentAuthority(roles, opts)
    );
  }
  return false;
}

/**
 * Suppression d’une pièce du dossier élève :
 * propriétaire du silo, ou autorité établissement (sauf psy / médical → propriétaire).
 */
export function canDeleteEleveDocument(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  return Boolean(
    opts?.platformAdmin ||
      opts?.orgAdmin ||
      isExactAdmin(roles) ||
      isDirection(roles) ||
      hasRole(roles, "administratif") ||
      hasRole(roles, "cpe") ||
      hasRole(roles, "comptabilite") ||
      hasRole(roles, "infirmerie") ||
      hasRole(roles, "psychologue"),
  );
}

/** Suppression PAP / PAI / PPS / GEVASCO : même périmètre métier dépôt. */
export function canDeleteEleveAccompagnementDocument(
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  return (
    Boolean(opts?.platformAdmin) ||
    isEstablishmentAuthority(roles, opts) ||
    hasRole(roles, "administratif") ||
    hasRole(roles, "infirmerie")
  );
}

export function canDeleteSpecificEleveDocument(
  doc: { tiroir: EleveDocTiroir; confidentialite: EleveDocConfidentialite; title?: string },
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (opts?.platformAdmin) return true;
  if (doc.tiroir === "psychologue") {
    return hasRole(roles, "psychologue");
  }
  if (doc.tiroir === "sante") {
    if (doc.title && isAccompagnementDocumentTitle(doc.title)) {
      return canDeleteEleveAccompagnementDocument(roles, opts);
    }
    return hasRole(roles, "infirmerie");
  }
  if (!canDeleteEleveDocument(roles, opts)) return false;
  return canRegisterEleveDocument(doc.tiroir, doc.confidentialite, roles, opts);
}

export function canOpenDocumentWithoutGrant(
  doc: Pick<EleveDocumentRow, "tiroir" | "confidentialite" | "title">,
  roles: string[],
  opts?: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  if (opts?.platformAdmin) return true;

  // Psychologue : uniquement le psy (même pas la direction).
  if (doc.tiroir === "psychologue") {
    return hasRole(roles, "psychologue");
  }

  // PAP / PAI / PPS / GEVASCO : accès pédagogique + métiers dépôt.
  if (doc.tiroir === "sante" && isAccompagnementDocumentTitle(doc.title)) {
    if (doc.confidentialite === "restreint") return false;
    if (
      isEstablishmentAuthority(roles, opts) ||
      hasRole(roles, "infirmerie") ||
      hasRole(roles, "administratif") ||
      hasRole(roles, "professeur") ||
      hasRole(roles, "cpe") ||
      hasRole(roles, "surveillant")
    ) {
      return true;
    }
    return false;
  }

  // Santé médicale (hors accompagnement) : infirmerie uniquement.
  if (doc.tiroir === "sante" || doc.confidentialite === "sante") {
    return hasRole(roles, "infirmerie");
  }

  if (doc.confidentialite === "restreint") {
    return isEstablishmentAuthority(roles, opts);
  }

  const allowed = eleveDocTiroirsForRoles(roles, opts);
  if (!allowed.has(doc.tiroir as EleveDocTiroir)) return false;

  // Prof / CPE / surveillant : pas d’ouverture libre hors accompagnement déjà traité.
  if (isPedagogicalAccompagnementViewer(roles, opts)) {
    return false;
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
    canDelete: boolean;
    lockedReason: "tiroir" | "confidentialite" | "grant_required" | null;
    canRequestAccess: boolean;
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
    canDelete: boolean;
    lockedReason: "tiroir" | "confidentialite" | "grant_required" | null;
    canRequestAccess: boolean;
  }> = [];

  const roleOpts = {
    orgAdmin: opts.orgAdmin,
    platformAdmin: opts.platformAdmin,
  };
  const allowedTiroirs = eleveDocTiroirsForRoles(opts.roles, roleOpts);
  const pedagogicalPapOnly = isPedagogicalAccompagnementViewer(opts.roles, roleOpts);
  const adminAccompagnementOnly = isAdministratifAccompagnementOnly(opts.roles, roleOpts);
  const authority = isEstablishmentAuthority(opts.roles, roleOpts);

  for (const doc of docs) {
    // Prof / CPE / surveillant : tiroir santé = accompagnement uniquement.
    if (
      pedagogicalPapOnly &&
      doc.tiroir === "sante" &&
      !isAccompagnementDocumentTitle(doc.title)
    ) {
      continue;
    }
    // Administratif : santé = accompagnement uniquement (pas le médical).
    if (
      adminAccompagnementOnly &&
      doc.tiroir === "sante" &&
      !isAccompagnementDocumentTitle(doc.title)
    ) {
      continue;
    }
    // Psychologue : invisible hors psy / autorité (autorité voit verrouillé pour demander).
    if (doc.tiroir === "psychologue") {
      const isPsy = hasRole(opts.roles, "psychologue");
      if (!isPsy && !authority && !opts.platformAdmin) {
        continue;
      }
    } else if (!allowedTiroirs.has(doc.tiroir as EleveDocTiroir)) {
      // Hors silo métier : invisible (ex. compta ne voit pas le scolaire).
      if (!authority && !opts.platformAdmin) {
        continue;
      }
    }

    let canOpen = canOpenDocumentWithoutGrant(doc, opts.roles, roleOpts);
    let lockedReason: "tiroir" | "confidentialite" | "grant_required" | null = null;
    if (!canOpen) {
      const grant = await hasActiveDocumentGrant({
        etablissementId: opts.etablissementId,
        documentId: doc.id,
        userId: opts.userId,
      });
      if (grant) {
        canOpen = true;
      } else if (isOwnerGatedDocument(doc)) {
        lockedReason = "grant_required";
      } else if (!allowedTiroirs.has(doc.tiroir as EleveDocTiroir)) {
        continue;
      } else {
        lockedReason = "confidentialite";
      }
    }

    const canDelete = canDeleteSpecificEleveDocument(
      {
        tiroir: doc.tiroir as EleveDocTiroir,
        confidentialite: doc.confidentialite as EleveDocConfidentialite,
        title: doc.title,
      },
      opts.roles,
      roleOpts,
    );

    const canRequestAccess =
      !canOpen &&
      lockedReason === "grant_required" &&
      !hasRole(opts.roles, "psychologue") &&
      !(doc.tiroir === "sante" && hasRole(opts.roles, "infirmerie"));

    out.push({
      id: doc.id,
      tiroir: doc.tiroir,
      title: doc.title,
      confidentialite: doc.confidentialite,
      source: doc.source,
      anneeLabel: doc.anneeLabel,
      mimeType: doc.mimeType,
      fileUrl:
        canOpen && (doc.fileUrl || doc.s3Key)
          ? eleveDocumentFileProxyPath(opts.eleveId, doc.id)
          : null,
      createdAt: doc.createdAt,
      canOpen,
      canDelete,
      lockedReason,
      canRequestAccess,
    });
  }
  return out;
}

export type EleveAccompagnementDoc = {
  kind: AccompagnementKind;
  id: string;
  title: string;
  fileUrl: string | null;
  mimeType: string | null;
  createdAt: Date;
};

export type EleveAccompagnementListItem = {
  kind: AccompagnementKind;
  documentId: string;
};

export async function listEleveLatestAccompagnementByKind(opts: {
  etablissementId: string;
  eleveIds: string[];
}): Promise<Map<string, EleveAccompagnementListItem[]>> {
  const ids = [...new Set(opts.eleveIds.filter(Boolean))];
  const out = new Map<string, EleveAccompagnementListItem[]>();
  if (ids.length === 0) return out;

  const db = getDb();
  const docs = await db
    .select({
      id: eleveDocument.id,
      eleveId: eleveDocument.eleveId,
      title: eleveDocument.title,
      fileUrl: eleveDocument.fileUrl,
      confidentialite: eleveDocument.confidentialite,
      createdAt: eleveDocument.createdAt,
    })
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, opts.etablissementId),
        eq(eleveDocument.tiroir, "sante"),
        inArray(eleveDocument.eleveId, ids),
      ),
    )
    .orderBy(desc(eleveDocument.createdAt));

  const latestByEleve = new Map<string, Map<AccompagnementKind, string>>();
  for (const doc of docs) {
    if (doc.confidentialite === "restreint" || doc.confidentialite === "sante") continue;
    if (!doc.fileUrl) continue;
    const kind = detectAccompagnementKind(doc.title);
    if (!kind) continue;
    let byKind = latestByEleve.get(doc.eleveId);
    if (!byKind) {
      byKind = new Map();
      latestByEleve.set(doc.eleveId, byKind);
    }
    if (byKind.has(kind)) continue;
    byKind.set(kind, doc.id);
  }

  const kindOrder = ACCOMPAGNEMENT_KINDS.map((k) => k.kind);
  for (const [eleveId, byKind] of latestByEleve) {
    out.set(
      eleveId,
      kindOrder.flatMap((kind) => {
        const documentId = byKind.get(kind);
        return documentId ? [{ kind, documentId }] : [];
      }),
    );
  }
  return out;
}

export async function listEleveAccompagnementKinds(opts: {
  etablissementId: string;
  eleveIds: string[];
}): Promise<Map<string, Set<AccompagnementKind>>> {
  const rich = await listEleveLatestAccompagnementByKind(opts);
  const out = new Map<string, Set<AccompagnementKind>>();
  for (const [eleveId, items] of rich) {
    out.set(eleveId, new Set(items.map((i) => i.kind)));
  }
  return out;
}

export async function listEleveIdsWithPap(opts: {
  etablissementId: string;
  eleveIds: string[];
}): Promise<Set<string>> {
  const map = await listEleveAccompagnementKinds(opts);
  return new Set(map.keys());
}

export async function getLatestAccompagnementDocumentsForEleve(opts: {
  etablissementId: string;
  eleveId: string;
}): Promise<EleveAccompagnementDoc[]> {
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
    .limit(80);

  const latestByKind = new Map<AccompagnementKind, EleveAccompagnementDoc>();
  for (const doc of docs) {
    if (doc.confidentialite === "restreint" || doc.confidentialite === "sante") continue;
    if (!doc.fileUrl) continue;
    const kind = detectAccompagnementKind(doc.title);
    if (!kind || latestByKind.has(kind)) continue;
    latestByKind.set(kind, {
      kind,
      id: doc.id,
      title: doc.title,
      fileUrl: doc.fileUrl,
      mimeType: doc.mimeType,
      createdAt: doc.createdAt,
    });
  }

  const order: AccompagnementKind[] = ACCOMPAGNEMENT_KINDS.map((k) => k.kind);
  return order.flatMap((k) => {
    const row = latestByKind.get(k);
    return row ? [row] : [];
  });
}

/** @deprecated Préférer `getLatestAccompagnementDocumentsForEleve`. */
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
  const rows = await getLatestAccompagnementDocumentsForEleve(opts);
  const pap = rows.find((r) => r.kind === "pap");
  if (!pap) return null;
  return {
    id: pap.id,
    title: pap.title,
    fileUrl: pap.fileUrl,
    mimeType: pap.mimeType,
    createdAt: pap.createdAt,
  };
}
