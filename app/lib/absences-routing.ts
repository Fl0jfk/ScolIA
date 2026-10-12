/**
 * Routage absences RH — validation vs traitement, 100 % paramétrable.
 *
 * Validation (accepter / prise d’acte) :
 *   OGEC  → fiche RH (exception) → absencesValidatorsOgec → directeur établissement lycée
 *   Prof  → absencesValidatorsProf{cycle} → directeur de l’établissement du cycle
 *
 * Traitement (après validation) : absencesNotify* (inchangé — collectAbsenceProcessors).
 */
import type {
  AbsenceNotifyPerson,
  Establishment,
  EstablishmentKind,
  NotificationsConfig,
} from "@/app/lib/app-config-schemas";
import type { AbsenceRecord, AbsenceScope } from "@/app/lib/absences-types";
import {
  normalizeOgecValidatorRef,
  parsePersonnelAbsenceManager,
  type OgecAbsenceValidatorRef,
} from "@/app/lib/absences-ogec-validators-shared";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import {
  directionRolesMatchEstablishmentRef,
  matchEstablishment,
} from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

/** Scope local (évite import circulaire absences-types ↔ routing). */
function absenceScope(
  abs: Pick<AbsenceRecord, "data"> & { source?: AbsenceRecord["source"] },
): AbsenceScope {
  if (abs.data.scope === "ogec" || abs.data.scope === "professeur") return abs.data.scope;
  if (abs.data.etablissement) return "professeur";
  if (abs.source === "admin_manual" || abs.source === "admin_pdf") return "professeur";
  return "ogec";
}

export type AbsenceValidatorRef = {
  email: string;
  userId?: string;
  label?: string;
};

function personToRef(
  p: Pick<AbsenceNotifyPerson, "email" | "userId" | "label"> | null | undefined,
): AbsenceValidatorRef | null {
  const email = String(p?.email || "")
    .trim()
    .toLowerCase();
  if (!email && !String(p?.userId || "").trim()) return null;
  return {
    email: email || "",
    userId: String(p?.userId || "").trim() || undefined,
    label: String(p?.label || "").trim() || undefined,
  };
}

function directorRefFromEstablishment(est: Establishment | null | undefined): AbsenceValidatorRef | null {
  if (!est) return null;
  const email = String(est.directorEmail || "")
    .trim()
    .toLowerCase();
  const userId = String(est.directorExternalUserId || "").trim() || undefined;
  if (!email && !userId) return null;
  return {
    email: email || "",
    userId,
    label: String(est.directorName || "").trim() || est.label || undefined,
  };
}

function establishmentForKind(
  establishments: Establishment[],
  kind: EstablishmentKind,
): Establishment | null {
  return (
    getActiveEstablishments(establishments).find((e) => inferEstablishmentKind(e) === kind) || null
  );
}

/** Validateurs OGEC configurés (liste paramétrage). */
export function configuredOgecValidators(
  notifications: NotificationsConfig | null | undefined,
): AbsenceValidatorRef[] {
  const list = notifications?.absencesValidatorsOgec;
  if (!Array.isArray(list)) return [];
  return list.map(personToRef).filter((p): p is AbsenceValidatorRef => Boolean(p));
}

/** Validateurs professeurs configurés pour un cycle. */
export function configuredProfValidatorsForKind(
  notifications: NotificationsConfig | null | undefined,
  kind: EstablishmentKind,
): AbsenceValidatorRef[] {
  const n = notifications;
  if (!n) return [];
  const raw =
    kind === "ecole"
      ? n.absencesValidatorsProfEcole
      : kind === "college"
        ? n.absencesValidatorsProfCollege
        : kind === "lycee"
          ? n.absencesValidatorsProfLycee
          : undefined;
  if (!Array.isArray(raw)) return [];
  return raw.map(personToRef).filter((p): p is AbsenceValidatorRef => Boolean(p));
}

/**
 * File de validation effective pour une absence.
 * Priorité OGEC : snapshot / fiche → liste paramétrage → directeur lycée (Établissements).
 * Priorité prof : liste paramétrage du cycle → directeur de l’établissement.
 */
