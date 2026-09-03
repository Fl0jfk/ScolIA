import "server-only";

import { and, desc, eq, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { absence, eleve, enseignant, personnel } from "@/db/schema";
import { loadAppConfig } from "@/app/lib/app-config";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { normalizeAbsencePeriodInput } from "@/app/lib/absence-period";
import { saveAbsenceRecord } from "@/app/lib/absences-storage";
import { computeStartEndAt, type AbsenceRecord, type AbsenceScope } from "@/app/lib/absences-types";
import { notifyAbsenceCreated } from "@/app/lib/absences-workflow-mail";
import { getAbsenceSyncPort } from "@/app/lib/absences-sync/port";
import { listMembersFromDb } from "@/app/lib/members-db";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import {
  asCycle,
  asDateKey,
  cycleLabel,
  datesOverlap,
  resolveEleveCycle,
  timesOverlap,
  type AccueilAbsenceCanal,
  type AccueilBoardKind,
  type AccueilBoardRow,
  type AccueilEleveNature,
  type AccueilPeriodMode,
  type AccueilPersonKind,
} from "@/app/lib/accueil-absences-types";
import {
  cancelAccueilEleveAbsence,
  createAbsenceAccueilEleve,
  listAccueilEleveAbsencesForDate,
} from "@/app/lib/vs-absences-db";

export type AccueilDeclareInput = {
  kind: AccueilPersonKind;
  subjectId: string;
  mode: AccueilPeriodMode;
  startDate: string;
  endDate: string;
  startTime?: string | null;
  endTime?: string | null;
  motif?: string | null;
  canal?: AccueilAbsenceCanal;
  /** Élèves uniquement : absence (défaut) ou retard. */
  eleveNature?: AccueilEleveNature;
  actor: {
    userId: string;
    name: string;
    email: string;
    roles: string[];
  };
};

function periodFromMode(input: AccueilDeclareInput) {
  if (input.mode === "hours") {
    return normalizeAbsencePeriodInput({
      periodType: "single_day",
      startDate: input.startDate,
      endDate: input.startDate,
      startTime: input.startTime,
      endTime: input.endTime,
    });
  }
  if (input.mode === "today") {
    return normalizeAbsencePeriodInput({
      periodType: "multi_day",
      startDate: input.startDate,
      endDate: input.startDate,
    });
  }
  return normalizeAbsencePeriodInput({
    periodType: "multi_day",
    startDate: input.startDate,
    endDate: input.endDate,
  });
}

function formatPeriodSubtitle(row: {
  dateDebut: string;
  dateFin: string;
  heureDebut: string | null;
  heureFin: string | null;
}): string {
  const start = row.dateDebut;
  const end = row.dateFin;
  if (row.heureDebut && row.heureFin && start === end) {
    return `${start} · ${row.heureDebut}–${row.heureFin}`;
  }
  if (start === end) return start;
  return `${start} → ${end}`;
}

async function resolveProfSiteLabel(secteurOrLabel: string | null | undefined): Promise<string> {
  const bundle = await loadAppConfig();
  const active = getActiveEstablishments(bundle.establishments);
  const cycle = asCycle(secteurOrLabel);
  if (cycle) {
    const hit = active.find((e) => inferEstablishmentKind(e) === cycle);
    if (hit?.label) return hit.label;
    if (hit?.id) return hit.id;
    const named = cycleLabel(cycle);
    if (named) return named;
  }
  if (secteurOrLabel?.trim()) return secteurOrLabel.trim();
  return active[0]?.label || active[0]?.id || "Établissement";
}

async function findRhOverlap(input: {
  etablissementId: string;
  personnelId?: string | null;
  enseignantId?: string | null;
  displayName: string;
  startDate: string;
  endDate: string;
  startTime?: string | null;
  endTime?: string | null;
}): Promise<boolean> {
  const db = getDb();
  const ident = [];
  if (input.personnelId) ident.push(eq(absence.personnelId, input.personnelId));
  if (input.enseignantId) ident.push(eq(absence.enseignantId, input.enseignantId));
  ident.push(eq(absence.displayName, input.displayName));
  const rows = await db
    .select({
      startDate: absence.startDate,
      endDate: absence.endDate,
      startTime: absence.startTime,
      endTime: absence.endTime,
      managerDecision: absence.managerDecision,
      source: absence.source,
    })
    .from(absence)
    .where(and(eq(absence.etablissementId, input.etablissementId), or(...ident)))
    .limit(80);
  return rows.some(
    (r) =>
      r.managerDecision !== "REFUSEE" &&
      datesOverlap(String(r.startDate), String(r.endDate), input.startDate, input.endDate) &&
      timesOverlap(r.startTime, r.endTime, input.startTime, input.endTime),
  );
}

async function createRhAccueilAbsence(input: {
  etablissementId: string;
  scope: AbsenceScope;
  displayName: string;
  siteLabel: string | null;
  personnelId?: string | null;
  enseignantId?: string | null;
  subjectUserId?: string;
  subjectEmail?: string;
  subjectRoles?: string[];
  period: NonNullable<ReturnType<typeof normalizeAbsencePeriodInput>["data"]>;
  motif: string;
  actor: AccueilDeclareInput["actor"];
}): Promise<{ id: string; displayName: string; pendingDirection: true }> {
  const { startAt, endAt } = computeStartEndAt({
    periodType: input.period.periodType,
    startDate: input.period.startDate,
    endDate: input.period.endDate,
    startTime: input.period.startTime,
    endTime: input.period.endTime,
  });
  const now = new Date().toISOString();
  const id = `accueil_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  const record: AbsenceRecord = {
    id,
    createdAt: now,
    updatedAt: now,
    source: "accueil",
    displayName: input.displayName,
    calendarVisible: false,
    createdBy: {
      userId: input.subjectUserId || "",
      name: input.displayName,
      email: input.subjectEmail || "",
      roles: input.subjectRoles || [],
    },
    submittedBy: {
      userId: input.actor.userId,
      name: input.actor.name,
      email: input.actor.email,
      roles: input.actor.roles,
    },
    personnelId: input.personnelId || null,
    enseignantId: input.enseignantId || null,
    data: {
      scope: input.scope,
      etablissement: input.scope === "ogec" ? null : input.siteLabel,
      periodType: input.period.periodType,
      startDate: input.period.startDate,
      endDate: input.period.endDate,
      startTime: input.period.startTime ?? null,
      endTime: input.period.endTime ?? null,
      startAt,
      endAt,
      reason: input.motif,
      details: "Déclaré à l’accueil (standard).",
    },
    workflowStatus: "OUVERTE",
    managerDecision: "EN_ATTENTE",
    closedAt: null,
    history: [
      {
        at: now,
        by: input.actor.name,
        action: "CREATION",
        note: `Saisie accueil pour ${input.displayName} — en attente de validation direction`,
      },
    ],
  };

  // Upsert unitaire uniquement — pas de saveAbsenceIndex (DELETE+réinsert)
  // ni de fusion fuzzy avec self/PDF (sinon source ≠ accueil → board vide).
  const saved = await saveAbsenceRecord(record);
  try {
    await notifyAbsenceCreated({
      record: saved,
      actorName: input.actor.name,
      fromAccueil: true,
    });
  } catch (err) {
    console.error("Accueil absences — mail direction:", err);
  }
  void getAbsenceSyncPort().pushAbsence({
    etablissementId: input.etablissementId,
    kind: input.scope === "ogec" ? "ogec" : "professeur",
    subjectId: input.personnelId || input.enseignantId || saved.id,
    displayName: saved.displayName,
    dateDebut: saved.data.startDate,
    dateFin: saved.data.endDate,
    heureDebut: saved.data.startTime,
    heureFin: saved.data.endTime,
    motif: saved.data.reason,
    source: "accueil",
  });
  return { id: saved.id, displayName: saved.displayName, pendingDirection: true };
}

export async function declareAccueilAbsence(
  etablissementId: string,
  input: AccueilDeclareInput,
): Promise<{
  id: string;
  kind: AccueilBoardKind;
  displayName: string;
  pendingDirection: boolean;
}> {
  const periodResult = periodFromMode(input);
  if (periodResult.error || !periodResult.data) {
    throw new Error(periodResult.error || "Période invalide.");
  }
  const period = periodResult.data;
  const motif = input.motif?.trim() || "Absence déclarée à l’accueil";

  if (input.kind === "eleve") {
    const db = getDb();
    const [row] = await db
      .select({
        id: eleve.id,
        nom: eleve.nom,
        prenom: eleve.prenom,
        classe: eleve.classe,
      })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, input.subjectId)))
      .limit(1);
    if (!row) throw new Error("Élève introuvable.");
    const nature: AccueilEleveNature = input.eleveNature === "retard" ? "retard" : "absence";
    const periodForEleve =
      nature === "retard"
        ? periodFromMode({
            ...input,
            mode: "hours",
            startTime: input.startTime || "08:00",
            endTime: input.endTime || "08:30",
          })
        : periodResult;
    if (periodForEleve.error || !periodForEleve.data) {
      throw new Error(periodForEleve.error || "Période invalide.");
    }
    const elevePeriod = periodForEleve.data;
    const created = await createAbsenceAccueilEleve(etablissementId, {
      eleveId: row.id,
      dateDebut: elevePeriod.startDate,
      dateFin: elevePeriod.endDate,
      heureDebut: elevePeriod.startTime,
      heureFin: elevePeriod.endTime,
      motif,
      canal: input.canal || "telephone",
      type: nature,
      createdByUserId: input.actor.userId,
      createdByNom: input.actor.name,
    });
    void getAbsenceSyncPort().pushAbsence({
      etablissementId,
      kind: "eleve",
      subjectId: row.id,
      displayName: `${row.prenom} ${row.nom}`.trim(),
      dateDebut: elevePeriod.startDate,
      dateFin: elevePeriod.endDate,
      heureDebut: elevePeriod.startTime,
      heureFin: elevePeriod.endTime,
      motif,
      source: "accueil",
    });
    return {
      id: created.id,
      kind: "eleve",
      displayName: `${row.prenom} ${row.nom}`.trim(),
      pendingDirection: false,
    };
  }

  if (input.kind === "enseignant") {
    const db = getDb();
    const [ensById] = await db
      .select()
      .from(enseignant)
      .where(and(eq(enseignant.etablissementId, etablissementId), eq(enseignant.id, input.subjectId)))
      .limit(1);

    let enseignantId: string | null = ensById?.id ?? null;
    let personnelId: string | null = null;
    let displayName = ensById ? `${ensById.prenom} ${ensById.nom}`.trim() : "";
    let siteLabel = ensById ? await resolveProfSiteLabel(ensById.secteur) : await resolveProfSiteLabel(null);
    let subjectUserId: string | undefined;
    let subjectEmail = ensById ? ensById.emailPro || ensById.email || "" : "";

    if (!ensById) {
      // Id = utilisateur Better-Auth (annuaire), pas une ligne du catalogue enseignant.
      const members = await listMembersFromDb(etablissementId);
      const member = members.find(
        (m) =>
          (m.externalUserId === input.subjectId || m.userId === input.subjectId) &&
          hasRole(normalizeIntranetRoles(m.roles), "professeur"),
      );
      if (!member) throw new Error("Professeur introuvable.");

      subjectUserId = member.externalUserId;
      subjectEmail = member.email || "";
      displayName =
        (member.displayName || "").trim() ||
        `${member.firstName || ""} ${member.lastName || ""}`.trim() ||
        member.email;

      const email = member.email.trim().toLowerCase();
      if (email) {
        const [ensByEmail] = await db
          .select()
          .from(enseignant)
          .where(
            and(
              eq(enseignant.etablissementId, etablissementId),
              sql`(lower(coalesce(${enseignant.email}, '')) = ${email} or lower(coalesce(${enseignant.emailPro}, '')) = ${email})`,
            ),
          )
          .limit(1);
        if (ensByEmail) {
          enseignantId = ensByEmail.id;
          siteLabel = await resolveProfSiteLabel(ensByEmail.secteur);
          if (!subjectEmail) subjectEmail = ensByEmail.emailPro || ensByEmail.email || "";
        }
      }

      const [pers] = await db
        .select()
        .from(personnel)
        .where(
          and(
            eq(personnel.etablissementId, etablissementId),
            eq(personnel.externalUserId, member.externalUserId),
            eq(personnel.active, true),
          ),
        )
        .limit(1);
      if (pers) {
        personnelId = pers.id;
        if (!subjectEmail) subjectEmail = pers.email || pers.emailPro || "";
      }
    }

    const clash = await findRhOverlap({
      etablissementId,
      enseignantId,
      personnelId,
      displayName,
      startDate: period.startDate,
      endDate: period.endDate,
      startTime: period.startTime,
      endTime: period.endTime,
    });
    if (clash) throw new Error("Une absence existe déjà sur cette période pour ce professeur.");
    const saved = await createRhAccueilAbsence({
      etablissementId,
      scope: "professeur",
      displayName,
      siteLabel,
      enseignantId,
      personnelId,
      subjectUserId,
      subjectEmail,
      subjectRoles: ["professeur"],
      period,
      motif,
      actor: input.actor,
    });
    return { id: saved.id, kind: "professeur", displayName, pendingDirection: true };
  }

  const db = getDb();
  const [pers] = await db
    .select()
    .from(personnel)
    .where(and(eq(personnel.etablissementId, etablissementId), eq(personnel.id, input.subjectId)))
    .limit(1);
  if (!pers) throw new Error("Personnel introuvable.");
  const displayName =
    pers.displayName?.trim() || `${pers.firstName} ${pers.lastName}`.trim();
  const categoryNorm = String(pers.category || "")
    .trim()
    .toLowerCase();
  const isProf =
    categoryNorm === "professeur" ||
    categoryNorm === "enseignant" ||
    categoryNorm === "teacher";
  const scope: AbsenceScope = isProf ? "professeur" : "ogec";
  const siteLabel = isProf ? await resolveProfSiteLabel(pers.establishmentLabel) : null;
  const clash = await findRhOverlap({
    etablissementId,
    personnelId: pers.id,
    displayName,
    startDate: period.startDate,
    endDate: period.endDate,
    startTime: period.startTime,
    endTime: period.endTime,
  });
  if (clash) throw new Error("Une absence existe déjà sur cette période pour cette personne.");
  const saved = await createRhAccueilAbsence({
    etablissementId,
    scope,
    displayName,
    siteLabel,
    personnelId: pers.id,
    subjectUserId: pers.externalUserId || undefined,
    subjectEmail: pers.email || pers.emailPro || "",
    subjectRoles: pers.category ? [pers.category] : [],
    period,
    motif,
    actor: input.actor,
  });
  return {
    id: saved.id,
    kind: isProf ? "professeur" : "ogec",
    displayName,
    pendingDirection: true,
  };
}

export async function listAccueilBoard(
  etablissementId: string,
  dateIso: string,
): Promise<AccueilBoardRow[]> {
  const day = asDateKey(dateIso);
  const eleves = await listAccueilEleveAbsencesForDate(etablissementId, day || dateIso);
  const studentRows: AccueilBoardRow[] = eleves.map((r) => {
    const nature: AccueilEleveNature = r.type === "retard" ? "retard" : "absence";
    const classe = r.eleveClasse?.trim() || null;
    const cycle = resolveEleveCycle({
      secteur: r.eleveSecteur,
      classe,
    });
    return {
      id: r.id,
      kind: "eleve" as const,
      displayName: `${r.elevePrenom} ${r.eleveNom}`.trim(),
      subtitle: [
        nature === "retard" ? "Retard" : "Absence",
        classe,
        cycleLabel(cycle) || null,
        formatPeriodSubtitle({
          dateDebut: asDateKey(r.dateDebut),
          dateFin: asDateKey(r.dateFin),
          heureDebut: r.heureDebut,
          heureFin: r.heureFin,
        }),
      ]
        .filter(Boolean)
        .join(" · "),
      dateDebut: asDateKey(r.dateDebut),
      dateFin: asDateKey(r.dateFin),
      heureDebut: r.heureDebut,
      heureFin: r.heureFin,
      motif: r.motif,
      createdByNom: r.createdByNom,
      source: "accueil",
      eleveNature: nature,
      cycle,
      classe,
    };
  });

  const db = getDb();
  const staff = await db
    .select()
    .from(absence)
    .where(and(eq(absence.etablissementId, etablissementId), eq(absence.source, "accueil")))
    .orderBy(desc(absence.startAt));

  const staffRows: AccueilBoardRow[] = staff
    .filter((r) => r.managerDecision !== "REFUSEE")
    .filter((r) =>
      day
        ? datesOverlap(asDateKey(r.startDate), asDateKey(r.endDate), day, day)
        : true,
    )
    .map((r) => {
      const kind: AccueilBoardKind = r.scope === "ogec" ? "ogec" : "professeur";
      const pending = r.managerDecision === "EN_ATTENTE";
      return {
        id: r.id,
        kind,
        displayName: r.displayName,
        subtitle: [
          kind === "ogec" ? "Personnel OGEC" : "Professeur",
          pending ? "En attente direction" : r.managerDecision === "VALIDEE" ? "Validée" : null,
          formatPeriodSubtitle({
            dateDebut: asDateKey(r.startDate),
            dateFin: asDateKey(r.endDate),
            heureDebut: r.startTime,
            heureFin: r.endTime,
          }),
        ]
          .filter(Boolean)
          .join(" · "),
        dateDebut: asDateKey(r.startDate),
        dateFin: asDateKey(r.endDate),
        heureDebut: r.startTime,
        heureFin: r.endTime,
        motif: r.reason,
        createdByNom: r.createdByName,
        source: "accueil",
      };
    });

  return [...studentRows, ...staffRows];
}

export async function cancelAccueilAbsence(
  etablissementId: string,
  id: string,
  actorName: string,
  noteCpe = "Annulée par l’accueil",
): Promise<boolean> {
  const eleveOk = await cancelAccueilEleveAbsence(etablissementId, id, noteCpe);
  if (eleveOk) return true;

  const db = getDb();
  const [row] = await db
    .select()
    .from(absence)
    .where(and(eq(absence.etablissementId, etablissementId), eq(absence.id, id)))
    .limit(1);
  if (!row || row.source !== "accueil") return false;
  if (row.managerDecision === "VALIDEE") {
    throw new Error("Absence déjà validée par la direction — annulation côté RH.");
  }

  const { getAbsenceFromDb } = await import("@/app/lib/absence-db");
  const record = await getAbsenceFromDb(etablissementId, id);
  if (!record) return false;
  const updated: AbsenceRecord = {
    ...record,
    managerDecision: "REFUSEE",
    workflowStatus: "CLOTUREE",
    calendarVisible: false,
    closedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    history: [
      ...record.history,
      {
        at: new Date().toISOString(),
        by: actorName,
        action: "ANNULATION_ACCUEIL",
        note: "Annulée depuis l’accueil avant validation direction",
      },
    ],
  };
  await saveAbsenceRecord(updated);
  return true;
}
