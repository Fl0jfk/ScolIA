import "server-only";

import type {
  AbsenceNotifyPerson,
  Establishment,
  NotificationsConfig,
} from "@/app/lib/app-config-schemas";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { normalizeAbsenceRecord, resolveAbsenceScope } from "@/app/lib/absences-types";
import {
  defaultOgecValidatorsFromConfig,
  normalizeOgecValidatorRef,
  parsePersonnelAbsenceManager,
  type OgecAbsenceValidatorRef,
} from "@/app/lib/absences-ogec-validators-shared";
import {
  findPersonnelByEmail,
  findPersonnelByExternalId,
  getPersonnelRecord,
} from "@/app/lib/personnel-storage";
import type { PersonnelRecord } from "@/app/lib/personnel-types";

export {
  serializePersonnelAbsenceManager,
  parsePersonnelAbsenceManager,
  normalizeOgecValidatorRef,
  resolveOgecValidatorsForAbsence,
  viewerMatchesOgecValidators,
  defaultOgecValidatorsFromConfig,
  lyceeEstablishment,
  type OgecAbsenceValidatorRef,
} from "@/app/lib/absences-ogec-validators-shared";

async function enrichValidatorPerson(
  person: OgecAbsenceValidatorRef | AbsenceNotifyPerson,
): Promise<OgecAbsenceValidatorRef | null> {
  let email = String(person.email || "")
    .trim()
    .toLowerCase();
  let userId = String(person.userId || "").trim() || undefined;
  let label = String(person.label || "").trim() || undefined;

  if (userId && !email) {
    const byUser = await findPersonnelByExternalId(userId);
    if (byUser?.email) {
      email = byUser.email.trim().toLowerCase();
      label = label || byUser.displayName;
    }
  }
  if (email && !userId) {
    const byEmail = await findPersonnelByEmail(email);
    if (byEmail?.externalUserId) {
      userId = byEmail.externalUserId;
      label = label || byEmail.displayName;
    }
  }
  if (!email && !userId) return null;
  return { email: email || "", userId, label };
}

export async function loadSubjectPersonnelForAbsence(
  record: Pick<AbsenceRecord, "personnelId" | "createdBy">,
): Promise<PersonnelRecord | null> {
  if (record.personnelId) {
    const byId = await getPersonnelRecord(record.personnelId);
    if (byId) return byId;
  }
  const userId = String(record.createdBy?.userId || "").trim();
  if (userId) {
    const byUser = await findPersonnelByExternalId(userId);
    if (byUser) return byUser;
  }
  const email = String(record.createdBy?.email || "")
    .trim()
    .toLowerCase();
  if (email) {
    return findPersonnelByEmail(email);
  }
  return null;
}

/**
 * Résout le validateur à snapshotter sur une nouvelle absence OGEC.
 * Priorité : fiche RH (`managerId`) → config globale / défaut lycée.
 */
export async function resolveOgecValidatorForNewAbsence(input: {
  personnel?: Pick<PersonnelRecord, "managerId"> | null;
  personnelId?: string | null;
  subjectUserId?: string | null;
  subjectEmail?: string | null;
  notifications?: NotificationsConfig | null;
  establishments: Establishment[];
}): Promise<OgecAbsenceValidatorRef | null> {
  let personnel = input.personnel || null;
  if (!personnel && input.personnelId) {
    personnel = await getPersonnelRecord(input.personnelId);
  }
  if (!personnel && input.subjectUserId) {
    personnel = await findPersonnelByExternalId(input.subjectUserId);
  }
  if (!personnel && input.subjectEmail) {
    personnel = await findPersonnelByEmail(input.subjectEmail);
  }

  const fromFiche = parsePersonnelAbsenceManager(personnel?.managerId);
  if (fromFiche) {
    const enriched = await enrichValidatorPerson(fromFiche);
    if (enriched) return enriched;
  }

  const defaults = defaultOgecValidatorsFromConfig(input.notifications, input.establishments);
  if (defaults[0]) {
    return (await enrichValidatorPerson(defaults[0])) || defaults[0];
  }
  return null;
}

