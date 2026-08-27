import type { PdfTextItem } from "@/app/lib/rh/planning-pdf-text";
import type {
  PlanningWeekday,
  TeacherPlanningSlot,
} from "@/app/lib/rh/planning-types";

export type PronoteTeacherParseResult = {
  personHint: string;
  weekA: Array<Omit<TeacherPlanningSlot, "id">>;
  weekB: Array<Omit<TeacherPlanningSlot, "id">>;
  warnings: string[];
  slotCount: number;
};

type DayDef = { day: PlanningWeekday; label: string; x: number };

type TimedItem = PdfTextItem & {
  kind: "time" | "text";
  start?: string;
  endHint?: string;
  week?: "A" | "B" | null;
};

const DAY_LABELS: Array<{ day: PlanningWeekday; re: RegExp }> = [
  { day: 1, re: /^lundi$/i },
  { day: 2, re: /^mardi$/i },
  { day: 3, re: /^mercredi$/i },
  { day: 4, re: /^jeudi$/i },
  { day: 5, re: /^vendredi$/i },
];

const TIME_RE = /^(\d{1,2})[:hH.](\d{2})(?:\s*\(([ABab])\))?$/;
const CLASS_CODE_RE = /^(CL_|LY_|EG_|ECOLE_|G_|GRP_)/i;
const ROOM_RE = /^(salle|s\s|std|sta|sp\d|cdi|eps|gym)/i;

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function parseTimeToken(raw: string): { hhmm: string; week: "A" | "B" | null } | null {
  const m = TIME_RE.exec(raw.trim());
  if (!m) return null;
  const h = Number(m[1]);
  const min = Number(m[2]);
  if (!Number.isFinite(h) || !Number.isFinite(min) || h > 23 || min > 59) return null;
  const weekRaw = m[3]?.toUpperCase();
  const week = weekRaw === "A" || weekRaw === "B" ? weekRaw : null;
  return { hhmm: `${pad2(h)}:${pad2(min)}`, week };
}

function looksLikeRoom(text: string): boolean {
  const t = text.trim();
  if (ROOM_RE.test(t)) return true;
  if (/^S\s+[A-Z0-9]/i.test(t)) return true;
  if (/^Salle\s+/i.test(t)) return true;
  return false;
}

