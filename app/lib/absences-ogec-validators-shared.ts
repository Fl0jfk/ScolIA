import type {
  AbsenceNotifyPerson,
  Establishment,
  NotificationsConfig,
} from "@/app/lib/app-config-schemas";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

/** Validateur nominatif d’une absence OGEC (snapshot sur la demande). */
export type OgecAbsenceValidatorRef = {
  email: string;
  userId?: string;
  label?: string;
};

/** Sérialise le validateur d’absences OGEC sur `personnel.manager_id`. */
export function serializePersonnelAbsenceManager(
  person: Pick<AbsenceNotifyPerson, "email" | "userId" | "label"> | null | undefined,
): string | null {
  if (!person) return null;
  const email = String(person.email || "")
    .trim()
    .toLowerCase();
  const userId = String(person.userId || "").trim() || undefined;
  const label = String(person.label || "").trim() || undefined;
  if (!email && !userId) return null;
  return JSON.stringify({
    ...(email ? { email } : {}),
    ...(userId ? { userId } : {}),
    ...(label ? { label } : {}),
  });
}

/** Lit `personnel.manager_id` (JSON, e-mail simple, ou userId). */
export function parsePersonnelAbsenceManager(
  raw: string | null | undefined,
): OgecAbsenceValidatorRef | null {
  const value = String(raw || "").trim();
  if (!value) return null;
  if (value.startsWith("{")) {
    try {
      const o = JSON.parse(value) as Record<string, unknown>;
      const email = String(o.email || "")
        .trim()
        .toLowerCase();
      const userId = String(o.userId || "").trim() || undefined;
      const label = String(o.label || "").trim() || undefined;
      if (!email && !userId) return null;
      return { email: email || "", userId, label };
    } catch {
      return null;
    }
  }
  if (value.includes("@")) {
    return { email: value.toLowerCase() };
  }
  return { email: "", userId: value };
}

export function normalizeOgecValidatorRef(
  raw: unknown,
): OgecAbsenceValidatorRef | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const email = String(o.email || "")
    .trim()
    .toLowerCase();
  const userId = String(o.userId || "").trim() || undefined;
  const label = String(o.label || "").trim() || undefined;
  if (!email && !userId) return null;
  return { email: email || "", userId, label };
}

export function lyceeEstablishment(establishments: Establishment[]): Establishment | null {
  const active = getActiveEstablishments(establishments);
  return active.find((e) => inferEstablishmentKind(e) === "lycee") || null;
}

export function defaultOgecValidatorsFromConfig(
  notifications: NotificationsConfig | null | undefined,
  establishments: Establishment[],
): OgecAbsenceValidatorRef[] {
  const configured = (notifications?.absencesValidatorsOgec || []).filter((p) =>
    String(p?.email || "").trim(),
  );
  if (configured.length > 0) {
    return configured.map((p) => ({
      email: String(p.email || "")
        .trim()
        .toLowerCase(),
      userId: String(p.userId || "").trim() || undefined,
      label: String(p.label || "").trim() || undefined,
    }));
  }
  const lycee = lyceeEstablishment(establishments);
  const email = String(lycee?.directorEmail || "")
    .trim()
    .toLowerCase();
  if (!email) return [];
  return [
    {
      email,
      userId: String(lycee?.directorExternalUserId || "").trim() || undefined,
      label: String(lycee?.directorName || "").trim() || lycee?.label || undefined,
    },
  ];
}

/**
 * Validateurs effectifs pour une absence OGEC (sync, côté UI + API).
 * Priorité : snapshot sur la demande → liste globale / défaut lycée.
 */
export function resolveOgecValidatorsForAbsence(
  abs: {
    data?: {
      ogecValidator?: unknown;
    };
  },
  notifications: NotificationsConfig | null | undefined,
  establishments: Establishment[],
): OgecAbsenceValidatorRef[] {
  const personal = normalizeOgecValidatorRef(abs.data?.ogecValidator);
  if (personal) return [personal];
  return defaultOgecValidatorsFromConfig(notifications, establishments);
}

export function viewerMatchesOgecValidators(
  validators: Array<Pick<OgecAbsenceValidatorRef, "email" | "userId">>,
  viewer: { email?: string | null; userId?: string | null },
): boolean {
  const email = String(viewer.email || "")
    .trim()
    .toLowerCase();
  const userId = String(viewer.userId || "").trim();
  return validators.some((p) => {
    if (email && p.email && p.email.trim().toLowerCase() === email) return true;
    if (userId && p.userId && p.userId === userId) return true;
    return false;
  });
}
