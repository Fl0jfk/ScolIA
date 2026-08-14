export type AbsencePeriodType = "single_day" | "multi_day";

type AbsencePeriodFields = {
  periodType?: AbsencePeriodType | null;
  startDate: string;
  endDate: string;
  startTime?: string | null;
  endTime?: string | null;
};

function formatDateFR(input?: string | null) {
  if (!input) return "—";
  const d = new Date(`${input}T12:00:00`);
  if (Number.isNaN(d.getTime())) return String(input);
  return d.toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

function formatTimeFR(hhmm: string) {
  const [hRaw, mRaw] = hhmm.split(":");
  const h = hRaw?.padStart(2, "0") ?? "00";
  const m = mRaw?.padStart(2, "0") ?? "00";
  return m === "00" ? `${h}h` : `${h}h${m}`;
}

function getAbsencePeriodType(data: AbsencePeriodFields): AbsencePeriodType {
  if (data.periodType === "single_day") return "single_day";
  if (data.periodType === "multi_day") return "multi_day";
  if (data.startTime && data.endTime) return "single_day";
  return "multi_day";
}

export function formatAbsencePeriod(data: AbsencePeriodFields): string {
  const type = getAbsencePeriodType(data);
  if (type === "single_day" && data.startTime && data.endTime) {
    return `${formatDateFR(data.startDate)} · ${formatTimeFR(data.startTime)} – ${formatTimeFR(data.endTime)}`;
  }
  if (data.startDate === data.endDate) {
    return formatDateFR(data.startDate);
  }
  return `Du ${formatDateFR(data.startDate)} au ${formatDateFR(data.endDate)}`;
}

function parseTimeToMinutes(value: string) {
  const match = /^(\d{1,2}):(\d{2})$/.exec(value.trim());
  if (!match) return null;
  const h = Number(match[1]);
  const m = Number(match[2]);
  if (h < 0 || h > 23 || m < 0 || m > 59) return null;
  return h * 60 + m;
}

export function normalizeAbsencePeriodInput(input: {
  periodType?: string | null;
  startDate?: string | null;
  endDate?: string | null;
  startTime?: string | null;
  endTime?: string | null;
}): { error?: string; data?: AbsencePeriodFields } {
  const periodType: AbsencePeriodType =
    input.periodType === "single_day" ? "single_day" : "multi_day";
  const reason = String(input.startDate || "").trim();

  if (periodType === "single_day") {
    const day = reason;
    const startTime = String(input.startTime || "").trim();
    const endTime = String(input.endTime || "").trim();
    if (!day || !startTime || !endTime) {
      return { error: "Pour une journée, indiquez la date et les heures de début et de fin." };
    }
    const startMin = parseTimeToMinutes(startTime);
    const endMin = parseTimeToMinutes(endTime);
    if (startMin === null || endMin === null) {
      return { error: "Format horaire invalide." };
    }
    if (endMin <= startMin) {
      return { error: "L'heure de fin doit être après l'heure de début." };
    }
    return {
      data: {
        periodType: "single_day",
        startDate: day,
        endDate: day,
        startTime,
        endTime,
      },
    };
  }

  const startDate = String(input.startDate || "").trim();
  const endDate = String(input.endDate || "").trim();
  if (!startDate || !endDate) {
    return { error: "Indiquez la date de début et la date de fin." };
  }
  const start = new Date(`${startDate}T12:00:00`);
  const end = new Date(`${endDate}T12:00:00`);
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
    return { error: "Dates invalides." };
  }
  if (end < start) {
    return { error: "La date de fin doit être après la date de début." };
  }
  return {
    data: {
      periodType: "multi_day",
      startDate,
      endDate,
      startTime: null,
      endTime: null,
    },
  };
}
