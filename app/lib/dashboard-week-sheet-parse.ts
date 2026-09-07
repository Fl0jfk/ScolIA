import { getMistralApiKey } from "@/app/lib/tenant-config";
import { pickActiveWeekSheet } from "@/app/lib/dashboard-week-sheet-active";
import {
  normalizeWeekDay,
  parseTimeToMinutes,
} from "@/app/lib/dashboard-week-sheet-time";
import type { WeekSheetData, WeekSheetEvent, WeekSheetWeek } from "@/app/lib/dashboard-week-sheet-types";
import {
  enrichWeekSheetWeek,
  inferWeekStartFromLabel,
  splitOcrByWeekSections,
} from "@/app/lib/dashboard-week-sheet-week";

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";

type MistralEvent = {
  day?: string;
  startTime?: string;
  endTime?: string;
  title?: string;
  location?: string;
  notes?: string;
};

type MistralWeek = {
  weekLabel?: string;
  weekStart?: string;
  events?: MistralEvent[];
};

type MistralPayload = {
  weekLabel?: string;
  weekStart?: string;
  events?: MistralEvent[];
  weeks?: MistralWeek[];
};

function normalizeEvent(raw: MistralEvent, index: number): WeekSheetEvent | null {
  const day = raw.day ? normalizeWeekDay(raw.day) : null;
  const title = raw.title?.trim();
  if (!day || !title) return null;
  // Annonces sans horaire (consignes journée) : 8h00 pour rester visibles le jour J.
  let startTime = raw.startTime?.trim() || "8h00";
  if (parseTimeToMinutes(startTime) === null) startTime = "8h00";
  return {
    id: `ev-${index}`,
    day,
    startTime,
    endTime: raw.endTime?.trim() || undefined,
    title,
    location: raw.location?.trim() || undefined,
  };
}

/** Supprime les sous-descriptions (notes) et déduplique les créneaux. */
function sanitizeWeekEvents(events: WeekSheetEvent[]): WeekSheetEvent[] {
  const deduped = new Map<string, WeekSheetEvent>();
  for (const ev of events) {
    const clean: WeekSheetEvent = {
      id: ev.id,
      day: ev.day,
      startTime: ev.startTime,
      endTime: ev.endTime,
      title: ev.title,
      ...(ev.location?.trim() ? { location: ev.location.trim() } : {}),
    };
    const key = `${clean.day}|${clean.startTime}|${clean.endTime ?? ""}|${clean.title.toLowerCase()}`;
    if (!deduped.has(key)) deduped.set(key, clean);
  }
  return [...deduped.values()];
}

function normalizeWeek(raw: MistralWeek, weekIndex: number): WeekSheetWeek | null {
  const events = sanitizeWeekEvents(
    (raw.events ?? [])
      .map((ev, i) => normalizeEvent(ev, weekIndex * 1000 + i))
      .filter((ev): ev is WeekSheetEvent => ev !== null),
  );
  if (events.length === 0) return null;
  const weekLabel = raw.weekLabel?.trim() || undefined;
  const weekStart = raw.weekStart?.trim() || inferWeekStartFromLabel(weekLabel);
  return enrichWeekSheetWeek({ weekLabel, weekStart, events });
}

