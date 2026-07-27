import { loadAppConfig } from "@/app/lib/app-config";
import {
  establishmentIdForStudentLevel,
  resolveDirectionSignatureDisplayUrlForLevel,
} from "@/app/lib/direction-signature";

export { establishmentIdForStudentLevel };

/** @deprecated Plus de signatures sur le CDN public — utiliser Paramètres → Établissements. */
export const DEFAULT_DIRECTION_SIGNATURE_URLS: Record<string, string> = {};

export async function resolveStagesAdminEmails(): Promise<string[]> {
  const bundle = await loadAppConfig();
  const fromNotif = bundle.notifications.stagesAdminEmails ?? [];
  if (fromNotif.length) return fromNotif;
  const fallback = bundle.notifications.photocopiesOps?.trim();
  return fallback ? [fallback] : [];
}

export async function resolveStagesDirectionEmail(studentLevel: string): Promise<string | undefined> {
  const bundle = await loadAppConfig();
  const explicit = bundle.notifications.stagesDirectionEmail?.trim();
  if (explicit) return explicit;
  const estId = establishmentIdForStudentLevel(studentLevel);
  const est = bundle.establishments.find((e) => e.id === estId || e.id.includes(estId));
  return est?.directorEmail?.trim() || bundle.establishments.find((e) => e.directorEmail)?.directorEmail?.trim();
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
