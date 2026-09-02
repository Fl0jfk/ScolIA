import { loadAppConfig } from "@/app/lib/app-config";
import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  establishmentIdForStudentLevel,
  resolveDirectionSignatureDisplayUrlForLevel,
} from "@/app/lib/direction-signature";
import { resolvePhotocopiesOpsEmails } from "@/app/lib/photocopies-couleur-ops";
import { inferSecteurFromFolderName } from "@/app/lib/onedrive-eleves";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";

export type StageCycleKind = "ecole" | "college" | "lycee";

const CYCLE_LABELS: Record<StageCycleKind, string> = {
  ecole: "École",
  college: "Collège",
  lycee: "Lycée",
};

export function stageCycleLabel(kind: StageCycleKind): string {
  return CYCLE_LABELS[kind];
}

/** Cycle stages dérivé du niveau / classe élève. */
export function stageCycleKindFromStudent(
  level: string,
  className?: string,
): StageCycleKind {
  const fromClass =
    inferSecteurFromFolderName(String(className || "").trim()) ||
    inferSecteurFromFolderName(String(level || "").trim());
  if (fromClass === "ecole" || fromClass === "college" || fromClass === "lycee") {
    return fromClass;
  }
  const id = establishmentIdForStudentLevel(level);
  if (id === "ecole" || id === "college" || id === "lycee") return id;
  return "lycee";
}

function establishmentMatchesCycle(est: Establishment, kind: StageCycleKind): boolean {
  if (est.active === false) return false;
  const estKind = (est.kind || est.id || "").toLowerCase();
  if (estKind === kind) return true;
  if (est.id.toLowerCase() === kind) return true;
  if (est.id.toLowerCase().includes(kind)) return true;
  if (kind === "college" && /coll[eè]ge/i.test(est.label)) return true;
  if (kind === "lycee" && /lyc[eé]e/i.test(est.label)) return true;
  if (kind === "ecole" && /[eé]cole/i.test(est.label)) return true;
  return false;
}

/**
 * E-mails de la file administratif pour une préconvention / dépôt.
 * Priorité : liste du cycle → liste plate legacy → réceptionnaires photocopies.
 */
export async function resolveStagesAdminEmails(
  studentLevel?: string,
  className?: string,
): Promise<string[]> {
  const bundle = await loadAppConfig();
  const byKind = bundle.notifications.stagesAdminEmailsByKind;
  if (studentLevel != null && String(studentLevel).trim()) {
    const kind = stageCycleKindFromStudent(studentLevel, className);
    const forKind = byKind?.[kind] ?? [];
    if (forKind.length) return forKind;
  } else if (byKind) {
    const merged = new Set<string>();
    for (const kind of ["ecole", "college", "lycee"] as const) {
      for (const e of byKind[kind] ?? []) merged.add(e);
    }
    if (merged.size) return [...merged];
  }

  const legacy = bundle.notifications.stagesAdminEmails ?? [];
  if (legacy.length) return legacy;
  return resolvePhotocopiesOpsEmails(bundle.notifications);
}

/**
 * E-mail direction signataire pour le cycle de l’élève.
 * Priorité : override notifications du cycle → directeur de l’établissement du cycle
 * → override global legacy (déprécié). Pas de repli sur un autre établissement.
 */
export async function resolveStagesDirectionEmail(
  studentLevel: string,
  className?: string,
): Promise<string | undefined> {
  const bundle = await loadAppConfig();
  const kind = stageCycleKindFromStudent(studentLevel, className);

  const byKind = bundle.notifications.stagesDirectionEmailByKind?.[kind]?.trim();
  if (byKind) return byKind;

  const est =
    bundle.establishments.find((e) => establishmentMatchesCycle(e, kind) && e.directorEmail?.trim()) ||
    bundle.establishments.find(
      (e) =>
        e.active !== false &&
        (e.id === kind || e.id.includes(kind)) &&
        e.directorEmail?.trim(),
    );
  const fromEst = est?.directorEmail?.trim();
  if (fromEst) return fromEst;

  const legacyGlobal = bundle.notifications.stagesDirectionEmail?.trim();
  return legacyGlobal || undefined;
}

export async function resolveStagesConventionTemplateUrl(): Promise<string | undefined> {
  const bundle = await loadAppConfig();
  return bundle.notifications.stagesConventionTemplateUrl?.trim() || undefined;
}

/**
 * URL signée de la signature direction (dataBucket privé).
 * Plus de fallback CDN public / chemins en dur.
 */
export async function resolveDirectionSignatureImageUrl(studentLevel: string): Promise<string | null> {
  return resolveDirectionSignatureDisplayUrlForLevel(studentLevel);
}

export function stageCycleKindToSecteur(kind: StageCycleKind): Secteur {
  return kind;
}
