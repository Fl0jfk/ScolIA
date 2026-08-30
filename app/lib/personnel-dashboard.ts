import { getAbsenceIndex } from "@/app/lib/absences-storage";

import type { AbsenceRecord } from "@/app/lib/absences-types";

import {

  computeEntretienNextDue,

  entretienDueThisYear as isEntretienDueThisYear,

  hasOpenEntretienForCycle,

  medecineDueThisYear as isMedecineDueThisYear,

} from "@/app/lib/personnel-rh-cycles";

import {

  daysUntil,

  isExpiringWithinDays,

  isOverdue,

  type PersonnelRecord,

  type OnboardingStatus,

} from "@/app/lib/personnel-types";



export type DashboardAbsenceToday = {

  id: string;

  displayName: string;

  reason: string;

  periodLabel: string;

};



export type DashboardPersonnelItem = {

  id: string;

  displayName: string;

  category: string;

  detail: string;

  urgency: "high" | "medium" | "low";

  dueDate?: string | null;

  link: string;

};



export type PersonnelDashboardData = {

  counts: {

    onboardings: number;

    signatures: number;

    habilitations: number;

    formations: number;

    medecine: number;

    entretiens: number;

    absencesToday: number;

    medecineDueThisYear: number;

    medecineOverdue: number;

    entretiensDueThisYear: number;

    entretiensPlannedThisYear: number;

    entretiensToPosition: number;

  };

  onboardings: DashboardPersonnelItem[];

  signatures: DashboardPersonnelItem[];

  habilitations: DashboardPersonnelItem[];

  formations: DashboardPersonnelItem[];

  medecine: DashboardPersonnelItem[];

  entretiens: DashboardPersonnelItem[];

  medecineYear: DashboardPersonnelItem[];

  entretiensYear: DashboardPersonnelItem[];

  absencesToday: DashboardAbsenceToday[];

  staffTotal: number;

  year: number;

};



function isAbsenceToday(record: AbsenceRecord) {
  if (record.data.scope !== "ogec") return false;
  const today = new Date();
  const dayStart = new Date(today.getFullYear(), today.getMonth(), today.getDate(), 0, 0, 0, 0);
  const dayEnd = new Date(today.getFullYear(), today.getMonth(), today.getDate(), 23, 59, 59, 999);
  const start = new Date(record.data.startAt);
  const end = new Date(record.data.endAt);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return false;
  return start <= dayEnd && end >= dayStart;
}



function formatPeriod(start: string, end: string) {

  const s = new Date(start);

  const e = new Date(end);

  if (Number.isNaN(s.getTime())) return "—";

  const opts: Intl.DateTimeFormatOptions = { day: "numeric", month: "short" };

  const a = s.toLocaleDateString("fr-FR", opts);

  const b = e.toLocaleDateString("fr-FR", opts);

  return a === b ? a : `${a} → ${b}`;

}



const ACTIVE_ONBOARDING: OnboardingStatus[] = ["brouillon", "en_cours", "signatures"];



