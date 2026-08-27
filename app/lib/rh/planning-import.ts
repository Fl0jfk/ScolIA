import "server-only";

import { getMistralApiKey } from "@/app/lib/tenant-config";
import { runTextractForPdfBytes } from "@/app/lib/ocr-textract";
import { extractPdfTextItems } from "@/app/lib/rh/planning-pdf-text";
import {
  looksLikePronoteTeacherPdf,
  parsePronoteTeacherGrid,
} from "@/app/lib/rh/planning-pronote-parse";
import {
  normalizeStaffPlanning,
  normalizeTeacherPlanning,
  type RhPlanningKind,
  type StaffPlanningDoc,
  type TeacherPlanningDoc,
} from "@/app/lib/rh/planning-types";

function cleanMistralJson(raw: string): Record<string, unknown> {
  let text = raw.trim();
  text = text.replace(/`{3}json/gi, "").replace(/`{3}/g, "");
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1) throw new Error("Réponse IA invalide.");
  return JSON.parse(text.substring(start, end + 1)) as Record<string, unknown>;
}

function todayLabelFr(d = new Date()) {
  return d.toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function isoDateOnly(d = new Date()) {
  return d.toISOString().slice(0, 10);
}

type PlanningImportResult = {
  kind: RhPlanningKind;
  planning: TeacherPlanningDoc | StaffPlanningDoc;
  warnings: string[];
  ocrChars: number;
  personHint?: string;
};

async function chatJson(system: string, user: string): Promise<Record<string, unknown>> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) throw new Error("Service IA non configuré (clé Mistral).");

  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${mistralKey}`,
    },
    body: JSON.stringify({
      model: "mistral-medium",
      temperature: 0.1,
      response_format: { type: "json_object" },
      messages: [
        { role: "system", content: system },
        { role: "user", content: user },
      ],
    }),
  });

  const data = (await res.json()) as {
    choices?: { message?: { content?: string } }[];
    error?: { message?: string };
  };
  if (!res.ok) {
    throw new Error(data.error?.message || `Mistral HTTP ${res.status}`);
  }
  const content = data.choices?.[0]?.message?.content?.trim() || "";
  if (!content) throw new Error("Réponse IA vide.");
  return cleanMistralJson(content);
}

const TEACHER_SYSTEM = `Tu extrais un emploi du temps professeur depuis un texte OCR (PDF français établissement scolaire, souvent Pronote).
Réponds UNIQUEMENT en JSON valide.
Règles :
- day = 1 (lundi) … 5 (vendredi). Pas de week-end.
- Heures au format HH:MM (24h).
- classes = tableau de codes classe / groupe (ex. ["6A","5B"] ou codes Pronote).
- N'invente pas de créneaux absents du texte.
- personHint = nom du professeur si visible, sinon "".
- Semaines A/B (CRITIQUE, règles Pronote) :
  * Un créneau marqué (A) ou « sem. A » → UNIQUEMENT dans weekA.
  * Un créneau marqué (B) ou « sem. B » → UNIQUEMENT dans weekB.
  * Un créneau SANS marqueur A ni B → recopier dans weekA ET dans weekB (cours toutes semaines).
  * Ne laisse JAMAIS weekB avec seulement les créneaux marqués B si d'autres cours non marqués existent : ceux-ci doivent aussi être dans weekB.
  * Ces grilles sont des SEMAINES TYPES valables toute l'année scolaire (pas un calendrier daté).`;

const STAFF_SYSTEM = `Tu extrais un planning de personnel OGEC (admin, compta, maintenance, surveillant / vie scolaire) depuis un texte OCR.
Réponds UNIQUEMENT en JSON valide.
Règles :
- mode = "fixed" pour un planning de poste stable (admin/compta/maintenance) = SEMAINE TYPE ANNUELLE.
- mode = "rotation" pour des missions / lieux qui changent (surveillance : entrée, cour, étude, internat…).
- day = 1 (lundi) … 5 (vendredi).
- Heures HH:MM (24h).
- Pour fixed : fixedSlots avec label (poste / activité).
  * Si le PDF indique seulement des jours / une date de présence SANS horaires début/fin clairs :
    utilise une journée type 08:00–12:00 (matin) et 12:45–17:00 (après-midi) — pause repas 12:00–12:45.
    Mercredi : si non précisé, demi-journée 08:00–12:30 (un seul créneau) et signale hoursInferred=true.
  * Si horaires explicites dans le PDF, utilise-les tels quels et hoursInferred=false.
- Pour rotation : slots avec mission + location (lieu dans l'établissement : Entrée, Cour, Hall, Étude…).
- annualHoursTarget = nombre d'heures annuelles si visible sur le document (ex. 1607), sinon null.
- hoursInferred = true si tu as déduit des horaires par défaut.
- N'invente pas de jours absents du texte.
- personHint = nom de la personne si visible, sinon "".`;

