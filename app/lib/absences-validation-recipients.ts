import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { isEducationSurveillanceStaff } from "@/app/lib/absences-types";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

export type AbsenceProcessorRef = {
  email: string;
  userId?: string;
  label?: string;
};

function addProcessor(
  list: AbsenceProcessorRef[],
  seen: Set<string>,
  person?: { email?: string; userId?: string; label?: string } | null,
) {
  const email = String(person?.email || "").trim().toLowerCase();
  if (!email || seen.has(email)) return;
  seen.add(email);
  const userId = String(person?.userId || "").trim() || undefined;
  const label = String(person?.label || "").trim() || undefined;
  list.push({ email, userId, label });
}

function addEmailProcessor(list: AbsenceProcessorRef[], seen: Set<string>, email: string | undefined | null) {
  addProcessor(list, seen, { email: email || undefined });
}

function addAllProfProcessors(
  list: AbsenceProcessorRef[],
  seen: Set<string>,
  n: NotificationsConfig,
) {
  addProcessor(list, seen, n.absencesNotifyProfEcole);
  addProcessor(list, seen, n.absencesNotifyProfCollege);
  addProcessor(list, seen, n.absencesNotifyProfLycee);
  addProcessor(list, seen, n.absencesNotifyProfCollegeLycee);
}

/**
 * Personnes qui traitent l’absence après validation direction
 * (rectorat / ONISE pour les profs, RH / compta pour l’OGEC).
 */
export function collectAbsenceProcessors(
  record: Pick<AbsenceRecord, "data" | "createdBy">,
  notifications: NotificationsConfig,
  establishments: Establishment[],
): AbsenceProcessorRef[] {
  const list: AbsenceProcessorRef[] = [];
  const seen = new Set<string>();
  const n = notifications;

  if (record.data.scope === "ogec") {
    for (const e of n.absencesNotifyOgecCompta || []) addEmailProcessor(list, seen, e);
    if (isEducationSurveillanceStaff(record.createdBy.roles)) {
      for (const e of n.absencesNotifySurveillanceResponsables || []) addEmailProcessor(list, seen, e);
    }
    return list;
  }

  const est = matchEstablishment(establishments, record.data.etablissement);
  const kind = est
    ? inferEstablishmentKind(est)
    : inferEstablishmentKind({ label: record.data.etablissement || "" });

  if (kind === "ecole") {
    addProcessor(list, seen, n.absencesNotifyProfEcole);
  } else if (kind === "college") {
    addProcessor(list, seen, n.absencesNotifyProfCollege);
    if (list.length === 0) addProcessor(list, seen, n.absencesNotifyProfCollegeLycee);
  } else if (kind === "lycee") {
    addProcessor(list, seen, n.absencesNotifyProfLycee);
    if (list.length === 0) addProcessor(list, seen, n.absencesNotifyProfCollegeLycee);
  }

  if (list.length === 0) addAllProfProcessors(list, seen, n);
  return list;
}

export function collectAbsenceValidationEmails(
  record: Pick<AbsenceRecord, "data" | "createdBy">,
  notifications: NotificationsConfig,
  establishments: Establishment[],
): string[] {
  return collectAbsenceProcessors(record, notifications, establishments).map((p) => p.email);
}

export function isConfiguredAbsenceProcessor(
  viewer: { email?: string | null; userId?: string | null },
  notifications: NotificationsConfig,
): boolean {
  const email = String(viewer.email || "").trim().toLowerCase();
  const userId = String(viewer.userId || "").trim();
  const people = [
    notifications.absencesNotifyProfEcole,
    notifications.absencesNotifyProfCollege,
    notifications.absencesNotifyProfLycee,
    notifications.absencesNotifyProfCollegeLycee,
  ];
  if (
    people.some((p) => {
      if (email && p?.email && p.email.trim().toLowerCase() === email) return true;
      if (userId && p?.userId && p.userId === userId) return true;
      return false;
    })
  ) {
    return true;
  }
  const extra = [
    ...(notifications.absencesNotifyOgecCompta || []),
    ...(notifications.absencesNotifySurveillanceResponsables || []),
  ];
  return extra.some((e) => email && e.trim().toLowerCase() === email);
}
