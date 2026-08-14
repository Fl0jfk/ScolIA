import { randomBytes } from "crypto";
import type { Establishment } from "@/app/lib/app-config-schemas";
import type {
  InternatEtablissement,
  InternatOuting,
  InternatOutingDirectionDecision,
  InternatOutingIndexEntry,
  InternatOutingParticipant,
  InternatOutingStatus,
  InternatStudent,
} from "@/app/lib/internat-types";
import { studentDisplayName } from "@/app/lib/internat-types";
import { matchEstablishment } from "@/app/lib/establishment-catalog";

function generateOutingToken(): string {
  return randomBytes(32).toString("base64url");
}

export function normalizeParentContact(raw: unknown): { nom?: string; email?: string; telephone?: string } | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const o = raw as Record<string, unknown>;
  const email = String(o.email || "").trim();
  const nom = String(o.nom || "").trim();
  const telephone = String(o.telephone || "").trim();
  if (!email && !nom && !telephone) return undefined;
  return {
    ...(nom ? { nom } : {}),
    ...(email ? { email } : {}),
    ...(telephone ? { telephone } : {}),
  };
}

function directorEmailForEtab(
  etablissement: InternatEtablissement,
  establishments: Establishment[],
): string | undefined {
  return matchEstablishment(establishments, etablissement)?.directorEmail?.trim() || undefined;
}

export function outingIndexEntry(outing: InternatOuting): InternatOutingIndexEntry {
  return {
    id: outing.id,
    title: outing.title,
    outingDate: outing.outingDate,
    status: outing.status,
    participantCount: outing.participants.length,
    createdAt: outing.createdAt,
  };
}

export function computeOutingStatus(outing: InternatOuting): InternatOutingStatus {
  if (outing.status === "cancelled") return "cancelled";

  const anyDirectionRefused = outing.directionDecisions.some((d) => d.status === "refused");
  if (anyDirectionRefused) return "refused";

  const allDirectionsApproved =
    outing.directionDecisions.length > 0 &&
    outing.directionDecisions.every((d) => d.status === "approved");
  if (!allDirectionsApproved) return "pending_direction";

  const allParentsOk = outing.participants.every((p) => p.parentStatus === "authorized");
  const anyParentRefused = outing.participants.some((p) => p.parentStatus === "refused");
  if (anyParentRefused) return "refused";
  if (allParentsOk) return "authorized";
  return "pending_parents";
}

export function buildOutingParticipants(
  studentIds: string[],
  students: InternatStudent[],
): { participants: InternatOutingParticipant[]; missingParents: string[] } {
  const participants: InternatOutingParticipant[] = [];
  const missingParents: string[] = [];

  for (const studentId of studentIds) {
    const s = students.find((x) => x.id === studentId && x.actif);
    if (!s) continue;
    const parent1Email = s.parent1?.email?.trim();
    const parent2Email = s.parent2?.email?.trim();
    if (!parent1Email && !parent2Email) {
      missingParents.push(studentDisplayName(s));
    }
    participants.push({
      studentId: s.id,
      studentName: studentDisplayName(s),
      etablissement: s.etablissement,
      classe: s.classe,
      parentToken: generateOutingToken(),
      parent1Email: parent1Email || undefined,
      parent2Email: parent2Email || undefined,
      parentStatus: "pending",
    });
  }

  return { participants, missingParents };
}

export function buildDirectionDecisions(
  participants: InternatOutingParticipant[],
  establishments: Establishment[],
): InternatOutingDirectionDecision[] {
  const etabs = new Set(participants.map((p) => p.etablissement));
  const decisions: InternatOutingDirectionDecision[] = [];
  for (const etablissement of etabs) {
    decisions.push({
      etablissement,
      token: generateOutingToken(),
      status: "pending",
      directorEmail: directorEmailForEtab(etablissement, establishments),
    });
  }
  return decisions;
}

export function participantsForEtab(outing: InternatOuting, etablissement: InternatEtablissement) {
  return outing.participants.filter((p) => p.etablissement === etablissement);
}

export function outingDateTimeLabel(outing: InternatOuting): string {
  const date = new Date(outing.outingDate).toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const times = [outing.departureTime, outing.returnTime].filter(Boolean);
  if (!times.length) return date;
  if (outing.departureTime && outing.returnTime) {
    return `${date} — départ ${outing.departureTime}, retour ${outing.returnTime}`;
  }
  return `${date} — ${times.join(", ")}`;
}

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

function toDateStr(d: Date) {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

/** Lundi → dimanche de la semaine calendaire suivante. */
export function nextWeekDateRange(from = new Date()) {
  const d = new Date(from);
  const dow = d.getDay();
  const daysToNextMonday = dow === 0 ? 1 : 8 - dow;
  const monday = new Date(d);
  monday.setDate(d.getDate() + daysToNextMonday);
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  return { start: toDateStr(monday), end: toDateStr(sunday) };
}

export function isDateInRange(date: string, start: string, end: string) {
  return date >= start && date <= end;
}

export function outingIsVisibleToParents(outing: InternatOuting) {
  return (
    outing.status !== "cancelled" &&
    outing.status !== "refused" &&
    (outing.status === "authorized" || outing.status === "pending_parents")
  );
}

export const OUTING_STATUS_LABELS: Record<InternatOutingStatus, string> = {
  pending_direction: "En attente direction",
  pending_parents: "En attente parents",
  authorized: "Autorisée",
  refused: "Refusée",
  cancelled: "Annulée",
};
