import "server-only";

import { and, desc, eq, gt, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveDocument } from "@/db/schema";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  listClassesForTeacherUser,
  studentInAssignedClasses,
} from "@/app/lib/class-allocation-teachers";
import {
  accompagnementKindDef,
  detectAccompagnementKind,
  type AccompagnementKind,
} from "@/app/lib/eleve-pap";

/** Fenêtre max des alertes (évite une notif éternelle). */
export const ACCOMPAGNEMENT_ALERT_WINDOW_DAYS = 21;

export type TeacherAccompagnementAlert = {
  documentId: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  classe: string | null;
  kind: AccompagnementKind;
  title: string;
  createdAt: string;
};

type SeenState = {
  /** Première activation pour ce prof — docs antérieurs = déjà vus (pas de notif rétroactive). */
  seededAt: string;
  documentIds: string[];
};

function seenRelPath(businessUserId: string): string {
  return `eleves/accompagnement-alerts-seen/${encodeURIComponent(businessUserId)}.json`;
}

async function loadSeenState(businessUserId: string): Promise<SeenState | null> {
  try {
    const hit = await getJson<SeenState>(seenRelPath(businessUserId));
    if (!hit?.data || typeof hit.data !== "object") return null;
    const seededAt = typeof hit.data.seededAt === "string" ? hit.data.seededAt : "";
    const documentIds = Array.isArray(hit.data.documentIds)
      ? hit.data.documentIds.filter((x): x is string => typeof x === "string")
      : [];
    if (!seededAt) return null;
    return { seededAt, documentIds };
  } catch {
    return null;
  }
}

async function saveSeenState(businessUserId: string, state: SeenState): Promise<void> {
  await putJson(seenRelPath(businessUserId), state);
}

async function listRecentAccompagnementDocsForClasses(opts: {
  etablissementId: string;
  assignedClasses: string[];
  since: Date;
}): Promise<TeacherAccompagnementAlert[]> {
  if (opts.assignedClasses.length === 0) return [];

  const db = getDb();
  const eleves = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(eq(eleve.etablissementId, opts.etablissementId));

  const inScope = eleves.filter((e) =>
    studentInAssignedClasses(e.classe ?? undefined, opts.assignedClasses),
  );
  if (inScope.length === 0) return [];

  const byId = new Map(inScope.map((e) => [e.id, e]));
  const eleveIds = [...byId.keys()];

  const docs = await db
    .select({
      id: eleveDocument.id,
      eleveId: eleveDocument.eleveId,
      title: eleveDocument.title,
      createdAt: eleveDocument.createdAt,
    })
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, opts.etablissementId),
        eq(eleveDocument.tiroir, "sante"),
        inArray(eleveDocument.eleveId, eleveIds),
        gt(eleveDocument.createdAt, opts.since),
      ),
    )
    .orderBy(desc(eleveDocument.createdAt))
    .limit(200);

  const out: TeacherAccompagnementAlert[] = [];
  for (const doc of docs) {
    const kind = detectAccompagnementKind(doc.title);
    if (!kind) continue;
    const e = byId.get(doc.eleveId);
    if (!e) continue;
    const createdAt =
      doc.createdAt instanceof Date
        ? doc.createdAt.toISOString()
        : String(doc.createdAt || "");
    out.push({
      documentId: doc.id,
      eleveId: e.id,
      eleveNom: e.nom,
      elevePrenom: e.prenom,
      classe: e.classe,
      kind,
      title: doc.title,
      createdAt,
    });
  }
  return out;
}

/**
 * PAP / PAI / PPS récemment ajoutés sur les élèves des classes du prof,
 * non encore consultés (ouverture de la fiche).
 */
export async function listUnseenAccompagnementAlertsForTeacher(opts: {
  etablissementId: string;
  businessUserId: string;
  windowDays?: number;
}): Promise<TeacherAccompagnementAlert[]> {
  const assignedClasses = await listClassesForTeacherUser(opts.businessUserId);
  if (assignedClasses.length === 0) return [];

  const days = opts.windowDays ?? ACCOMPAGNEMENT_ALERT_WINDOW_DAYS;
  const since = new Date(Date.now() - days * 24 * 60 * 60 * 1000);
  const recent = await listRecentAccompagnementDocsForClasses({
    etablissementId: opts.etablissementId,
    assignedClasses,
    since,
  });
  if (recent.length === 0) return [];

  let seen = await loadSeenState(opts.businessUserId);
  if (!seen) {
    // Première fois : on seed pour ne pas spammer avec l’historique déjà en place.
    seen = {
      seededAt: new Date().toISOString(),
      documentIds: recent.map((r) => r.documentId),
    };
    await saveSeenState(opts.businessUserId, seen);
    return [];
  }

  const seenSet = new Set(seen.documentIds);
  return recent.filter((r) => {
    if (seenSet.has(r.documentId)) return false;
    // Docs antérieurs au seed = déjà en place à l’activation.
    return r.createdAt > seen!.seededAt;
  });
}

/** Marque les dispositifs d’accompagnement d’un élève comme vus (après ouverture de fiche). */
export async function markAccompagnementAlertsSeenForEleve(opts: {
  etablissementId: string;
  businessUserId: string;
  eleveId: string;
}): Promise<void> {
  const db = getDb();
  const docs = await db
    .select({ id: eleveDocument.id, title: eleveDocument.title })
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, opts.etablissementId),
        eq(eleveDocument.eleveId, opts.eleveId),
        eq(eleveDocument.tiroir, "sante"),
      ),
    );

  const ids = docs
    .filter((d) => detectAccompagnementKind(d.title))
    .map((d) => d.id);
  if (ids.length === 0) return;

  const prev = (await loadSeenState(opts.businessUserId)) ?? {
    seededAt: new Date().toISOString(),
    documentIds: [],
  };
  const set = new Set(prev.documentIds);
  let changed = false;
  for (const id of ids) {
    if (!set.has(id)) {
      set.add(id);
      changed = true;
    }
  }
  if (!changed) return;
  await saveSeenState(opts.businessUserId, {
    seededAt: prev.seededAt,
    documentIds: [...set],
  });
}

export function formatAccompagnementAlertDetail(alert: TeacherAccompagnementAlert): string {
  const code = accompagnementKindDef(alert.kind).code;
  const name = `${alert.elevePrenom} ${alert.eleveNom}`.trim();
  const classe = alert.classe?.trim();
  return classe ? `${code} ajouté — ${name} (${classe})` : `${code} ajouté — ${name}`;
}
