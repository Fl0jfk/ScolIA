import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";

type KnowledgeDomain = {
  id: string;
  label: string;
  file: string;
  isYearlyReset: boolean;
  keywords: string[];
};

type KnowledgeIndex = {
  version: number;
  updatedAt: string;
  domains: KnowledgeDomain[];
};

type KnowledgeEntry = {
  id: string;
  title: string;
  content: string;
  source: string;
  audiences?: Array<"public" | "private">;
  updatedAt?: string;
};

type KnowledgeDocument = {
  domainId: string;
  schoolYear?: string;
  updatedAt: string;
  entries: KnowledgeEntry[];
};

async function getKnowledgeBucket() {
  const { getTenantBucketName } = await import("@/app/lib/tenant-config");
  return getTenantBucketName();
}

function knowledgeKey(file: string) {
  const prefix = (process.env.KNOWLEDGE_PREFIX || "knowledge").replace(/^\/+|\/+$/g, "");
  return `${prefix}/${file}`;
}

async function readKnowledgeJsonFromS3<T>(file: string): Promise<T> {
  const s3 = await getTenantDataS3Client();
  const bucket = await getKnowledgeBucket();
  const key = knowledgeKey(file);
  const signed = await getSignedUrl( s3, new GetObjectCommand({ Bucket: bucket, Key: key }), { expiresIn: 120 });
  const res = await fetch(signed, { cache: "no-store" });
  if (!res.ok) { throw new Error(`Lecture knowledge S3 impossible (${res.status}) pour ${key}`)}
  return (await res.json()) as T;
}

async function writeKnowledgeJsonToS3(file: string, data: unknown) {
  const s3 = await getTenantDataS3Client();
  const bucket = await getKnowledgeBucket();
  const key = knowledgeKey(file);
  const signed = await getSignedUrl( s3,new PutObjectCommand({ Bucket: bucket, Key: key, ContentType: "application/json; charset=utf-8",}),{ expiresIn: 120 });
  const payload = JSON.stringify(data, null, 2);
  const res = await fetch(signed, {
    method: "PUT",
    headers: { "Content-Type": "application/json; charset=utf-8" },
    body: payload,
  });
  if (!res.ok) { throw new Error(`Ecriture knowledge S3 impossible (${res.status})`);}
}

export async function readKnowledgeIndex(): Promise<KnowledgeIndex> {  return readKnowledgeJsonFromS3<KnowledgeIndex>("index.json")}

export async function readKnowledgeDocument(file: string): Promise<KnowledgeDocument> { return readKnowledgeJsonFromS3<KnowledgeDocument>(file)}

export function selectDomainByMessage(domains: KnowledgeDomain[], message: string): KnowledgeDomain {
  const text = message.toLowerCase();
  let best = domains[0];
  let bestScore = -1;
  for (const domain of domains) {
    const score = domain.keywords.reduce((acc, keyword) => (text.includes(keyword.toLowerCase()) ? acc + 1 : acc), 0);
    if (score > bestScore) {
      bestScore = score;
      best = domain;
    }
  }
  return best;
}

function relevanceScore(query: string, entry: KnowledgeEntry) {
  const tokens = query.toLowerCase().split(/[^a-z0-9à-ÿ]+/i).filter((t) => t.length > 2);
  const hay = `${entry.title} ${entry.content}`.toLowerCase();
  const keywordHits = tokens.reduce((acc, t) => (hay.includes(t) ? acc + 1 : acc), 0);
  const recencyBonus = entry.updatedAt ? 1 : 0;
  return keywordHits * 2 + recencyBonus;
}

function hasPastIntent(query: string) {
  const q = query.toLowerCase();
  return /(\b(année dernière|an dernier|l'an dernier|passé|passée|mois dernier|semaine dernière|historique|archive)\b|\b\d{4}\b|\b\d{1,2}[\/\-]\d{1,2}(?:[\/\-]\d{2,4})?\b)/i.test(
    q
  );
}

function isRecentEntry(entry: KnowledgeEntry, recentDays: number) {
  const raw = entry.updatedAt;
  if (!raw) return false;
  const ts = Date.parse(raw);
  if (Number.isNaN(ts)) return false;
  const ageMs = Date.now() - ts;
  return ageMs <= recentDays * 24 * 60 * 60 * 1000;
}

export function buildContextFromEntries(
  domain: KnowledgeDomain,
  doc: KnowledgeDocument,
  audience: "public" | "private",
  maxEntries = 8,
  query = "",
  recentDays = 90
): string {
  const keepHistorical = hasPastIntent(query);
  const applyRecencyFilter = domain.isYearlyReset && !keepHistorical;
  const rows = doc.entries.filter((entry) => !entry.audiences || entry.audiences.includes(audience)).filter((entry) => !applyRecencyFilter || isRecentEntry(entry, recentDays)).sort((a, b) => relevanceScore(query, b) - relevanceScore(query, a)).slice(0, maxEntries).map((entry) => `- [${entry.title}] ${entry.content} (source: ${entry.source})`);
  return rows.join("\n");
}

export async function appendEntryToKnowledgeFile( file: string, entry: { title: string; content: string; source: string; audiences?: Array<"public" | "private"> }) {
  const doc = await readKnowledgeDocument(file);
  if (!Array.isArray(doc.entries)) { doc.entries = []}
  const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  doc.entries.unshift({
    id,
    title: entry.title,
    content: entry.content,
    source: entry.source,
    audiences: entry.audiences ?? ["public", "private"],
    updatedAt: new Date().toISOString(),
  });
  doc.updatedAt = new Date().toISOString().slice(0, 10);
  await writeKnowledgeJsonToS3(file, doc);
}