/** Destinataires mail / décision pour une absence OGEC déjà créée ou en cours de création. */
export async function resolveOgecAbsenceDecisionPeople(input: {
  record?: Pick<AbsenceRecord, "personnelId" | "createdBy" | "data"> | null;
  notifications?: NotificationsConfig | null;
  establishments: Establishment[];
}): Promise<OgecAbsenceValidatorRef[]> {
  if (input.record) {
    const personnel = await loadSubjectPersonnelForAbsence(input.record);
    const fromFiche = parsePersonnelAbsenceManager(personnel?.managerId);
    if (fromFiche) {
      const enriched = await enrichValidatorPerson(fromFiche);
      if (enriched) return [enriched];
    }
  }

  const snapshot = normalizeOgecValidatorRef(input.record?.data?.ogecValidator);
  if (snapshot) {
    const enriched = await enrichValidatorPerson(snapshot);
    return enriched ? [enriched] : [snapshot];
  }

  const defaults = defaultOgecValidatorsFromConfig(input.notifications, input.establishments);
  const out: OgecAbsenceValidatorRef[] = [];
  for (const d of defaults) {
    const enriched = await enrichValidatorPerson(d);
    if (enriched) out.push(enriched);
  }
  return out.length > 0 ? out : defaults;
}

/**
 * Enrichit les absences OGEC avec le validateur issu de la fiche RH (`managerId`).
 * La fiche prime : changer le rattachement reclasse les dossiers encore en attente.
 */
export async function attachOgecValidatorsFromPersonnel(
  records: AbsenceRecord[],
): Promise<AbsenceRecord[]> {
  if (records.length === 0) return records;

  const personnelByKey = new Map<string, PersonnelRecord>();
  const loadKeys: Array<{ key: string; load: () => Promise<PersonnelRecord | null> }> = [];

  for (const raw of records) {
    const record = normalizeAbsenceRecord(raw);
    if (resolveAbsenceScope(record) !== "ogec") continue;
    if (record.personnelId) {
      const key = `id:${record.personnelId}`;
      if (!personnelByKey.has(key) && !loadKeys.some((k) => k.key === key)) {
        const id = record.personnelId;
        loadKeys.push({ key, load: () => getPersonnelRecord(id) });
      }
      continue;
    }
    const userId = String(record.createdBy?.userId || "").trim();
    if (userId) {
      const key = `user:${userId}`;
      if (!personnelByKey.has(key) && !loadKeys.some((k) => k.key === key)) {
        loadKeys.push({ key, load: () => findPersonnelByExternalId(userId) });
      }
      continue;
    }
    const email = String(record.createdBy?.email || "")
      .trim()
      .toLowerCase();
    if (email) {
      const key = `email:${email}`;
      if (!personnelByKey.has(key) && !loadKeys.some((k) => k.key === key)) {
        loadKeys.push({ key, load: () => findPersonnelByEmail(email) });
      }
    }
  }

  await Promise.all(
    loadKeys.map(async ({ key, load }) => {
      const pers = await load();
      if (pers) personnelByKey.set(key, pers);
    }),
  );

  const out: AbsenceRecord[] = [];
  for (const raw of records) {
    const record = normalizeAbsenceRecord(raw);
    if (resolveAbsenceScope(record) !== "ogec") {
      out.push(record);
      continue;
    }
    let personnel: PersonnelRecord | null = null;
    if (record.personnelId) personnel = personnelByKey.get(`id:${record.personnelId}`) || null;
    if (!personnel) {
      const userId = String(record.createdBy?.userId || "").trim();
      if (userId) personnel = personnelByKey.get(`user:${userId}`) || null;
    }
    if (!personnel) {
      const email = String(record.createdBy?.email || "")
        .trim()
        .toLowerCase();
      if (email) personnel = personnelByKey.get(`email:${email}`) || null;
    }

    const fromFiche = parsePersonnelAbsenceManager(personnel?.managerId);
    if (fromFiche) {
      const enriched = await enrichValidatorPerson(fromFiche);
      if (enriched) {
        out.push({
          ...record,
          personnelId: record.personnelId || personnel?.id || null,
          data: { ...record.data, ogecValidator: enriched },
        });
        continue;
      }
    }
    out.push(record);
  }
  return out;
}
