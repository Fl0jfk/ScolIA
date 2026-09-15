import { loadAppConfig } from "@/app/lib/app-config";
import { notifyInternatRollCallIncomplete } from "@/app/lib/internat-notify";
import {
  isInternatEveningRollCallDay,
  rollCallPendingActivityStudents,
  sectionIsComplete,
  todayDateParis,
} from "@/app/lib/internat-stats";
import { studentDisplayName } from "@/app/lib/internat-types";
import { getInternatRollCall, getInternatStudents, saveInternatRollCall } from "@/app/lib/internat-storage";

function parisHour() {
  return Number(
    new Intl.DateTimeFormat("fr-FR", {
      timeZone: "Europe/Paris",
      hour: "numeric",
      hour12: false,
    }).format(new Date()),
  );
}

/**
 * Un seul rappel / soir (heure configurée, défaut 21h).
 * - Appel incomplet → rappel classique
 * - Sections OK mais élèves encore en « Activité » → rappel doux (pas d’alarme)
 * Jamais de spam horaire.
 */
export async function runInternatRollCallReminder(options?: { force?: boolean }) {
  const bundle = await loadAppConfig();
  const config = bundle.internat;
  if (!options?.force && config.rollCallReminderEnabled === false) {
    return { sent: false, reason: "disabled" };
  }

  const date = todayDateParis();
  // Toujours hors lun–jeu : pas de rappel (cron inclus) — internes absents.
  if (!isInternatEveningRollCallDay(date)) {
    return { sent: false, reason: "not_evening_roll_call_day", date };
  }

  const targetHour = config.rollCallReminderHour ?? 21;
  const hour = parisHour();
  if (!options?.force && hour !== targetHour) {
    return { sent: false, reason: "wrong_hour", hour, targetHour };
  }

  const [rollCall, students] = await Promise.all([getInternatRollCall(date), getInternatStudents()]);

  if (rollCall.status === "validee") {
    return { sent: false, reason: "already_validated" };
  }
  if (!options?.force && rollCall.reminderSentAt) {
    return { sent: false, reason: "already_reminded" };
  }

  const active = students.filter((s) => s.actif);
  const allMarks = { ...rollCall.boys.marks, ...rollCall.girls.marks };
  const markedCount = active.filter((s) => allMarks[s.id]).length;
  const pending = rollCallPendingActivityStudents(rollCall, students);
  const sectionsDone =
    sectionIsComplete(rollCall.boys, students, "M") &&
    sectionIsComplete(rollCall.girls, students, "F");

  // Rien à rappeler si l’appel n’a même pas démarré et qu’on n’est pas en attente activité.
  if (markedCount === 0 && pending.length === 0) {
    return { sent: false, reason: "not_started", date };
  }

  const mail = await notifyInternatRollCallIncomplete({
    rollCall,
    students,
    markedCount,
    totalCount: active.length,
    pendingActivityNames:
      pending.length > 0 ? pending.map((s) => studentDisplayName(s)) : undefined,
  });

  if (mail.sent) {
    const now = new Date().toISOString();
    await saveInternatRollCall({ ...rollCall, reminderSentAt: now, updatedAt: now });
  }

  return {
    ...mail,
    date,
    markedCount,
    totalCount: active.length,
    pendingActivityCount: pending.length,
    sectionsDone,
  };
}
