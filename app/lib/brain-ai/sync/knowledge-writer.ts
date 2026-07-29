import {
  appendEntryToKnowledgeFile,
  readKnowledgeDocument,
  readKnowledgeIndex,
} from "@/app/lib/knowledge";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";

const ACTUALITE_DOMAIN_ID = "actualite";
const ACTUALITE_FILE = "actualite.json";

async function getKnowledgeBucket() {
  const { getTenantBucketName } = await import("@/app/lib/tenant-config");
  return getTenantBucketName();
}

function knowledgeKey(file: string) {
  const prefix = (process.env.KNOWLEDGE_PREFIX || "knowledge").replace(/^\/+|\/+$/g, "");
  return `${prefix}/${file}`;
}

async function writeKnowledgeJson(file: string, data: unknown) {
  const s3 = await getTenantDataS3Client();
  const bucket = await getKnowledgeBucket();
  const key = knowledgeKey(file);
  const signed = await getSignedUrl(
    s3,
    new PutObjectCommand({
      Bucket: bucket,
      Key: key,
      ContentType: "application/json; charset=utf-8",
    }),
    { expiresIn: 120 },
  );
  const res = await fetch(signed, {
    method: "PUT",
    headers: { "Content-Type": "application/json; charset=utf-8" },
    body: JSON.stringify(data, null, 2),
  });
  if (!res.ok) throw new Error(`Ecriture knowledge S3 impossible (${res.status})`);
}

async function ensureActualiteDomain(): Promise<string> {
  try {
    const index = await readKnowledgeIndex();
    const existing = index.domains.find((d) => d.id === ACTUALITE_DOMAIN_ID);
    if (existing) return existing.file;

    index.domains.push({
      id: ACTUALITE_DOMAIN_ID,
      label: "Actualité établissement",
      file: ACTUALITE_FILE,
      isYearlyReset: true,
      keywords: [
        "actualité",
        "semaine",
        "feuille",
        "voyage",
        "séjour",
        "sortie",
        "aujourd'hui",
        "planning",
      ],
    });
    index.updatedAt = new Date().toISOString().slice(0, 10);
    await writeKnowledgeJson("index.json", index);

    try {
      await readKnowledgeDocument(ACTUALITE_FILE);
    } catch {
      await writeKnowledgeJson(ACTUALITE_FILE, {
        domainId: ACTUALITE_DOMAIN_ID,
        updatedAt: new Date().toISOString().slice(0, 10),
        entries: [],
      });
    }
    return ACTUALITE_FILE;
  } catch (err) {
    console.warn("[brain-ai/sync] ensureActualiteDomain failed", err);
    return ACTUALITE_FILE;
  }
}

/** Upsert non bloquant d'une fiche actualité (tools = vérité ; knowledge = rappel RAG). */
export async function syncActualiteEntry(entry: {
  entryId: string;
  title: string;
  content: string;
  source: string;
}): Promise<void> {
  try {
    const file = await ensureActualiteDomain();
    let doc: Awaited<ReturnType<typeof readKnowledgeDocument>>;
    try {
      doc = await readKnowledgeDocument(file);
    } catch {
      doc = {
        domainId: ACTUALITE_DOMAIN_ID,
        updatedAt: new Date().toISOString().slice(0, 10),
        entries: [],
      };
    }
    if (!Array.isArray(doc.entries)) doc.entries = [];
    const now = new Date().toISOString();
    const next = {
      id: entry.entryId,
      title: entry.title,
      content: entry.content,
      source: entry.source,
      audiences: ["private" as const],
      updatedAt: now,
    };
    const idx = doc.entries.findIndex((e) => e.id === entry.entryId || e.source === entry.source);
    if (idx >= 0) doc.entries[idx] = { ...doc.entries[idx], ...next };
    else doc.entries.unshift(next);
    doc.updatedAt = now.slice(0, 10);
    await writeKnowledgeJson(file, doc);
  } catch (err) {
    console.warn("[brain-ai/sync] syncActualiteEntry failed", err);
  }
}

export async function syncWeekSheetActualite(summary: {
  weekLabel?: string;
  weekStart?: string;
  eventCount: number;
  todayTitles?: string[];
}): Promise<void> {
  const titles = (summary.todayTitles || []).slice(0, 8).join(" · ") || "(aucun événement listé)";
  await syncActualiteEntry({
    entryId: "actualite-feuille-semaine",
    title: `Feuille de semaine${summary.weekLabel ? ` — ${summary.weekLabel}` : ""}`,
    content:
      `Semaine ${summary.weekStart || "en cours"} (${summary.eventCount} événement(s)). ` +
      `Aperçu : ${titles}.`,
    source: "sync:week-sheet",
  });
}

export async function syncTripActualite(trip: {
  id: string;
  title?: string;
  dates?: string;
  classes?: string;
  statusLabel?: string;
}): Promise<void> {
  await syncActualiteEntry({
    entryId: `actualite-travel-${trip.id}`,
    title: trip.title || `Séjour ${trip.id}`,
    content:
      `Séjour « ${trip.title || trip.id} »` +
      (trip.dates ? ` — dates : ${trip.dates}` : "") +
      (trip.classes ? ` — classes : ${trip.classes}` : "") +
      (trip.statusLabel ? ` — statut : ${trip.statusLabel}` : "") +
      `.`,
    source: `sync:travels/${trip.id}`,
  });
}

/** Helper legacy si un domaine file existe déjà. */
export async function appendActualiteViaDomain(entry: {
  title: string;
  content: string;
  source: string;
}) {
  try {
    const file = await ensureActualiteDomain();
    await appendEntryToKnowledgeFile(file, {
      title: entry.title,
      content: entry.content,
      source: entry.source,
      audiences: ["private"],
    });
  } catch (err) {
    console.warn("[brain-ai/sync] appendActualiteViaDomain failed", err);
  }
}
