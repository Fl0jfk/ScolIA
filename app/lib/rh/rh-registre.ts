import "server-only";

import { RH_MEDECINE_INTERVAL_YEARS, computeNextMedecineDue } from "@/app/lib/personnel-rh-cycles";
import { daysUntil, isExpiringWithinDays, isOverdue } from "@/app/lib/personnel-types";
import { readMetaRhByFolderName, readRhPersonnelIndex } from "@/app/lib/rh/meta-storage";
import type {
  RhRegistreAlert,
  RhRegistrePayload,
  RhRegistreRow,
  RhRegistreUrgency,
  RhSkillBucket,
} from "@/app/lib/rh/rh-registre-types";
import {
  RH_CATEGORY_LABELS,
  computeRhComplianceFlags,
  type MetaRhDocument,
  type RhPersonnelIndexEntry,
} from "@/app/lib/rh/types";

const HAB_ALERT_DAYS = 90;
const MED_ALERT_DAYS = 45;

function sortAlerts(alerts: RhRegistreAlert[]): RhRegistreAlert[] {
  const order: Record<RhRegistreUrgency, number> = { high: 0, medium: 1, low: 2 };
  return alerts.sort((a, b) => {
    const u = order[a.urgency] - order[b.urgency];
    if (u !== 0) return u;
    if (a.dueDate && b.dueDate) return new Date(a.dueDate).getTime() - new Date(b.dueDate).getTime();
    return a.displayName.localeCompare(b.displayName, "fr");
  });
}

function resolveMedecineNext(meta: MetaRhDocument): string | null {
  const med = meta.medecineTravail;
  if (med.nextVisitAt?.trim()) return med.nextVisitAt.trim();
  if (med.lastVisitAt?.trim()) return computeNextMedecineDue(med.lastVisitAt.trim());
  const latest = [...(med.visits || [])].sort(
    (a, b) => new Date(b.visitedAt).getTime() - new Date(a.visitedAt).getTime(),
  )[0];
  if (latest?.visitedAt) return computeNextMedecineDue(latest.visitedAt);
  return null;
}

function buildRowFromMeta(entry: RhPersonnelIndexEntry, meta: MetaRhDocument): RhRegistreRow {
  const flags = computeRhComplianceFlags(meta);
  return {
    id: meta.id || entry.id,
    folderName: entry.folderName,
    displayName:
      `${meta.identity.firstName} ${meta.identity.lastName}`.trim() ||
      entry.displayName ||
      entry.folderName,
    email: meta.identity.email || entry.email,
    category: meta.category || entry.category,
    categoryLabel: RH_CATEGORY_LABELS[meta.category || entry.category] || entry.category,
    active: meta.active !== false,
    accountStatus: meta.accountStatus,
    jobTitle: meta.contract.jobTitle || null,
    hireDate: meta.hireDate || entry.hireDate || null,
    birthDate: meta.identity.birthDate || null,
    socialSecurityNumber: meta.identity.socialSecurityNumber || null,
    contractType: meta.contract.type || null,
    missingSocialSecurity: !!flags.missingSocialSecurity,
    missingContractType: !!flags.missingContractType,
    missingBirthDate: !!flags.missingBirthDate,
    hasMissingData:
      !!flags.missingSocialSecurity || !!flags.missingContractType || !!flags.missingBirthDate,
    medecineNextVisitAt: resolveMedecineNext(meta),
    medecineLastVisitAt: meta.medecineTravail.lastVisitAt || null,
    habilitationsCount: meta.habilitations?.length ?? 0,
    formationsCount: meta.formations?.length ?? 0,
    updatedAt: meta.updatedAt || null,
    metaFound: true,
  };
}

function buildRowMissingMeta(entry: RhPersonnelIndexEntry, error?: string): RhRegistreRow {
  return {
    id: entry.id,
    folderName: entry.folderName,
    displayName: entry.displayName || entry.folderName,
    email: entry.email,
    category: entry.category,
    categoryLabel: RH_CATEGORY_LABELS[entry.category] || entry.category,
    active: entry.active !== false,
    accountStatus: entry.accountStatus,
    jobTitle: null,
    hireDate: entry.hireDate || null,
    birthDate: null,
    socialSecurityNumber: null,
    contractType: null,
    missingSocialSecurity: true,
    missingContractType: true,
    missingBirthDate: true,
    hasMissingData: true,
    medecineNextVisitAt: null,
    medecineLastVisitAt: null,
    habilitationsCount: 0,
    formationsCount: 0,
    updatedAt: null,
    metaFound: false,
    metaError: error,
  };
}

