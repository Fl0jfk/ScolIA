import { scolaImageUrl } from "@/app/lib/scola-image";
import { loadAppConfig, looksLikeLaProvidenceTenant } from "@/app/lib/app-config";

/** Signatures direction La Providence (même source que /api/travels/sign-pdf). */
export const DEFAULT_DIRECTION_SIGNATURE_URLS: Record<string, string> = {
  ecole: scolaImageUrl("signatures/Sans+titre.jpg"),
  college: scolaImageUrl("signatures/signas.png"),
  lycee: scolaImageUrl("signatures/signature_AMD.png"),
};

/** college | lycee | ecole */
export function establishmentIdForStudentLevel(level: string): string {
  const lv = level.trim().toLowerCase();
  if (["cp", "ce1", "ce2", "cm1", "cm2", "école", "ecole"].some((x) => lv.includes(x))) return "ecole";
  if (["3e", "4e", "5e", "6e", "5ème", "4ème", "3ème", "college", "collège"].some((x) => lv.includes(x))) {
    return "college";
  }
  return "lycee";
}

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

/** Image de signature direction (config voyages, ou défaut La Providence). */
export async function resolveDirectionSignatureImageUrl(studentLevel: string): Promise<string | null> {
  const bundle = await loadAppConfig();
  const estId = establishmentIdForStudentLevel(studentLevel);
  const fromConfig = bundle.travels?.signatureImageUrls?.[estId]?.trim();
  if (fromConfig) return fromConfig;
  if (looksLikeLaProvidenceTenant(bundle.identity)) {
    return DEFAULT_DIRECTION_SIGNATURE_URLS[estId] ?? null;
  }
  return null;
}
