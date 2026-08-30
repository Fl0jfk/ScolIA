import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { isEducationSurveillanceStaff } from "@/app/lib/absences-types";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

function addEmail(set: Set<string>, email: string | undefined | null) {
  const v = String(email || "").trim().toLowerCase();
  if (v) set.add(v);
}

function addNotifyPerson(set: Set<string>, person?: { email?: string } | null) {
  addEmail(set, person?.email);
}

function addAllProfNotifyPeople(set: Set<string>, n: NotificationsConfig) {
  addNotifyPerson(set, n.absencesNotifyProfEcole);
  addNotifyPerson(set, n.absencesNotifyProfCollege);
  addNotifyPerson(set, n.absencesNotifyProfLycee);
  addNotifyPerson(set, n.absencesNotifyProfCollegeLycee);
}

/**
 * Destinataires après validation direction :
 * - professeurs → personne qui déclare au rectorat / ONISE (réglages Notifications, par cycle)
 * - OGEC → compta RH (+ responsables des surveillants si profil surveillance)
 *
 * Même circuit pour une auto-déclaration et une saisie accueil.
 * Si le cycle n’est pas reconnu ou si le destinataire du cycle n’est pas renseigné,
 * on notifie toutes les personnes « absences professeurs » configurées plutôt que
 * de laisser tomber la déclaration rectorat.
 */
export function collectAbsenceValidationEmails(
  record: Pick<AbsenceRecord, "data" | "createdBy">,
  notifications: NotificationsConfig,
  establishments: Establishment[],
): string[] {
  const emails = new Set<string>();
  const n = notifications;

  if (record.data.scope === "ogec") {
    for (const e of n.absencesNotifyOgecCompta || []) addEmail(emails, e);
    if (isEducationSurveillanceStaff(record.createdBy.roles)) {
      for (const e of n.absencesNotifySurveillanceResponsables || []) addEmail(emails, e);
    }
    return [...emails];
  }

  const est = matchEstablishment(establishments, record.data.etablissement);
  const kind = est
    ? inferEstablishmentKind(est)
    : inferEstablishmentKind({ label: record.data.etablissement || "" });

  if (kind === "ecole") {
    addNotifyPerson(emails, n.absencesNotifyProfEcole);
  } else if (kind === "college") {
    addNotifyPerson(emails, n.absencesNotifyProfCollege);
    if (emails.size === 0) addNotifyPerson(emails, n.absencesNotifyProfCollegeLycee);
  } else if (kind === "lycee") {
    addNotifyPerson(emails, n.absencesNotifyProfLycee);
    if (emails.size === 0) addNotifyPerson(emails, n.absencesNotifyProfCollegeLycee);
  }

  if (emails.size === 0) addAllProfNotifyPeople(emails, n);
  return [...emails];
}
