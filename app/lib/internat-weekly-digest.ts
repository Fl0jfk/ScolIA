import { loadAppConfig, saveInternatModule } from "@/app/lib/app-config";
import {
  isDateInRange,
  nextWeekDateRange,
  outingIsVisibleToParents,
  OUTING_STATUS_LABELS,
} from "@/app/lib/internat-outing";
import { sendInternatWeeklyParentDigest, type ParentWeeklyDigestLine } from "@/app/lib/internat-notify";
import { listInternatOutings } from "@/app/lib/internat-storage";
import type { InternatOuting } from "@/app/lib/internat-types";
import { todayDateParis } from "@/app/lib/internat-stats";

function parentStatusLabel(outing: InternatOuting, studentId: string) {
  const p = outing.participants.find((x) => x.studentId === studentId);
  if (!p) return "—";
  if (outing.status === "authorized" && p.parentStatus === "authorized") return "Autorisée";
  if (p.parentStatus === "refused") return "Refusée";
  if (p.parentStatus === "pending") return "En attente de votre accord";
  return OUTING_STATUS_LABELS[outing.status];
}

function buildWeeklyParentDigestLines(outings: InternatOuting[], range = nextWeekDateRange()) {
  const linesByEmail = new Map<string, ParentWeeklyDigestLine[]>();

  for (const outing of outings) {
    if (!outingIsVisibleToParents(outing)) continue;
    if (!isDateInRange(outing.outingDate, range.start, range.end)) continue;

    for (const p of outing.participants) {
      const line: ParentWeeklyDigestLine = {
        studentName: p.studentName,
        date: outing.outingDate,
        title: outing.title,
        activity: outing.activity,
        destination: outing.destination,
        timeLabel:
          outing.departureTime || outing.returnTime
            ? [outing.departureTime, outing.returnTime].filter(Boolean).join(" → ")
            : "—",
        statusLabel: parentStatusLabel(outing, p.studentId),
      };
      for (const email of [p.parent1Email, p.parent2Email].filter(Boolean) as string[]) {
        const key = email.toLowerCase();
        const bucket = linesByEmail.get(key) || [];
        bucket.push(line);
        linesByEmail.set(key, bucket);
      }
    }
  }

  return { linesByEmail, range };
}

export async function runInternatWeeklyParentDigest(options?: { force?: boolean }) {
  const bundle = await loadAppConfig();
  const config = bundle.internat;
  if (!options?.force && config.weeklyParentDigestEnabled === false) {
    return { sent: false, reason: "disabled" };
  }

  const today = todayDateParis();
  const weekday = new Date().getDay();
  if (!options?.force && weekday !== (config.weeklyParentDigestWeekday ?? 0)) {
    return { sent: false, reason: "wrong_day", weekday };
  }
  if (!options?.force && config.weeklyParentDigestLastSent === today) {
    return { sent: false, reason: "already_sent" };
  }

  const outings = await listInternatOutings(200);
  const { linesByEmail, range } = buildWeeklyParentDigestLines(outings);
  const weekLabel = new Date(range.start).toLocaleDateString("fr-FR", {
    day: "numeric",
    month: "long",
  });

  const mail = await sendInternatWeeklyParentDigest({ linesByEmail, weekLabel });
  if (mail.sent) {
    await saveInternatModule({
      ...config,
      weeklyParentDigestLastSent: today,
    });
  }

  return { ...mail, weekLabel, range, parentCount: linesByEmail.size };
}
