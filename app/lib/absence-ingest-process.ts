import { DeleteObjectCommand, GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { runTextractForPdfBytes } from "@/app/lib/ocr-textract";
import {
  readIngestJob,
  writeIngestJob,
  type IngestJob,
  type IngestJobCreated,
  type IngestJobPhase,
} from "@/app/api/absences/ingest/ingest-job";
import {
  buildAdminAbsenceRecord,
  normalizeEtablissement,
  type AbsenceRecord,
  type Etablissement,
} from "@/app/lib/absences-types";
import { getAbsenceIndex, purgeExpiredAbsences, saveAbsenceIndex, saveOrMergeAbsenceRecord } from "@/app/lib/absences-storage";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName, requireMistralApiKey } from "@/app/lib/tenant-config";

const RUN_LOCK_PREFIX = "absences/ingest-locks/";
const MISTRAL_TIMEOUT_MS = 120_000;
const MISTRAL_OCR_SLICE = 16_000;

type ParsedSlot = { startAt: string; endAt: string };
type ParsedConvocation = {
  teacherName: string;
  examType: string;
  etablissement: Etablissement;
  sourceDocument: string;
  documentKey: string;
  confidence: number;
  slots: ParsedSlot[];
};


function runLockKey(jobId: string) {
  return `${RUN_LOCK_PREFIX}${jobId}.lock`;
}

/** Un seul worker par jobId (S3 create-if-absent). */
async function acquireRunLock(jobId: string): Promise<boolean> {
  const s3Client = await getTenantDataS3Client();
  try {
    await s3Client.send(
      new PutObjectCommand({
        Bucket: (await getTenantBucketName()),
        Key: runLockKey(jobId),
        Body: JSON.stringify({ acquiredAt: new Date().toISOString() }),
        ContentType: "application/json",
        IfNoneMatch: "*",
      }),
    );
    return true;
  } catch (e: unknown) {
    const meta = (e as { $metadata?: { httpStatusCode?: number } }).$metadata;
    const name = (e as { name?: string }).name;
    if (meta?.httpStatusCode === 412 || name === "PreconditionFailed") return false;
    throw e;
  }
}