function collectAlerts(entry: RhPersonnelIndexEntry, meta: MetaRhDocument): RhRegistreAlert[] {
  const alerts: RhRegistreAlert[] = [];
  const displayName =
    `${meta.identity.firstName} ${meta.identity.lastName}`.trim() || entry.displayName;
  const personnelId = meta.id || entry.id;
  const folderName = entry.folderName;
  const flags = computeRhComplianceFlags(meta);

  if (flags.missingSocialSecurity) {
    alerts.push({
      id: `conf-nir-${personnelId}`,
      kind: "conformite",
      personnelId,
      displayName,
      folderName,
      title: "N° sécu manquant",
      detail: "Numéro de sécurité sociale non renseigné",
      urgency: "high",
    });
  }
  if (flags.missingContractType) {
    alerts.push({
      id: `conf-contrat-${personnelId}`,
      kind: "conformite",
      personnelId,
      displayName,
      folderName,
      title: "Type de contrat manquant",
      detail: "CDI / CDD / … non renseigné",
      urgency: "high",
    });
  }
  if (flags.missingBirthDate) {
    alerts.push({
      id: `conf-naissance-${personnelId}`,
      kind: "conformite",
      personnelId,
      displayName,
      folderName,
      title: "Date de naissance manquante",
      detail: "À compléter dans le dossier",
      urgency: "medium",
    });
  }

  for (const h of meta.habilitations || []) {
    if (!h.expiresAt) continue;
    if (!isOverdue(h.expiresAt) && !isExpiringWithinDays(h.expiresAt, HAB_ALERT_DAYS)) continue;
    const d = daysUntil(h.expiresAt);
    const isSst = /sst|secu|sécurité|incendie|habilitation|caces|electri/i.test(h.label);
    alerts.push({
      id: `hab-${personnelId}-${h.id}`,
      kind: "habilitation",
      personnelId,
      displayName,
      folderName,
      title: isSst ? `Recyclage ${h.label}` : h.label,
      detail:
        d !== null && d < 0
          ? `Expirée depuis ${Math.abs(d)} j — renouvellement urgent`
          : `Expire dans ${d} j`,
      urgency: d !== null && d <= 30 ? "high" : "medium",
      dueDate: h.expiresAt,
    });
  }

  for (const f of meta.formations || []) {
    if (f.status !== "planifiee" && f.status !== "demandee") continue;
    const due = f.plannedDate || f.reminderAt || null;
    const d = daysUntil(due);
    alerts.push({
      id: `form-${personnelId}-${f.id}`,
      kind: "formation",
      personnelId,
      displayName,
      folderName,
      title: f.title,
      detail:
        f.status === "demandee"
          ? due
            ? `Demandée · souhait ${new Date(due).toLocaleDateString("fr-FR")}`
            : "Demande en attente de planification"
          : due
            ? d !== null && d < 0
              ? `Prévue le ${new Date(due).toLocaleDateString("fr-FR")} · en retard`
              : `Prévue le ${new Date(due).toLocaleDateString("fr-FR")}`
            : "Planifiée — date à confirmer",
      urgency:
        f.status === "demandee"
          ? "medium"
          : d !== null && d <= 14
            ? "high"
            : d !== null && d <= 60
              ? "medium"
              : "low",
      dueDate: due,
    });
  }

  const nextMed = resolveMedecineNext(meta);
  if (nextMed && (isOverdue(nextMed) || isExpiringWithinDays(nextMed, MED_ALERT_DAYS))) {
    const d = daysUntil(nextMed);
    alerts.push({
      id: `med-${personnelId}`,
      kind: "medecine",
      personnelId,
      displayName,
      folderName,
      title: meta.medecineTravail.visitType || "Médecine du travail",
      detail:
        d !== null && d < 0
          ? `Visite en retard de ${Math.abs(d)} j (cycle ${RH_MEDECINE_INTERVAL_YEARS} ans)`
          : `Prochaine visite dans ${d} j`,
      urgency: d !== null && d <= 14 ? "high" : "medium",
      dueDate: nextMed,
    });
  } else if (!nextMed && meta.active !== false) {
    alerts.push({
      id: `med-missing-${personnelId}`,
      kind: "medecine",
      personnelId,
      displayName,
      folderName,
      title: "Médecine du travail",
      detail: "Aucune visite / échéance renseignée",
      urgency: "low",
    });
  }

  for (const e of meta.entretiens || []) {
    if (e.status === "realise") continue;
    const due = e.scheduledAt || e.reminderAt || e.nextDueAt || null;
    alerts.push({
      id: `ent-${personnelId}-${e.id}`,
      kind: "entretien",
      personnelId,
      displayName,
      folderName,
      title: "Entretien professionnel",
      detail:
        e.status === "a_planifier"
          ? "À planifier"
          : due
            ? `Prévu le ${new Date(due).toLocaleDateString("fr-FR")}`
            : "Planifié",
      urgency: due && isOverdue(due) ? "high" : "medium",
      dueDate: due,
    });
  }

  return alerts;
}