function buildPrompt(ocrText: string, singleWeek: boolean): string {
  const scope = singleWeek
    ? "Extrais les créneaux de CETTE semaine uniquement (le texte ne couvre qu'une semaine)."
    : "Le document peut contenir PLUSIEURS semaines : extrais CHAQUE semaine séparément.";
  const year = new Date().getFullYear();
  const exampleMonday = `${year}-09-07`;

  return (
    `Tu analyses une feuille de semaine (planning) d'un établissement scolaire en France.\n` +
    `${scope}\n` +
    `Pour chaque semaine, extrais TOUS les créneaux / rendez-vous / réunions / activités du lundi au vendredi.\n` +
    `IMPORTANT : chaque activité distincte = une entrée séparée dans events (absence d'un prof, réunion, consigne horaire, etc.).\n` +
    `Si une activité n'a pas d'horaire précis, mets startTime à "8h00".\n` +
    `Ne mets JAMAIS d'information dans "notes" : laisse notes vide ou omets le champ.\n` +
    `Année civile courante : ${year}. Si le PDF n'indique pas l'année, utilise ${year}.\n` +
    `Réponds UNIQUEMENT en JSON valide avec ce schéma :\n` +
    `{\n` +
    `  "weeks": [\n` +
    `    {\n` +
    `      "weekLabel": "Semaine du 7 au 13 septembre ${year}",\n` +
    `      "weekStart": "${exampleMonday}",\n` +
    `      "events": [\n` +
    `        {\n` +
    `          "day": "lundi|mardi|mercredi|jeudi|vendredi",\n` +
    `          "startTime": "8h30 ou 08:30",\n` +
    `          "endTime": "9h15",\n` +
    `          "title": "intitulé complet (ex. Absence de M. Dupont)",\n` +
    `          "location": "lieu physique uniquement, sinon vide"\n` +
    `        }\n` +
    `      ]\n` +
    `    }\n` +
    `  ]\n` +
    `}\n` +
    `Règles : weekStart = date du LUNDI (YYYY-MM-DD) en ${year} sauf si une autre année est écrite dans le PDF, pas de sous-texte ni notes, jours en français, heures 8h30 ou 08:30, pas de texte hors JSON.\n\n` +
    `Texte OCR :\n${ocrText.slice(0, 14_000)}`
  );
}

async function callMistral(apiKey: string, ocrText: string, singleWeek: boolean): Promise<MistralPayload> {
  const res = await fetch(MISTRAL_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      temperature: 0.1,
      response_format: { type: "json_object" },
      messages: [{ role: "user", content: buildPrompt(ocrText, singleWeek) }],
    }),
  });

  if (!res.ok) {
    const err = await res.text().catch(() => "");
    throw new Error(`Analyse IA échouée (${res.status})${err ? ` : ${err.slice(0, 200)}` : ""}`);
  }

  const data = await res.json();
  try {
    return JSON.parse(data?.choices?.[0]?.message?.content ?? "{}") as MistralPayload;
  } catch {
    throw new Error("Réponse IA illisible.");
  }
}

function payloadToWeeks(parsed: MistralPayload, indexOffset = 0): WeekSheetWeek[] {
  const fromWeeks = (parsed.weeks ?? [])
    .map((w, i) => normalizeWeek(w, indexOffset + i))
    .filter((w): w is WeekSheetWeek => w !== null);

  if (fromWeeks.length > 0) return fromWeeks;

  const legacyEvents = (parsed.events ?? [])
    .map((ev, i) => normalizeEvent(ev, indexOffset * 1000 + i))
    .filter((ev): ev is WeekSheetEvent => ev !== null);

  if (legacyEvents.length === 0) return [];

  return [
    enrichWeekSheetWeek({
      weekLabel: parsed.weekLabel?.trim() || undefined,
      weekStart: parsed.weekStart?.trim() || inferWeekStartFromLabel(parsed.weekLabel),
      events: legacyEvents,
    }),
  ];
}

export async function parseWeekSheetWithMistral(ocrText: string): Promise<WeekSheetData> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) {
    throw new Error("Clé Mistral non configurée — impossible d'analyser la feuille.");
  }

  const sections = splitOcrByWeekSections(ocrText);
  const weeks: WeekSheetWeek[] = [];

  if (sections.length > 1) {
    for (let i = 0; i < sections.length; i++) {
      const parsed = await callMistral(apiKey, sections[i], true);
      weeks.push(...payloadToWeeks(parsed, i));
    }
  } else {
    const parsed = await callMistral(apiKey, ocrText, false);
    weeks.push(...payloadToWeeks(parsed, 0));
  }

  const uniqueWeeks = weeks.filter(
    (week, idx, arr) =>
      arr.findIndex(
        (other) =>
          (other.weekStart && other.weekStart === week.weekStart) ||
          (other.weekLabel && other.weekLabel === week.weekLabel),
      ) === idx,
  );

  if (uniqueWeeks.length === 0) {
    throw new Error("Aucun créneau reconnu dans la feuille — vérifiez le PDF.");
  }

  const base: WeekSheetData = {
    weeks: uniqueWeeks,
    weekLabel: uniqueWeeks[0].weekLabel,
    weekStart: uniqueWeeks[0].weekStart,
    events: uniqueWeeks[0].events,
    rawTextPreview: ocrText.slice(0, 500),
  };

  return pickActiveWeekSheet(base);
}