async function releaseRunLock(jobId: string) {
  const s3Client = await getTenantDataS3Client();
  try {
    await s3Client.send(
      new DeleteObjectCommand({ Bucket: (await getTenantBucketName()), Key: runLockKey(jobId) }),
    );
  } catch {
    /* ignore */
  }
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const norm = (s: string) =>
  String(s || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, " ")
    .trim();

export function mapIngestFailureMessage(error: unknown) {
  const raw = error instanceof Error ? error.message : String(error || "");
  const msg = raw.toLowerCase();

  if (msg.includes("textract") && (msg.includes("timeout") || msg.includes("délai"))) {
    return {
      code: "OCR_TIMEOUT",
      error: "Lecture du PDF trop longue (OCR). Réessayez ou utilisez un PDF plus léger.",
    };
  }
  if (msg.includes("textract") || msg.includes("texte vide")) {
    return {
      code: "OCR_FAILED",
      error: "Impossible de lire le texte du PDF (scan flou, image ou fichier protégé).",
    };
  }
  if (msg.includes("mistral") || msg.includes("json introuvable") || msg.includes("abort")) {
    return {
      code: "AI_FAILED",
      error: "Analyse IA impossible sur ce document. Réessayez ou complétez en saisie manuelle.",
    };
  }
  if (msg.includes("credentials") || msg.includes("access denied") || msg.includes("bucket")) {
    return {
      code: "STORAGE_FAILED",
      error: "Erreur de stockage du fichier. Contactez l'administrateur si le problème persiste.",
    };
  }
  if (msg.includes("mistral_api_key") || msg.includes("api key")) {
    return {
      code: "AI_CONFIG",
      error: "Service IA non configuré (clé API manquante).",
    };
  }

  return {
    code: "INGEST_FAILED",
    error: "Erreur technique pendant l'import. Réessayez dans quelques instants.",
    detail: raw.slice(0, 280),
  };
}

async function patchJob(jobId: string, patch: Partial<IngestJob>) {
  const job = await readIngestJob(jobId);
  if (!job) return;
  await writeIngestJob({ ...job, ...patch });
}

async function downloadS3Pdf(key: string): Promise<Uint8Array> {
  const s3Client = await getTenantDataS3Client();
  const res = await s3Client.send(
    new GetObjectCommand({ Bucket: (await getTenantBucketName()), Key: key }),
  );
  const bytes = await res.Body?.transformToByteArray();
  if (!bytes?.length) throw new Error("OCR: PDF introuvable sur S3.");
  return bytes;
}

/** OCR du PDF via Mistral OCR — délègue à la façade unifiée. */
async function runOcrForS3Key(key: string): Promise<string> {
  const bytes = await downloadS3Pdf(key);
  const result = await runTextractForPdfBytes(bytes);
  if (!result.text.trim()) throw new Error("OCR: texte vide.");
  return result.text;
}

function parseJsonFromMistral(content: string) {
  const raw = String(content || "")
    .trim()
    .replace(/```json/gi, "")
    .replace(/```/g, "")
    .trim();
  const start = raw.indexOf("{");
  const end = raw.lastIndexOf("}");
  if (start < 0 || end < 0) throw new Error("Mistral: JSON introuvable.");
  return JSON.parse(raw.slice(start, end + 1));
}

function toIso(value: string) {
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return null;
  return d.toISOString();
}

function readSlotBoundary(slot: Record<string, unknown>, keys: string[]) {
  for (const key of keys) {
    const raw = slot[key];
    if (raw === undefined || raw === null) continue;
    const value = String(raw).trim();
    if (value) return value;
  }
  return "";
}

function normalizeParsedSlots(raw: unknown): ParsedSlot[] {
  if (!Array.isArray(raw)) return [];
  const out: ParsedSlot[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const slot = item as Record<string, unknown>;
    const startAt = readSlotBoundary(slot, ["startAt", "start", "debut", "début", "dateDebut", "date_debut"]);
    const endAt = readSlotBoundary(slot, ["endAt", "end", "fin", "dateFin", "date_fin"]);
    if (!startAt || !endAt) continue;
    const startIso = toIso(startAt);
    const endIso = toIso(endAt);
    if (!startIso || !endIso) continue;
    if (new Date(endIso) <= new Date(startIso)) continue;
    out.push({ startAt: startIso, endAt: endIso });
  }
  return out;
}

function dedupeSlots(slots: ParsedSlot[]) {
  const seen = new Set<string>();
  const out: ParsedSlot[] = [];
  for (const slot of slots) {
    const key = `${slot.startAt}|${slot.endAt}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(slot);
  }
  return out.sort((a, b) => +new Date(a.startAt) - +new Date(b.startAt));
}

const PARIS_TZ = "Europe/Paris";

function parisDateKey(iso: string): string {
  return new Intl.DateTimeFormat("en-CA", { timeZone: PARIS_TZ }).format(new Date(iso));
}

function addCalendarDay(dateKey: string): string {
  const [y, m, d] = dateKey.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d + 1)).toISOString().slice(0, 10);
}

/** Heure murale à Paris → instant UTC (gère +01 / +02). */
function parisWallTimeToUtc(
  dateKey: string,
  hour: number,
  minute: number,
  second: number,
  ms: number,
): number {
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  for (const offset of ["+02:00", "+01:00"]) {
    const iso = `${dateKey}T${pad(hour)}:${pad(minute)}:${pad(second)}.${pad(ms, 3)}${offset}`;
    if (parisDateKey(iso) === dateKey) return +new Date(iso);
  }
  return +new Date(`${dateKey}T${pad(hour)}:${pad(minute)}:${pad(second)}+02:00`);
}

function listParisDateKeysFromTo(startIso: string, endIso: string): string[] {
  const keys: string[] = [];
  let k = parisDateKey(startIso);
  const last = parisDateKey(endIso);
  while (true) {
    keys.push(k);
    if (k === last) break;
    k = addCalendarDay(k);
  }
  return keys;
}

/** Découpe un créneau multi-jours en segments par jour (heures réelles chaque jour). */
function expandSlotToDaySegments(slot: ParsedSlot): ParsedSlot[] {
  const startMs = +new Date(slot.startAt);
  const endMs = +new Date(slot.endAt);
  if (endMs <= startMs) return [];

  const out: ParsedSlot[] = [];
  for (const dayKey of listParisDateKeysFromTo(slot.startAt, slot.endAt)) {
    const dayStart = parisWallTimeToUtc(dayKey, 0, 0, 0, 0);
    const dayEnd = parisWallTimeToUtc(dayKey, 23, 59, 59, 999);
    const segStart = Math.max(startMs, dayStart);
    const segEnd = Math.min(endMs, dayEnd);
    if (segEnd > segStart) {
      out.push({
        startAt: new Date(segStart).toISOString(),
        endAt: new Date(segEnd).toISOString(),
      });
    }
  }
  return out;
}

/** Un seul créneau par jour calendaire (Paris) : heure de début la plus tôt, fin la plus tard. */
function mergeOneSlotPerCalendarDay(slots: ParsedSlot[]): ParsedSlot[] {
  const byDay = new Map<string, ParsedSlot>();
  for (const slot of dedupeSlots(slots)) {
    for (const seg of expandSlotToDaySegments(slot)) {
      const key = parisDateKey(seg.startAt);
      const cur = byDay.get(key);
      if (!cur) {
        byDay.set(key, { ...seg });
        continue;
      }
      if (+new Date(seg.startAt) < +new Date(cur.startAt)) cur.startAt = seg.startAt;
      if (+new Date(seg.endAt) > +new Date(cur.endAt)) cur.endAt = seg.endAt;
    }
  }
  return [...byDay.values()].sort((a, b) => +new Date(a.startAt) - +new Date(b.startAt));
}

function buildIngestedAbsenceRecord(params: {
  etablissement: Etablissement;
  examType: string;
  teacherName: string;
  sourceDocument: string;
  documentKey: string;
  confidence: number;
  slot: ParsedSlot;
  createdBy: AbsenceRecord["createdBy"];
}): AbsenceRecord {
  return buildAdminAbsenceRecord({
    source: "admin_pdf",
    displayName: params.teacherName,
    etablissement: params.etablissement,
    reason: params.examType,
    startAt: params.slot.startAt,
    endAt: params.slot.endAt,
    documentKeys: params.documentKey ? [params.documentKey] : [],
    sourceDocument: params.sourceDocument,
    confidence: params.confidence,
    createdBy: params.createdBy,
  });
}

async function parseConvocationWithMistral(text: string, sourceDocument: string): Promise<ParsedConvocation> {
  const ocrSlice = text.slice(0, MISTRAL_OCR_SLICE);
  const prompt = `
Tu analyses une convocation d'examen (ou document assimilé) pour un professeur convoqué.
Réponds strictement en JSON valide, sans texte avant ni après.

RÈGLE PRINCIPALE — UN SLOT PAR JOUR CALENDAIRE (fuseau Europe/Paris):
- Chaque jour distinct où le professeur est convoqué = exactement UN objet dans "slots".
- Le même jour ne doit jamais apparaître deux fois (ex. matin + après-midi le 19/06 → UN seul slot ce jour-là).
- Pour ce jour : "startAt" = première heure de convocation ce jour-là, "endAt" = dernière heure ce jour-là (garder les vraies heures du document).
- Jours différents = slots différents (ex. lundi 3 juin et vendredi 14 juin → 2 slots).

RÈGLE POURSUITE SUR PLUSIEURS JOURS (ex. bac, décoration de copies):
- Si une mission s'étend sur plusieurs jours calendaires, produire UN slot PAR jour concerné, chacun avec les heures réelles ce jour-là (pas un seul slot sur toute la période).
- Ex. vendredi 19/06 14h30–17h30 puis poursuite jusqu'au 01/07 16h00 → un slot pour chaque jour du 19/06 au 01/07 avec les bornes horaires de ce jour (premier jour dès 14h30, dernier jour jusqu'à 16h00, jours intermédiaires sur la plage indiquée ou journée complète si le texte l'impose).

Tu dois parcourir TOUT le texte OCR (toutes les pages).
Si seule la date est indiquée sans heure, utiliser 08:00–18:00 ce jour-là (Europe/Paris).

Format JSON:
{
  "teacherName": "Prénom NOM",
  "examType": "brevet|bac|oral|convocation…",
  "etablissement": "École|Collège|Lycée",
  "sourceDocument": "${sourceDocument}",
  "confidence": 0.0,
  "slots": [{ "startAt": "ISO8601", "endAt": "ISO8601" }]
}

Texte OCR:
---
${ocrSlice}
---
`;

  const mistralKey = await requireMistralApiKey();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), MISTRAL_TIMEOUT_MS);
  try {
    const response = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      signal: controller.signal,
      headers: {
        Authorization: `Bearer ${mistralKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        temperature: 0,
        response_format: { type: "json_object" },
        messages: [{ role: "user", content: prompt }],
      }),
    });

    if (!response.ok) {
      const err = await response.text();
      throw new Error(`Mistral: ${err}`);
    }

    const payload = await response.json();
    const content = payload?.choices?.[0]?.message?.content || "";
    const json = parseJsonFromMistral(content) as ParsedConvocation & { slots?: unknown };
    return {
      teacherName: String(json?.teacherName || "").trim(),
      examType: String(json?.examType || "Examen").trim(),
      etablissement: normalizeEtablissement(String(json?.etablissement || "")),
      sourceDocument,
      documentKey: "",
      confidence: Number(json?.confidence || 0),
      slots: normalizeParsedSlots(json?.slots),
    };
  } finally {
    clearTimeout(timer);
  }
}