async function tryExtractTeacherPronoteSpatial(input: {
  pdfBytes: Buffer | Uint8Array;
  personnelId: string;
  sourceFileName?: string;
}): Promise<PlanningImportResult | null> {
  try {
    const items = await extractPdfTextItems(input.pdfBytes);
    if (!looksLikePronoteTeacherPdf(items)) return null;
    const parsed = parsePronoteTeacherGrid(items);
    if (!parsed || parsed.slotCount < 3) return null;

    const planning = normalizeTeacherPlanning(
      {
        kind: "teacher",
        personnelId: input.personnelId,
        weekA: parsed.weekA,
        weekB: parsed.weekB,
        source: "pdf_import",
        sourceFileName: input.sourceFileName,
      },
      input.personnelId,
    );

    if (planning.weekA.length === 0 && planning.weekB.length === 0) return null;

    const warnings = [
      ...parsed.warnings,
      `Semaine A : ${planning.weekA.length} créneau(x) · Semaine B : ${planning.weekB.length} créneau(x).`,
      "Sans marqueur (A)/(B) = présent dans les deux semaines ; (A) = A seulement ; (B) = B seulement.",
      "Semaines A/B = semaines types pour toute l’année scolaire.",
    ];

    return {
      kind: "teacher",
      planning: {
        ...planning,
        source: "pdf_import",
        sourceFileName: input.sourceFileName,
      },
      warnings,
      ocrChars: items.length,
      personHint: parsed.personHint || undefined,
    };
  } catch (e) {
    console.warn("[planning-import] parse Pronote spatial échoué, repli OCR/IA", e);
    return null;
  }
}

