import "server-only";

import { createHash } from "node:crypto";
import { normalizeParentCalendarTime } from "@/app/lib/travels-parent-calendar";
import type { TravelsParentCalendar, TravelsTripData } from "@/app/lib/travels-types";

/** Empreinte stable du contenu calendrier (.ics) pour dédoublonner les envois parents. */
export function computeParentCalendarNotifyHash(
  data: Pick<TravelsTripData, "startDate" | "endDate" | "date" | "startTime" | "endTime">,
  calendar: TravelsParentCalendar,
): string {
  const startDate = String(data.startDate || data.date || "").slice(0, 10);
  const endDate = String(data.endDate || startDate).slice(0, 10);
  const startTime = normalizeParentCalendarTime(data.startTime) || "";
  const endTime = normalizeParentCalendarTime(data.endTime) || "";
  const points = (calendar.points || [])
    .map((p) => ({
      kind: p.kind,
      date: String(p.date || "").slice(0, 10),
      time: normalizeParentCalendarTime(p.time) || "",
      place: String(p.place || "").trim(),
    }))
    .sort((a, b) =>
      `${a.kind}:${a.date}:${a.time}:${a.place}`.localeCompare(
        `${b.kind}:${b.date}:${b.time}:${b.place}`,
      ),
    );
  const payload = JSON.stringify({
    includeTripSpan: calendar.includeTripSpan !== false,
    span: { startDate, endDate, startTime, endTime },
    points,
  });
  return createHash("sha256").update(payload).digest("hex");
}