export function resolveAbsenceValidationQueue(
  abs: Pick<AbsenceRecord, "data" | "createdBy" | "personnelId">,
  notifications: NotificationsConfig | null | undefined,
  establishments: Establishment[],
  opts?: { ficheManagerId?: string | null },
): AbsenceValidatorRef[] {
  const scope = absenceScope(abs as AbsenceRecord);

  if (scope === "ogec") {
    const configured = configuredOgecValidators(notifications);
    const lyceeDir = directorRefFromEstablishment(establishmentForKind(establishments, "lycee"));
    const defaults: AbsenceValidatorRef[] =
      configured.length > 0 ? configured : lyceeDir ? [lyceeDir] : [];

    const fromFiche = parsePersonnelAbsenceManager(opts?.ficheManagerId);
    const snapshot = normalizeOgecValidatorRef(abs.data?.ogecValidator);
    const personal: AbsenceValidatorRef | null = fromFiche
      ? {
          email: fromFiche.email || "",
          userId: fromFiche.userId,
          label: fromFiche.label,
        }
      : snapshot
        ? {
            email: snapshot.email || "",
            userId: snapshot.userId,
            label: snapshot.label,
          }
        : null;

    if (personal && (personal.email || personal.userId)) {
      const matchesDefault = defaults.some(
        (d) =>
          (personal.email && d.email && personal.email === d.email) ||
          (personal.userId && d.userId && personal.userId === d.userId),
      );
      // Exception fiche / snapshot hors défaut → uniquement cette personne
      if (!matchesDefault) return [personal];
    }
    return defaults;
  }

  // Professeurs — par cycle
  const est = matchEstablishment(establishments, abs.data.etablissement);
  const kind = est
    ? inferEstablishmentKind(est)
    : inferEstablishmentKind({ label: abs.data.etablissement || "" });
  const configured = configuredProfValidatorsForKind(notifications, kind);
  if (configured.length > 0) return configured;
  const director = directorRefFromEstablishment(est || establishmentForKind(establishments, kind));
  return director ? [director] : [];
}

export function viewerMatchesValidators(
  validators: AbsenceValidatorRef[],
  viewer: {
    email?: string | null;
    userId?: string | null;
    userIds?: Array<string | null | undefined>;
  },
): boolean {
  const email = String(viewer.email || "")
    .trim()
    .toLowerCase();
  const ids = new Set<string>();
  for (const raw of [viewer.userId, ...(viewer.userIds ?? [])]) {
    const s = String(raw || "").trim();
    if (s) ids.add(s);
  }
  return validators.some((v) => {
    if (email && v.email && v.email.trim().toLowerCase() === email) return true;
    const vid = String(v.userId || "").trim();
    if (vid && ids.has(vid)) return true;
    return false;
  });
}

/**
 * Le viewer peut-il valider cette absence ?
 * Config d’abord ; repli rôle direction du cycle pour les profs si la file
 * est le directeur d’établissement (compat comptes direction_*).
 */
export function viewerCanValidateAbsence(
  abs: Pick<AbsenceRecord, "data" | "createdBy" | "personnelId">,
  roles: string[],
  ctx: {
    establishments: Establishment[];
    notifications?: NotificationsConfig | null;
    email?: string | null;
    userId?: string | null;
    userIds?: Array<string | null | undefined>;
    ficheManagerId?: string | null;
  },
): boolean {
  const queue = resolveAbsenceValidationQueue(abs, ctx.notifications, ctx.establishments, {
    ficheManagerId: ctx.ficheManagerId,
  });
  if (viewerMatchesValidators(queue, ctx)) return true;

  const scope = absenceScope(abs as AbsenceRecord);
  if (scope === "professeur") {
    // Compat : rôle direction du cycle même si e-mail ≠ directorEmail configuré
    return directionRolesMatchEstablishmentRef(
      roles,
      abs.data.etablissement,
      ctx.establishments,
      ctx.userId,
    );
  }

  // OGEC : si la file = directeur lycée (repli Établissements) et le viewer a
  // le rôle direction_lycee / direction générique — même sans match e-mail.
  if (queue.length === 0) return false;
  const lycee = establishmentForKind(ctx.establishments, "lycee");
  const lyceeDir = directorRefFromEstablishment(lycee);
  const queueIsLyceeDirector =
    Boolean(lyceeDir) &&
    queue.every(
      (v) =>
        (lyceeDir!.email && v.email && v.email === lyceeDir!.email) ||
        (lyceeDir!.userId && v.userId && v.userId === lyceeDir!.userId),
    );
  if (!queueIsLyceeDirector) return false;
  const spaced = roles.map((r) =>
    String(r)
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[_-]+/g, " "),
  );
  const isLyceeRole = spaced.some(
    (r) => r.includes("direction lycee") || r === "direction" || r.includes("directionlycee"),
  );
  const isCycleEcoleOrCollege = spaced.some(
    (r) => r.includes("direction ecole") || r.includes("direction college"),
  );
  return isLyceeRole && !isCycleEcoleOrCollege;
}

/** @deprecated alias — snapshots OGEC */
export function ogecValidatorRefsToNotify(
  refs: AbsenceValidatorRef[],
): OgecAbsenceValidatorRef[] {
  return refs.map((r) => ({
    email: r.email,
    userId: r.userId,
    label: r.label,
  }));
}