function normalizeRoom(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

/** Codes Pronote → libellés classes plus lisibles (conservateurs). */
export function humanizePronoteClassCode(code: string): string {
  const c = code.trim();
  const cl = /^CL_(\d+)_(.+)$/i.exec(c);
  if (cl) return `${cl[1]} (${cl[2]})`;
  const ly = /^LY_(\d*)(.*)$/i.exec(c);
  if (ly) {
    const niveau = ly[1] || "";
    const rest = (ly[2] || "").replace(/^ALL/i, "ALL");
    if (niveau === "0" || niveau === "") return `Lycée ${rest || c}`.trim();
    if (niveau === "1") return `1re ${rest}`.trim();
    if (niveau === "2") return `Tle ${rest}`.trim();
    return `LY${niveau} ${rest}`.trim();
  }
  return c;
}

function detectDayColumns(items: PdfTextItem[]): DayDef[] | null {
  const found: DayDef[] = [];
  for (const it of items) {
    for (const d of DAY_LABELS) {
      if (!d.re.test(it.text.trim())) continue;
      if (found.some((f) => f.day === d.day)) continue;
      found.push({ day: d.day, label: it.text.trim(), x: it.x });
    }
  }
  if (found.length < 3) return null;
  found.sort((a, b) => a.x - b.x);
  return found;
}

function assignDay(x: number, days: DayDef[]): PlanningWeekday {
  let best = days[0]!;
  let bestDist = Infinity;
  for (const d of days) {
    const dist = Math.abs(x - d.x);
    if (dist < bestDist) {
      bestDist = dist;
      best = d;
    }
  }
  return best.day;
}

function isNoise(text: string): boolean {
  if (/^(lundi|mardi|mercredi|jeudi|vendredi)$/i.test(text)) return true;
  if (/providence|semaint|semaine\s+type|rentr[eé]e|\d{2}\/\d{2}\/\d{4}/i.test(text)) return true;
  if (/^\d{1,2}h\d{2}$/i.test(text) && text.length <= 6) return false;
  return false;
}

function extractPersonHint(items: PdfTextItem[]): string {
  const ranked = items
    .slice()
    .sort((a, b) => b.y - a.y || a.x - b.x)
    .slice(0, 12);
  for (const it of ranked) {
    const text = it.text.trim();
    // « NOM Prénom (Semaine type…) » — format Pronote fréquent
    const withParen =
      /^([A-ZÀÂÄÉÈÊËÏÎÔÖÙÛÜŸÇ][A-ZÀÂÄÉÈÊËÏÎÔÖÙÛÜŸÇ'’-]{1,40})\s+([A-ZÀÂÄÉÈÊËÏÎÔÖÙÛÜŸÇa-zàâäéèêëïîôöùûüÿç'’-]+)\s*\(/u.exec(
        text,
      );
    if (withParen) {
      return `${withParen[1].trim()} ${withParen[2].trim()}`.replace(/\s+/g, " ");
    }
  }
  for (const it of ranked) {
    const text = it.text.trim();
    const plain =
      /^([A-ZÀÂÄÉÈÊËÏÎÔÖÙÛÜŸÇ][A-ZÀÂÄÉÈÊËÏÎÔÖÙÛÜŸÇ'’-]{2,40})\s+([A-ZÀÂÄÉÈÊËÏÎÔÖÙÛÜŸÇ][a-zàâäéèêëïîôöùûüÿç'’-]+)$/u.exec(
        text,
      );
    if (plain) {
      return `${plain[1].trim()} ${plain[2].trim()}`.replace(/\s+/g, " ");
    }
  }
  return "";
}

type RawSlot = {
  day: PlanningWeekday;
  start: string;
  end: string;
  week: "A" | "B" | null;
  subject: string;
  room?: string;
  classes: string[];
  yTop: number;
  x: number;
};

/**
 * Parse une grille EDT professeur Pronote (PDF texte + coordonnées).
 * Retourne null si le document ne ressemble pas à une grille Pronote exploitable.
 */
export function parsePronoteTeacherGrid(items: PdfTextItem[]): PronoteTeacherParseResult | null {
  if (items.length < 20) return null;
  const days = detectDayColumns(items);
  if (!days) return null;

  const warnings: string[] = [];
  const personHint = extractPersonHint(items);

  const byDay = new Map<PlanningWeekday, TimedItem[]>();
  for (const d of days) byDay.set(d.day, []);

  for (const it of items) {
    if (isNoise(it.text)) continue;
    const day = assignDay(it.x, days);
    const time = parseTimeToken(it.text);
    const list = byDay.get(day);
    if (!list) continue;
    if (time) {
      list.push({ ...it, kind: "time", start: time.hhmm, week: time.week });
    } else {
      list.push({ ...it, kind: "text" });
    }
  }

  const rawSlots: RawSlot[] = [];

  for (const day of days.map((d) => d.day)) {
    const col = (byDay.get(day) || []).slice().sort((a, b) => b.y - a.y || a.x - b.x);
    rawSlots.push(...parseDayColumn(day, col));
  }

  if (rawSlots.length < 3) {
    return null;
  }

  const weekA: PronoteTeacherParseResult["weekA"] = [];
  const weekB: PronoteTeacherParseResult["weekB"] = [];

  for (const slot of rawSlots) {
    const normalized = {
      day: slot.day,
      start: slot.start,
      end: slot.end,
      subject: slot.subject || "Cours",
      classes: slot.classes.length
        ? slot.classes.map(humanizePronoteClassCode)
        : [],
      room: slot.room,
    };
    if (slot.week === "A") {
      weekA.push(normalized);
    } else if (slot.week === "B") {
      weekB.push(normalized);
    } else {
      weekA.push(normalized);
      weekB.push(normalized);
    }
  }

  const onlyA = rawSlots.filter((s) => s.week === "A").length;
  const onlyB = rawSlots.filter((s) => s.week === "B").length;
  const both = rawSlots.filter((s) => s.week == null).length;
  if (onlyA || onlyB) {
    warnings.push(
      `Créneaux A/B Pronote détectés (${onlyA} en A, ${onlyB} en B, ${both} communs aux deux semaines).`,
    );
  } else {
    warnings.push("Aucune distinction A/B : créneaux reportés sur les semaines A et B.");
  }
  warnings.push("Import grille Pronote (texte + positions) — vérifiez avant validation.");

  return {
    personHint,
    weekA,
    weekB,
    warnings,
    slotCount: rawSlots.length,
  };
}

function parseDayColumn(day: PlanningWeekday, col: TimedItem[]): RawSlot[] {
  if (col.length === 0) return [];

  const timeItems = col.filter((c) => c.kind === "time");
  if (timeItems.length < 2) return parseLinearDay(day, col);

  const timeClusters = clusterItemsByX(timeItems, 42);
  if (timeClusters.length <= 1) {
    return parseLinearDay(day, col);
  }

  const meta = timeClusters.map((c) => {
    const xs = c.map((i) => i.x);
    const ys = c.map((i) => i.y);
    return {
      items: c.slice(),
      center: xs.reduce((s, v) => s + v, 0) / xs.length,
      yMax: Math.max(...ys),
      yMin: Math.min(...ys),
    };
  });

  const buckets: TimedItem[][] = meta.map((m) => m.items.slice());
  for (const it of col) {
    if (it.kind === "time") continue;
    const covering = meta
      .map((m, idx) => ({ m, idx }))
      .filter(({ m }) => it.y <= m.yMax + 8 && it.y >= m.yMin - 8);
    const candidates = covering.length > 0 ? covering : meta.map((m, idx) => ({ m, idx }));
    let best = candidates[0]!.idx;
    let bestDist = Infinity;
    for (const c of candidates) {
      const dist = Math.abs(it.x - c.m.center);
      if (dist < bestDist) {
        bestDist = dist;
        best = c.idx;
      }
    }
    buckets[best]!.push(it);
  }

  const out: RawSlot[] = [];
  for (const bucket of buckets) {
    if (bucket.filter((b) => b.kind === "time").length >= 2) {
      out.push(...parseLinearDay(day, bucket));
    }
  }
  return out.length > 0 ? out : parseLinearDay(day, col);
}

function clusterItemsByX(items: TimedItem[], maxGap: number): TimedItem[][] {
  if (items.length === 0) return [];
  const sorted = items.slice().sort((a, b) => a.x - b.x);
  const clusters: TimedItem[][] = [[sorted[0]!]];
  for (let i = 1; i < sorted.length; i++) {
    const it = sorted[i]!;
    const cur = clusters[clusters.length - 1]!;
    const last = cur[cur.length - 1]!;
    if (it.x - last.x <= maxGap) {
      cur.push(it);
    } else {
      clusters.push([it]);
    }
  }
  return clusters;
}

function parseLinearDay(day: PlanningWeekday, col: TimedItem[]): RawSlot[] {
  const sorted = col.slice().sort((a, b) => b.y - a.y || a.x - b.x);
  const slots: RawSlot[] = [];
  let i = 0;
  while (i < sorted.length) {
    const startItem = sorted[i];
    if (!startItem || startItem.kind !== "time" || !startItem.start) {
      i += 1;
      continue;
    }
    const start = startItem.start;
    const week = startItem.week;
    const body: TimedItem[] = [];
    i += 1;
    while (i < sorted.length && sorted[i]!.kind !== "time") {
      body.push(sorted[i]!);
      i += 1;
    }
    let end = "";
    let endWeek = week;
    if (i < sorted.length && sorted[i]!.kind === "time" && sorted[i]!.start) {
      end = sorted[i]!.start!;
      endWeek = sorted[i]!.week ?? week;
      i += 1;
      // Pronote duplique souvent l'heure de fin comme début du créneau suivant.
      // On laisse le prochain time identique pour l'itération suivante.
      if (i < sorted.length && sorted[i]!.kind === "time" && sorted[i]!.start === end) {
        /* no-op */
      }
    }
    if (!end || end === start || body.length === 0) {
      continue;
    }
    if (start >= end) continue;

    let subject = "";
    let room: string | undefined;
    const classes: string[] = [];
    for (const b of body) {
      const t = b.text.trim();
      if (!t) continue;
      if (CLASS_CODE_RE.test(t)) {
        classes.push(t);
        continue;
      }
      if (looksLikeRoom(t)) {
        room = normalizeRoom(t);
        continue;
      }
      if (!subject) subject = t;
      else if (!room && t.length <= 24) room = normalizeRoom(t);
      else subject = `${subject} ${t}`.trim();
    }

    slots.push({
      day,
      start,
      end,
      week: week || endWeek || null,
      subject: subject || "Cours",
      room,
      classes,
      yTop: startItem.y,
      x: startItem.x,
    });
  }
  return slots;
}

/** Score rapide : le PDF a-t-il une tête de grille Pronote ? */
export function looksLikePronoteTeacherPdf(items: PdfTextItem[]): boolean {
  const days = detectDayColumns(items);
  if (!days || days.length < 4) return false;
  const times = items.filter((i) => parseTimeToken(i.text)).length;
  const codes = items.filter((i) => CLASS_CODE_RE.test(i.text)).length;
  return times >= 6 && (codes >= 2 || items.some((i) => /allemand|math|fran[cç]ais|histoire|eps/i.test(i.text)));
}