const PROCESSING_ACTIVE_MS = 90 * 1000;

/** Évite deux exécutions parallèles ; relance si un traitement semble bloqué. */
export async function tryClaimIngestJob(jobId: string): Promise<boolean> {
  const job = await readIngestJob(jobId);
  if (!job) return false;
  if (job.status === "completed" || job.status === "failed") return false;

  if (job.status === "processing" && job.processingStartedAt) {
    const age = Date.now() - new Date(job.processingStartedAt).getTime();
    if (age < PROCESSING_ACTIVE_MS) return false;
    console.warn("[absence-ingest] reprise job stale", jobId, age);
    await releaseRunLock(jobId);
  }

  if (job.status !== "pending" && job.status !== "processing") return false;

  await writeIngestJob({
    ...job,
    status: "processing",
    processingStartedAt: new Date().toISOString(),
    phase: "ocr",
  });
  return true;
}

export async function runAbsenceIngestJob(jobId: string, documentKey: string, sourceFileName: string) {
  let existing = await readIngestJob(jobId);
  if (!existing) return;
  if (existing.status === "completed" || existing.status === "failed") return;

  if (!(await acquireRunLock(jobId))) {
    return;
  }

  existing = await readIngestJob(jobId);
  if (!existing || existing.status === "completed" || existing.status === "failed") {
    await releaseRunLock(jobId);
    return;
  }

  const fail = async (error: string, code: string, parsed?: Record<string, unknown>) => {
    const j = await readIngestJob(jobId);
    if (!j || j.status === "completed") return;
    await writeIngestJob({
      ...j,
      status: "failed",
      error,
      code,
      parsed: parsed ?? j.parsed,
      phase: undefined,
    });
  };

  const setPhase = async (phase: IngestJobPhase) => {
    await patchJob(jobId, { phase });
  };

  try {
    await setPhase("ocr");
    const ocrText = await runOcrForS3Key(documentKey);

    await setPhase("ai");
    const parsed = await parseConvocationWithMistral(ocrText, sourceFileName);
    parsed.documentKey = documentKey;

    if (!parsed.teacherName || parsed.slots.length === 0) {
      const hints: string[] = [];
      if (!parsed.teacherName) hints.push("nom du professeur non reconnu");
      if (parsed.slots.length === 0) hints.push("dates/heures non détectées");
      await fail(
        `Extraction incomplète (${hints.join(", ") || "données manquantes"}). Vérifiez la qualité du scan ou saisissez l'absence à la main.`,
        "EXTRACTION_INCOMPLETE",
        parsed as unknown as Record<string, unknown>,
      );
      return;
    }

    const normalizedSlots = mergeOneSlotPerCalendarDay(parsed.slots);
    if (normalizedSlots.length === 0) {
      await fail(
        "Les dates extraites sont invalides (format ou ordre incohérent). Essayez une saisie manuelle.",
        "INVALID_SLOTS",
        parsed as unknown as Record<string, unknown>,
      );
      return;
    }

    existing = await readIngestJob(jobId);
    if (existing?.status === "completed") return;

    await setPhase("saving");
    const jobMeta = await readIngestJob(jobId);
    const createdBy: AbsenceRecord["createdBy"] = {
      userId: jobMeta?.userId || "system",
      name: jobMeta?.creatorName || "Import PDF",
      email: jobMeta?.creatorEmail || "",
      roles: jobMeta?.creatorRoles || [],
    };
    const createdRecords: AbsenceRecord[] = [];
    let workingIndex = await purgeExpiredAbsences(await getAbsenceIndex());
    for (const slot of normalizedSlots) {
      const record = buildIngestedAbsenceRecord({
        etablissement: parsed.etablissement,
        examType: parsed.examType,
        teacherName: parsed.teacherName,
        sourceDocument: parsed.sourceDocument,
        documentKey: parsed.documentKey,
        confidence: parsed.confidence,
        slot,
        createdBy,
      });
      const { index: nextIndex, record: saved } = await saveOrMergeAbsenceRecord(
        workingIndex,
        record,
        createdBy.name,
      );
      workingIndex = nextIndex;
      createdRecords.push(saved);
    }
    await saveAbsenceIndex(workingIndex);

    const created: IngestJobCreated[] = createdRecords.map((r) => ({
      id: r.id,
      teacherName: r.displayName,
      startDate: r.data.startDate,
      endDate: r.data.endDate,
    }));

    await writeIngestJob({
      ...(await readIngestJob(jobId))!,
      status: "completed",
      created,
      parsed: parsed as unknown as Record<string, unknown>,
      error: undefined,
      code: undefined,
      phase: undefined,
    });
  } catch (error) {
    console.error("[absence-ingest] job:", error);
    const mapped = mapIngestFailureMessage(error);
    await fail(mapped.error, mapped.code || "INGEST_FAILED");
  } finally {
    await releaseRunLock(jobId);
  }
}