export async function buildPersonnelDashboard(records: PersonnelRecord[]): Promise<PersonnelDashboardData> {

  const year = new Date().getFullYear();

  const onboardings: DashboardPersonnelItem[] = [];

  const signatures: DashboardPersonnelItem[] = [];

  const habilitations: DashboardPersonnelItem[] = [];

  const formations: DashboardPersonnelItem[] = [];

  const medecine: DashboardPersonnelItem[] = [];

  const entretiens: DashboardPersonnelItem[] = [];

  const medecineYear: DashboardPersonnelItem[] = [];

  const entretiensYear: DashboardPersonnelItem[] = [];



  let medecineDueThisYear = 0;

  let medecineOverdue = 0;

  let entretiensDueThisYear = 0;

  let entretiensPlannedThisYear = 0;



  for (const r of records) {

    const link = `/rh/${r.id}`;



    if (r.onboarding && ACTIVE_ONBOARDING.includes(r.onboarding.status)) {

      const done = r.onboarding.checklist.filter((c) => c.done).length;

      const total = r.onboarding.checklist.length;

      onboardings.push({

        id: `onb-${r.id}`,

        displayName: r.displayName,

        category: r.category,

        detail: `Entrée ${formatPeriod(r.onboarding.startDate, r.onboarding.startDate)} · ${done}/${total} étapes`,

        urgency: r.onboarding.status === "signatures" ? "high" : "medium",

        dueDate: r.onboarding.startDate,

        link,

      });



      for (const sig of r.onboarding.signatures.filter((s) => s.status === "en_attente")) {

        signatures.push({

          id: `sig-${r.id}-${sig.id}`,

          displayName: r.displayName,

          category: sig.label,

          detail: `Signature ${sig.label} en attente`,

          urgency: "high",

          link,

        });

      }

    }



    for (const h of r.habilitations) {

      if (isOverdue(h.expiresAt) || isExpiringWithinDays(h.expiresAt, 90)) {

        const d = daysUntil(h.expiresAt);

        habilitations.push({

          id: `hab-${r.id}-${h.id}`,

          displayName: r.displayName,

          category: h.label,

          detail:

            d !== null && d < 0

              ? `${h.label} · expirée — renouvellement urgent`

              : `${h.label} · renouvellement dans ${d} j`,

          urgency: d !== null && d <= 30 ? "high" : "medium",

          dueDate: h.expiresAt,

          link,

        });

      }

    }



    for (const f of r.formations) {

      if (f.status !== "planifiee" && f.status !== "demandee") continue;

      const due = f.plannedDate || f.reminderAt;

      if (f.status === "demandee") {

        formations.push({

          id: `form-${r.id}-${f.id}`,

          displayName: r.displayName,

          category: f.title,

          detail: due

            ? `${f.title} · demandée · souhait ${new Date(due).toLocaleDateString("fr-FR")}`

            : `${f.title} · demande en attente de planification`,

          urgency: "medium",

          dueDate: due,

          link,

        });

        continue;

      }

      const d = daysUntil(due);

      formations.push({

        id: `form-${r.id}-${f.id}`,

        displayName: r.displayName,

        category: f.title,

        detail: due

          ? d !== null && d < 0

            ? `${f.title} · prévue le ${new Date(due).toLocaleDateString("fr-FR")} · en retard`

            : `${f.title} · prévue le ${new Date(due).toLocaleDateString("fr-FR")}`

          : `${f.title} · planifiée — date à confirmer`,

        urgency: d !== null && d <= 14 ? "high" : d !== null && d <= 60 ? "medium" : "low",

        dueDate: due,

        link,

      });

    }



    const nextMed = r.medecineTravail.nextVisitAt;

    if (isMedecineDueThisYear(r, year)) {

      medecineDueThisYear += 1;

      const d = daysUntil(nextMed);

      if (nextMed && isOverdue(nextMed)) medecineOverdue += 1;

      const yearItem: DashboardPersonnelItem = {

        id: `med-y-${r.id}`,

        displayName: r.displayName,

        category: r.medecineTravail.visitType || "Visite médicale",

        detail: nextMed

          ? d !== null && d < 0

            ? `Échéance ${new Date(nextMed).toLocaleDateString("fr-FR")} · retard ${Math.abs(d)} j`

            : `Échéance ${new Date(nextMed).toLocaleDateString("fr-FR")}`

          : "Prochaine visite non renseignée",

        urgency: d !== null && d <= 14 ? "high" : d !== null && d < 0 ? "high" : "medium",

        dueDate: nextMed,

        link,

      };

      medecineYear.push(yearItem);

    }



    if (nextMed && (isOverdue(nextMed) || isExpiringWithinDays(nextMed, 45))) {

      const d = daysUntil(nextMed);

      medecine.push({

        id: `med-${r.id}`,

        displayName: r.displayName,

        category: r.medecineTravail.visitType || "Visite médicale",

        detail: d !== null && d < 0 ? `En retard de ${Math.abs(d)} j` : `Dans ${d} j`,

        urgency: d !== null && d <= 14 ? "high" : "medium",

        dueDate: nextMed,

        link,

      });

    } else if (!nextMed && r.active) {

      medecine.push({

        id: `med-missing-${r.id}`,

        displayName: r.displayName,

        category: "Médecine du travail",

        detail: "Prochaine visite non renseignée",

        urgency: "low",

        link,

      });

    }



    if (isEntretienDueThisYear(r, year)) {

      entretiensDueThisYear += 1;

      const nextDue = computeEntretienNextDue(r);

      const d = daysUntil(nextDue);

      entretiensYear.push({

        id: `ent-y-${r.id}`,

        displayName: r.displayName,

        category: "Entretien professionnel",

        detail: nextDue

          ? `À positionner · échéance cycle ${new Date(nextDue).toLocaleDateString("fr-FR")}`

          : "Jamais réalisé — à planifier cette année",

        urgency: d !== null && d <= 30 ? "high" : "medium",

        dueDate: nextDue,

        link,

      });

    }



    if (hasOpenEntretienForCycle(r, year)) {

      entretiensPlannedThisYear += 1;

    }



    for (const e of r.entretiens.filter((x) => x.status !== "realise")) {

      const due = e.scheduledAt || e.reminderAt;

      entretiens.push({

        id: `ent-${r.id}-${e.id}`,

        displayName: r.displayName,

        category: "Entretien professionnel",

        detail:

          e.status === "a_planifier"

            ? "À planifier"

            : due

              ? `Prévu le ${new Date(due).toLocaleDateString("fr-FR")}`

              : "Planifié",

        urgency: due && isOverdue(due) ? "high" : "medium",

        dueDate: due,

        link,

      });

    }

  }



  let absenceIndex: AbsenceRecord[] = [];
  try {
    absenceIndex = await getAbsenceIndex();
  } catch (err) {
    console.error("[personnel-dashboard] lecture absences", err);
  }

  const absencesToday: DashboardAbsenceToday[] = absenceIndex

    .filter(isAbsenceToday)

    .map((a) => ({

      id: a.id,

      displayName: a.displayName || a.createdBy.name,

      reason: a.data.reason || "Absence",

      periodLabel: formatPeriod(a.data.startAt, a.data.endAt),

    }));



  const sortUrgency = (items: DashboardPersonnelItem[]) =>

    items.sort((a, b) => {

      const order = { high: 0, medium: 1, low: 2 };

      const ua = order[a.urgency] - order[b.urgency];

      if (ua !== 0) return ua;

      if (a.dueDate && b.dueDate) return new Date(a.dueDate).getTime() - new Date(b.dueDate).getTime();

      return 0;

    });



  const entretiensToPosition = Math.max(0, entretiensDueThisYear - entretiensPlannedThisYear);



  return {

    year,

    counts: {

      onboardings: onboardings.length,

      signatures: signatures.length,

      habilitations: habilitations.length,

      formations: formations.length,

      medecine: medecine.length,

      entretiens: entretiens.length,

      absencesToday: absencesToday.length,

      medecineDueThisYear,

      medecineOverdue,

      entretiensDueThisYear,

      entretiensPlannedThisYear,

      entretiensToPosition,

    },

    onboardings: sortUrgency(onboardings),

    signatures: sortUrgency(signatures),

    habilitations: sortUrgency(habilitations),

    formations: sortUrgency(formations),

    medecine: sortUrgency(medecine),

    entretiens: sortUrgency(entretiens),

    medecineYear: sortUrgency(medecineYear),

    entretiensYear: sortUrgency(entretiensYear),

    absencesToday,

    staffTotal: records.filter((r) => r.active).length,

  };

}