function collectSkills(
  entry: RhPersonnelIndexEntry,
  meta: MetaRhDocument,
  map: Map<string, RhSkillBucket>,
) {
  const person = {
    id: meta.id || entry.id,
    displayName:
      `${meta.identity.firstName} ${meta.identity.lastName}`.trim() || entry.displayName,
    folderName: entry.folderName,
  };

  for (const h of meta.habilitations || []) {
    const label = (h.label || "").trim();
    if (!label) continue;
    const key = `hab:${label.toLowerCase()}`;
    const prev = map.get(key);
    if (prev) {
      if (!prev.people.some((p) => p.id === person.id)) {
        prev.people.push(person);
        prev.count = prev.people.length;
      }
    } else {
      map.set(key, { label, kind: "habilitation", count: 1, people: [person] });
    }
  }

  for (const f of meta.formations || []) {
    if (f.status !== "realisee") continue;
    const label = (f.title || "").trim();
    if (!label) continue;
    const key = `form:${label.toLowerCase()}`;
    const prev = map.get(key);
    if (prev) {
      if (!prev.people.some((p) => p.id === person.id)) {
        prev.people.push(person);
        prev.count = prev.people.length;
      }
    } else {
      map.set(key, { label, kind: "formation", count: 1, people: [person] });
    }
  }
}

/** Agrège index + meta-rh OneDrive → registre + alertes + compétences. */
export async function buildRhRegistre(): Promise<
  { ok: true; data: RhRegistrePayload } | { ok: false; error: string; code: string }
> {
  const indexHit = await readRhPersonnelIndex();
  if (!indexHit.ok) return { ok: false, error: indexHit.error, code: indexHit.code };

  const entries = indexHit.index.entries.filter((e) => e.active !== false);
  const metas = await Promise.all(
    entries.map(async (entry) => {
      const hit = await readMetaRhByFolderName(entry.folderName);
      return { entry, hit };
    }),
  );

  const rows: RhRegistreRow[] = [];
  const alerts: RhRegistreAlert[] = [];
  const skillMap = new Map<string, RhSkillBucket>();
  let metaLoaded = 0;
  let metaMissing = 0;

  for (const { entry, hit } of metas) {
    if (!hit.ok) {
      metaMissing += 1;
      rows.push(buildRowMissingMeta(entry, hit.error));
      alerts.push({
        id: `meta-missing-${entry.id}`,
        kind: "conformite",
        personnelId: entry.id,
        displayName: entry.displayName || entry.folderName,
        folderName: entry.folderName,
        title: "meta-rh.json manquant",
        detail: hit.error || "Dossier OneDrive incomplet",
        urgency: "high",
      });
      continue;
    }
    metaLoaded += 1;
    rows.push(buildRowFromMeta(entry, hit.meta));
    alerts.push(...collectAlerts(entry, hit.meta));
    collectSkills(entry, hit.meta, skillMap);
  }

  rows.sort((a, b) => a.displayName.localeCompare(b.displayName, "fr"));
  const sortedAlerts = sortAlerts(alerts);
  const skills = [...skillMap.values()].sort(
    (a, b) => b.count - a.count || a.label.localeCompare(b.label, "fr"),
  );

  return {
    ok: true,
    data: {
      basePath: indexHit.basePath,
      generatedAt: new Date().toISOString(),
      medecineIntervalYears: RH_MEDECINE_INTERVAL_YEARS,
      rows,
      alerts: sortedAlerts,
      skills,
      counts: {
        staff: rows.length,
        metaLoaded,
        metaMissing,
        withMissingData: rows.filter((r) => r.hasMissingData).length,
        alertsHigh: sortedAlerts.filter((a) => a.urgency === "high").length,
        alertsTotal: sortedAlerts.length,
      },
    },
  };
}