export async function extractPlanningFromPdfBytes(input: {
  pdfBytes: Buffer | Uint8Array;
  personnelId: string;
  kind: RhPlanningKind;
  /** Hint si staff : force le mode attendu. */
  preferredStaffMode?: "fixed" | "rotation";
  sourceFileName?: string;
}): Promise<PlanningImportResult> {
  if (input.kind === "teacher") {
    const spatial = await tryExtractTeacherPronoteSpatial(input);
    if (spatial) return spatial;
  }

  const ocr = await runTextractForPdfBytes(input.pdfBytes);
  const text = ocr.text?.trim() || "";
  if (text.length < 40) {
    throw new Error("OCR trop pauvre : PDF illisible ou vide. Réessayez avec un PDF texte / scanné plus net.");
  }

  const warnings: string[] = [];
  const truncated = text.slice(0, 100_000);
  if (text.length > 100_000) warnings.push("Document long : seule la première partie a été analysée.");

  if (input.kind === "teacher") {
    const parsed = await chatJson(
      TEACHER_SYSTEM,
      `personnelId=${input.personnelId}\n\nTexte OCR:\n---\n${truncated}\n---\n\nSchéma JSON:\n{"personHint":"","weekA":[{"day":1,"start":"08:00","end":"09:00","subject":"Maths","classes":["6A"],"room":"S12"}],"weekB":[],"notes":""}`,
    );

    const planning = normalizeTeacherPlanning(
      {
        kind: "teacher",
        personnelId: input.personnelId,
        weekA: parsed.weekA,
        weekB: parsed.weekB,
        source: "pdf_import",
        sourceFileName: input.sourceFileName,
      },
      input.personnelId,
    );

    if (planning.weekA.length === 0 && planning.weekB.length === 0) {
      warnings.push("Aucun créneau détecté — vérifiez le PDF ou corrigez à la main.");
    } else if (planning.weekB.length === 0) {
      warnings.push("Aucune semaine B détectée : tous les créneaux sont en semaine A (modifiable avant validation).");
    }
    warnings.push("Semaines A/B = semaines types pour toute l’année scolaire.");
    warnings.push("Import via OCR + IA (grille Pronote non détectée en mode spatial).");

    return {
      kind: "teacher",
      planning: {
        ...planning,
        source: "pdf_import",
        sourceFileName: input.sourceFileName,
      },
      warnings,
      ocrChars: text.length,
      personHint: typeof parsed.personHint === "string" ? parsed.personHint.trim() : undefined,
    };
  }

  const preferred = input.preferredStaffMode || "fixed";
  const parsed = await chatJson(
    STAFF_SYSTEM,
    `personnelId=${input.personnelId}\nmode_préféré=${preferred}\n\nTexte OCR:\n---\n${truncated}\n---\n\nSchéma JSON:\n{"personHint":"","mode":"fixed|rotation","hoursInferred":false,"annualHoursTarget":null,"fixedSlots":[{"day":1,"start":"08:00","end":"12:00","label":"Matin"},{"day":1,"start":"12:45","end":"17:00","label":"Après-midi"}],"slots":[{"day":1,"start":"08:00","end":"09:00","mission":"Surveillance","location":"Entrée établissement"}],"notes":""}`,
  );

  const mode =
    parsed.mode === "rotation" || preferred === "rotation"
      ? "rotation"
      : parsed.mode === "fixed"
        ? "fixed"
        : preferred;

  const label = `Import ${todayLabelFr()}`;
  const annualHoursTarget =
    typeof parsed.annualHoursTarget === "number"
      ? parsed.annualHoursTarget
      : typeof parsed.annualHoursTarget === "string" && parsed.annualHoursTarget
        ? Number(String(parsed.annualHoursTarget).replace(",", "."))
        : undefined;

  const rawDoc =
    mode === "rotation"
      ? {
          kind: "staff" as const,
          personnelId: input.personnelId,
          mode: "rotation" as const,
          fixedSlots: [],
          rotations: [
            {
              id: `rot_import_${Date.now().toString(36)}`,
              label,
              startDate: isoDateOnly(),
              endDate: null,
              slots: Array.isArray(parsed.slots) ? parsed.slots : [],
            },
          ],
          exceptions: [],
          source: "pdf_import" as const,
          sourceFileName: input.sourceFileName,
        }
      : {
          kind: "staff" as const,
          personnelId: input.personnelId,
          mode: "fixed" as const,
          fixedSlots: Array.isArray(parsed.fixedSlots)
            ? parsed.fixedSlots
            : Array.isArray(parsed.slots)
              ? (parsed.slots as { day?: unknown; start?: unknown; end?: unknown; mission?: unknown; label?: unknown }[]).map(
                  (s) => ({
                    day: s.day,
                    start: s.start,
                    end: s.end,
                    label: s.label || s.mission || "Poste",
                  }),
                )
              : [],
          rotations: [],
          exceptions: [],
          annualHoursTarget: Number.isFinite(annualHoursTarget) ? annualHoursTarget : undefined,
          source: "pdf_import" as const,
          sourceFileName: input.sourceFileName,
        };

  const planning = normalizeStaffPlanning(rawDoc, input.personnelId);
  const slotCount =
    planning.mode === "fixed"
      ? planning.fixedSlots.length
      : planning.rotations.reduce((n, r) => n + r.slots.length, 0);

  if (slotCount === 0) {
    warnings.push("Aucun créneau détecté — vérifiez le PDF ou corrigez à la main.");
  }
  if (mode === "rotation") {
    warnings.push(
      "Import missions : une nouvelle variante datée sera proposée. À la validation, elle remplace ou s’ajoute selon votre choix.",
    );
  } else {
    warnings.push("Semaine type annuelle — chaque personne peut ajuster ses horaires ensuite.");
    if (parsed.hoursInferred === true) {
      warnings.push(
        "Horaires déduits (souvent 8h–17h avec pause 12h–12h45 ; mercredi souvent demi-journée) — à faire ajuster par la personne.",
      );
    }
    if (planning.annualHoursTarget) {
      warnings.push(`Quota annuel détecté : ${planning.annualHoursTarget} h.`);
    }
  }

  return {
    kind: "staff",
    planning: {
      ...planning,
      source: "pdf_import",
      sourceFileName: input.sourceFileName,
    },
    warnings,
    ocrChars: text.length,
    personHint: typeof parsed.personHint === "string" ? parsed.personHint.trim() : undefined,
  };
}

/** Fusionne un import staff rotation avec l’existant (nouvelle variante en tête, historique conservé). */
export function mergeStaffImport(
  existing: StaffPlanningDoc | null,
  imported: StaffPlanningDoc,
  strategy: "replace" | "append_rotation",
): StaffPlanningDoc {
  if (imported.mode !== "rotation" || strategy === "replace") {
    return imported;
  }
  const prev = existing?.rotations || [];
  const incoming = imported.rotations[0];
  if (!incoming) return imported;
  const merged = [incoming, ...prev.filter((r) => r.id !== incoming.id)].slice(0, 12);
  return {
    ...imported,
    mode: "rotation",
    rotations: merged,
    fixedSlots: [],
    exceptions: existing?.exceptions || imported.exceptions || [],
    annualHoursTarget: imported.annualHoursTarget ?? existing?.annualHoursTarget,
  };
}
