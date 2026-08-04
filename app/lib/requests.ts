import { DeleteObjectCommand, DeleteObjectsCommand, ListObjectsV2Command, PutObjectCommand} from "@aws-sdk/client-s3";
import { getJson, putJson, putObject, getS3Client, getBucketName, getObjectBytes } from "@/app/lib/s3-storage";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { s3Key } from "@/app/lib/s3-path";
import { LEGACY_ROUTE_TO_BRANCH, normalizeRequestBranchId, normalizeRequestEmail, isCorbeilleBranchId} from "@/app/lib/requests-board";
import { getFirstBranchForStaffEmailFromDirectory } from "@/app/lib/staff-directory";
import { ensureRequestRoutes, getRouteById } from "@/app/lib/requests-routes-cache";
import type { RequestRouteDef } from "@/app/lib/requests-types";
import { resolveRoutingFromCatalog } from "@/app/lib/requests-routing-config";
import { getTenantAppUrl, tenantAbsolutePath } from "@/app/lib/tenant-context";

export type RequestStatus = "NOUVELLE" | "EN_COURS" | "EN_ATTENTE" | "TERMINEE";
export type RequestAttachment = {
  id: string;
  key: string;
  fileName: string;
  contentType: string;
  size: number;
  uploadedAt: string;
};
export const MAX_REQUEST_ATTACHMENT_BYTES = 12 * 1024 * 1024;
export const MAX_REQUEST_ATTACHMENTS_PER_UPLOAD = 12;
const ATTACHMENT_EXT_OK = /\.(pdf|png|jpe?g|gif|webp|heic|doc|docx|xls|xlsx)$/i;
const ATTACHMENT_MIME_OK = new Set([
  "image/jpeg",
  "image/png",
  "image/gif",
  "image/webp",
  "image/heic",
  "application/pdf",
  "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/vnd.ms-excel",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
]);

export function assertEligibleRequestAttachment(
  fileName: string,
  contentType: string,
  size: number,
): { ok: true } | { ok: false; error: string } {
  if (size > MAX_REQUEST_ATTACHMENT_BYTES) {  return { ok: false, error: `Chaque fichier doit faire au plus ${MAX_REQUEST_ATTACHMENT_BYTES / 1024 / 1024} Mo.` }}
  if (size <= 0) return { ok: false, error: "Fichier vide." };
  const mime = (contentType || "").toLowerCase().split(";")[0].trim();
  if (ATTACHMENT_MIME_OK.has(mime)) return { ok: true };
  if (mime === "" || mime === "application/octet-stream") {
    if (ATTACHMENT_EXT_OK.test(fileName)) return { ok: true };
  }
  return {
    ok: false,
    error: "Type non autorisé : images, PDF, Word (.doc, .docx) ou Excel (.xls, .xlsx).",
  };
}

export function sanitizeRequestFileName(name: string): string {
  const base = name.replace(/[/\\?*]/g, "_").replace(/[^\w.\- ()éàèùïöüÄÉÀÈçÇ]+/gi, "_").trim().slice(0, 180);
  return base || "fichier";
}

export async function uploadBuffersAsRequestAttachments(
  requestId: string,
  items: { buffer: Buffer; fileName: string; contentType: string }[],
): Promise<RequestAttachment[]> {
  if (items.length > MAX_REQUEST_ATTACHMENTS_PER_UPLOAD) { throw new Error(`Trop de fichiers (max ${MAX_REQUEST_ATTACHMENTS_PER_UPLOAD}).`)}
  const out: RequestAttachment[] = [];
  const now = new Date().toISOString();
  for (const item of items) {
    const check = assertEligibleRequestAttachment(item.fileName, item.contentType, item.buffer.length);
    if (!check.ok) throw new Error(check.error);
    const attId = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const safe = sanitizeRequestFileName(item.fileName);
    const rel = `requests/${requestId}/files/${attId}_${safe}`;
    const key = await putObject( rel, item.buffer, item.contentType || "application/octet-stream");
    out.push({
      id: attId,
      key,
      fileName: item.fileName,
      contentType: item.contentType || "application/octet-stream",
      size: item.buffer.length,
      uploadedAt: now,
    });
  }
  return out;
}

export type RequestComment = {
  id: string;
  at: string;
  by: string;
  byEmail?: string;
  toRequester: boolean;
  content: string;
  attachments?: RequestAttachment[];
};

export type RequestHistoryItem = {
  at: string;
  by: string;
  action: string;
  note?: string;
};

/** Contexte dépôt parent (page publique). */
export type RequestParentContext = {
  source: "parent_portal";
  matched: boolean;
  children: Array<{
    ine: string;
    nom: string;
    prenom: string;
    classe?: string;
  }>;
};

export type RequestRecord = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: RequestStatus;
  category: string;
  subject: string;
  description: string;
  requester: {
    firstName: string;
    lastName: string;
    fullName: string;
    email: string;
    phone: string;
    userId?: string | null;
  };
  assignedTo: {
    routeId?: string;
    unit: string;
    roleLabel: string;
    email: string;
    ccEmails?: string[];
    poolEmails?: string[];
    claimedBy?: {
      email: string;
      name: string;
      userId: string | null;
      at: string;
    } | null;
  };
  routing: {
    source: "ai" | "fallback";
    confidence: number;
    reason: string;
    suggestedRouteId?: string;
    assignmentId?: string;
    taskId?: string;
    directionHint?: {
      suggestedQueueId: string;
      label: string;
      confidence: number;
      reason: string;
    };
  };
  parentContext?: RequestParentContext;
  attachments?: RequestAttachment[];
  comments: RequestComment[];
  history: RequestHistoryItem[];
  purgeAt?: string | null;
};

export function findRequestAttachment(record: RequestRecord, attachmentId: string): RequestAttachment | null {
  for (const a of record.attachments ?? []) {
    if (a.id === attachmentId) return a;
  }
  for (const c of record.comments) {
    for (const a of c.attachments ?? []) {
      if (a.id === attachmentId) return a;
    }
  }
  return null;
}

export type RequestCreateInput = {
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  subject: string;
  description: string;
  userId?: string | null;
};

const INDEX_KEY = "requests/index.json";
const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";
export const REQUEST_STATUSES: RequestStatus[] = ["NOUVELLE", "EN_COURS", "EN_ATTENTE", "TERMINEE"];
export const REQUEST_TERMINATED_RETENTION_DAYS = 30;
export function computePurgeAtForTerminated(fromIso: string): string {
  const d = new Date(fromIso);
  d.setUTCDate(d.getUTCDate() + REQUEST_TERMINATED_RETENTION_DAYS);
  return d.toISOString();
}
export function finalizeRequestPurgeMetadata(prev: RequestRecord, next: RequestRecord, nowIso: string): RequestRecord {
  if (next.status === "TERMINEE") {
    if (prev.status !== "TERMINEE") { return { ...next, purgeAt: computePurgeAtForTerminated(nowIso) }}
    return next;
  }
  return { ...next, purgeAt: null };
}

export function requestShouldBePurged(record: RequestRecord, now = new Date()): boolean {
  if (record.status !== "TERMINEE") return false;
  if (record.purgeAt) return now >= new Date(record.purgeAt);
  const legacy = new Date(record.updatedAt);
  legacy.setUTCDate(legacy.getUTCDate() + REQUEST_TERMINATED_RETENTION_DAYS);
  return now >= legacy;
}

export async function purgeExpiredRequests(): Promise<{ removed: number }> {
  const index = await getRequestsIndex();
  const now = new Date();
  const keep: RequestRecord[] = [];
  const remove: RequestRecord[] = [];
  for (const r of index) {
    if (requestShouldBePurged(r, now)) remove.push(r);
    else keep.push(r);
  }
  if (remove.length === 0) return { removed: 0 };
  const s3Client = await getS3Client();
  const bucket = await getBucketName();
  for (const r of remove) {
    try {
      await s3Client.send(
        new DeleteObjectCommand({
          Bucket: bucket,
          Key: s3Key( `requests/${r.id}.json`),
        }),
      );
    } catch (e) {
      console.error(`purge request file ${r.id}:`, e);
    }
    try {
      let token: string | undefined;
      const prefix = s3Key( `requests/${r.id}/`);
      do {
        const list = await s3Client.send( new ListObjectsV2Command({ Bucket: bucket, Prefix: prefix, ContinuationToken: token }));
        const keys = (list.Contents ?? []).map((c) => c.Key).filter(Boolean) as string[];
        if (keys.length > 0) {
          await s3Client.send(
            new DeleteObjectsCommand({
              Bucket: bucket,
              Delete: { Objects: keys.map((Key) => ({ Key })) },
            }),
          );
        }
        token = list.IsTruncated ? list.NextContinuationToken : undefined;
      } while (token);
    } catch (e) {
      console.error(`purge request folder ${r.id}:`, e);
    }
  }
  await saveRequestsIndex(keep);
  return { removed: remove.length };
}

function compact(value: string) { return value.trim().replace(/\s+/g, " ")}

export function deriveRequestSubject(description: string): string {
  const line = compact(description).split(/\n/)[0] || compact(description);
  if (line.length <= 80) return line;
  return `${line.slice(0, 77)}…`;
}

export function validateRequestInput(input: Partial<RequestCreateInput>) {
  const firstName = compact(String(input.firstName || ""));
  const lastName = compact(String(input.lastName || ""));
  const email = compact(String(input.email || "")).toLowerCase();
  let phone = compact(String(input.phone || ""));
  let subject = compact(String(input.subject || ""));
  const description = compact(String(input.description || ""));
  const userId = input.userId ?? null;
  if (!firstName || !lastName || !email || !description) {
    return { ok: false as const, error: "Prénom, nom, e-mail et description sont obligatoires." };
  }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) { return { ok: false as const, error: "Email invalide." }}
  if (!userId && phone.length < 8) { return { ok: false as const, error: "Téléphone invalide." }}
  if (userId && !phone) phone = "Non renseigné";
  if (!subject) subject = deriveRequestSubject(description);
  if (!subject) { return { ok: false as const, error: "Merci de détailler votre demande." }}
  if (description.length < 15) { return { ok: false as const, error: "Merci de détailler un peu plus votre demande." }}
  return {
    ok: true as const,
    value: { firstName, lastName, email, phone, subject, description, userId },
  };
}

/** Validation dépôt page parents — téléphone optionnel. */
export function validateParentPortalInput(input: {
  fullName?: string;
  email?: string;
  phone?: string;
  description?: string;
}) {
  const fullName = compact(String(input.fullName || ""));
  const email = compact(String(input.email || "")).toLowerCase();
  const phone = compact(String(input.phone || ""));
  const description = compact(String(input.description || ""));
  if (!fullName || !email || !description) {
    return { ok: false as const, error: "Nom, e-mail et description sont obligatoires." };
  }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    return { ok: false as const, error: "Email invalide." };
  }
  if (phone && phone.length < 8) {
    return { ok: false as const, error: "Téléphone invalide." };
  }
  if (description.length < 15) {
    return { ok: false as const, error: "Merci de détailler un peu plus votre demande." };
  }
  const parts = fullName.split(/\s+/).filter(Boolean);
  const firstName = parts[0] || fullName;
  const lastName = parts.slice(1).join(" ") || "—";
  const subject = deriveRequestSubject(description);
  return {
    ok: true as const,
    value: {
      firstName,
      lastName,
      email,
      phone: phone || "Non renseigné",
      subject,
      description,
      fullName,
    },
  };
}

export async function getRequestsIndex(): Promise<RequestRecord[]> {
  const hit = await getJson<RequestRecord[]>( INDEX_KEY);
  return hit?.data ?? [];
}

export async function saveRequestsIndex(index: RequestRecord[]) {
  await putJson(INDEX_KEY, index);
}

export async function saveRequestFile(record: RequestRecord) {
  await putJson(`requests/${record.id}.json`, record);
}

function normalizeForMatch(input: string) {
  return input.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "");
}

const ROUTING_CONFIDENCE_MIN = 0.52;

export type { RequestRouteDef } from "@/app/lib/requests-types";

/** Jamais assignées automatiquement — réservées au transfert manuel admin. */
export const MANUAL_ONLY_DIRECTION_IDS = new Set(["direction_ecole", "direction_college", "direction_lycee"]);

const DIRECTION_TO_ADMIN_QUEUE: Record<string, string> = {
  direction_ecole: "admin_ecole",
  direction_college: "admin_college",
  direction_lycee: "admin_lycee",
};

async function applyDirectionGate(routing: ResolvedRequestRouting): Promise<ResolvedRequestRouting> {
  const assignedRoute = routing.assignedTo.routeId ?? routing.assignedTo.unit;
  const hintId = MANUAL_ONLY_DIRECTION_IDS.has(assignedRoute)
    ? assignedRoute
    : routing.suggestedRouteId && MANUAL_ONLY_DIRECTION_IDS.has(routing.suggestedRouteId)
      ? routing.suggestedRouteId
      : null;
  if (!hintId) return routing;

  const directionDef = await getRouteById(hintId);
  const directionHint = {
    suggestedQueueId: hintId,
    label: directionDef?.roleLabel ?? hintId,
    confidence: routing.confidence,
    reason: routing.reason,
  };

  if (!MANUAL_ONLY_DIRECTION_IDS.has(assignedRoute)) {
    return { ...routing, directionHint };
  }

  const adminId = DIRECTION_TO_ADMIN_QUEUE[hintId];
  const adminDef = adminId ? await getRouteById(adminId) : null;
  if (!adminDef) return { ...routing, directionHint };
  return {
    category: adminDef.category,
    assignedTo: materializeAssigned(adminDef),
    source: routing.source,
    confidence: routing.confidence,
    reason: `${routing.reason} — déposée en administratif ; indicateur direction.`,
    directionHint,
    suggestedRouteId: routing.suggestedRouteId,
  };
}

export async function getAllBranchStaffEmails(): Promise<string[]> {
  const { routes } = await ensureRequestRoutes();
  const s = new Set<string>();
  for (const r of routes) {
    if (r.id === "corbeille") continue;
    for (const e of r.poolEmails()) {
      s.add(normalizeRequestEmail(e));
    }
  }
  return [...s];
}

function materializeAssigned(def: RequestRouteDef): RequestRecord["assignedTo"] {
  if (def.id === "corbeille") {
    const email = normalizeRequestEmail(def.primaryEmail());
    return {
      routeId: "corbeille",
      unit: "corbeille",
      roleLabel: def.roleLabel,
      email,
      claimedBy: null,
    };
  }
  const effectivePool = [...new Set(def.poolEmails().map(normalizeRequestEmail).filter(Boolean))];
  const email = effectivePool[0]!;
  const poolEmails = effectivePool.length > 1 ? effectivePool : undefined;
  return {
    routeId: def.id,
    unit: def.id,
    roleLabel: def.roleLabel,
    email,
    claimedBy: null,
    ...(poolEmails ? { poolEmails } : {}),
  };
}

export async function listRequestRoutesForPicker(): Promise<Array<{ id: string; label: string; category: string }>> {
  const { routes } = await ensureRequestRoutes();
  return routes.filter((r) => r.id !== "corbeille").map((r) => ({
    id: r.id,
    label: r.roleLabel,
    category: r.category,
  }));
}

export async function listRequestRoutesForTransmit(): Promise<Array<{ id: string; label: string; category: string }>> {
  const { routes } = await ensureRequestRoutes();
  return routes.filter((r) => r.id !== "corbeille" && MANUAL_ONLY_DIRECTION_IDS.has(r.id)).map((r) => ({
    id: r.id,
    label: r.roleLabel,
    category: r.category,
  }));
}

export async function isLeaderForRequestBranch(
  routeId: string | undefined,
  unit: string | undefined,
  actorEmail: string,
): Promise<boolean> {
  if (!actorEmail) return false;
  const b = normalizeRequestBranchId(routeId, unit);
  const def = await getRouteById(b);
  if (!def) return false;
  const u = normalizeRequestEmail(actorEmail);
  return def.leaderEmails().map(normalizeRequestEmail).includes(u);
}

export async function getDefaultRequestBranchForStaffEmail(actorEmail: string): Promise<string | null> {
  return getFirstBranchForStaffEmailFromDirectory(actorEmail);
}

export async function getRequestPoolEmails(record: RequestRecord): Promise<string[]> {
  const branch = normalizeRequestBranchId(record.assignedTo.routeId, record.assignedTo.unit);
  if (isCorbeilleBranchId(branch)) {
    return getAllBranchStaffEmails();
  }
  const a = record.assignedTo;
  if (a.poolEmails && a.poolEmails.length > 0) {
    return [...new Set(a.poolEmails.map(normalizeRequestEmail).filter(Boolean))];
  }
  return [normalizeRequestEmail(a.email)];
}

export async function getDelegateTargetEmailsForRequest(
  record: RequestRecord,
  leaderEmail: string,
): Promise<string[]> {
  const u = normalizeRequestEmail(leaderEmail);
  return (await getRequestPoolEmails(record)).map(normalizeRequestEmail).filter((e) => e && e !== u).sort();
}

export async function isUserInRequestPool(record: RequestRecord, userEmail: string) {
  return (await getRequestPoolEmails(record)).includes(normalizeRequestEmail(userEmail));
}

export function isSharedRequestPool(record: RequestRecord): boolean {
  return (record.assignedTo.poolEmails?.length ?? 0) > 1;
}

export async function isVisibleInMyQueue(record: RequestRecord, userEmail: string) {
  if (!userEmail) return false;
  const u = normalizeRequestEmail(userEmail);
  const c = record.assignedTo.claimedBy;
  if (c?.email && normalizeRequestEmail(c.email) === u) return true;
  if (isSharedRequestPool(record)) return false;
  if (!(await isUserInRequestPool(record, userEmail))) return false;
  if (!c?.email) return true;
  return normalizeRequestEmail(c.email) === u;
}

async function staffMailTargets(record: RequestRecord): Promise<{ to: string; cc: string | undefined }> {
  const branch = normalizeRequestBranchId(record.assignedTo.routeId, record.assignedTo.unit);
  if (isCorbeilleBranchId(branch)) {
    const def = await getRouteById("corbeille");
    const to = (def?.leaderEmails() ?? [record.assignedTo.email]).map(normalizeRequestEmail).filter(Boolean).join(", ");
    return { to, cc: undefined };
  }
  const pool = await getRequestPoolEmails(record);
  const to = pool.join(", ");
  const ccParts = (record.assignedTo.ccEmails || []).map(normalizeRequestEmail).filter((e) => e && !pool.map(normalizeRequestEmail).includes(e));
  return { to, cc: ccParts.length > 0 ? ccParts.join(", ") : undefined };
}

async function computeFallbackRouting(subject: string, description: string) {
  const { routes, map } = await ensureRequestRoutes();
  const text = normalizeForMatch(`${subject} ${description}`);
  const candidates = routes.filter((r) => r.id !== "corbeille");
  const corb = map.get("corbeille");
  if (!corb) {
    return {
      category: "Établissement",
      assignedTo: {
        routeId: "corbeille",
        unit: "corbeille",
        roleLabel: "Corbeille",
        email: "",
        claimedBy: null,
      },
      source: "fallback" as const,
      confidence: 0.32,
      reason: "Aucune route configurée.",
    };
  }
  let best = corb;
  let bestScore = -1;
  for (const rule of candidates) {
    const score = rule.keywords.reduce((acc, kw) => (text.includes(normalizeForMatch(kw)) ? acc + 1 : acc), 0);
    if (score > bestScore) {
      best = rule;
      bestScore = score;
    }
  }
  const chosen = bestScore > 0 ? best : corb;
  return {
    category: chosen.category,
    assignedTo: materializeAssigned(chosen),
    source: "fallback" as const,
    confidence: bestScore > 0 ? Math.min(0.75, 0.45 + bestScore * 0.08) : 0.32,
    reason: bestScore > 0 ? `Routage par mots-clés (score ${bestScore}) vers ${chosen.id}.` : "Aucun mot-clé fort : file de tri.",
  };
}

type MistralRouteResult = {
  category: string;
  assignedTo: RequestRecord["assignedTo"];
  source: "ai";
  confidence: number;
  reason: string;
  suggestedRouteId?: string;
};

async function routeWithMistral(subject: string, description: string): Promise<MistralRouteResult | null> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) return null;
  const { routes, map } = await ensureRequestRoutes();
  const routeList = routes.map((r) => `- ${r.id}: ${r.promptLine}`).join("\n");
  const prompt = `Tu es un classificateur pour un établissement scolaire. Choisis UNE SEULE routeId parmi la liste (identifiant exact).
Routes possibles:
${routeList}
Réponds uniquement en JSON:
{
  "routeId": "l'identifiant exact",
  "confidence": 0.0,
  "reason": "une courte phrase en français"
}
Règles (identifiants exacts):
- papier, toner, lampe, fuite, salle, bricolage, PC, mot de passe, Wi‑Fi, imprimante => maintenance
- facture, paiement, compta => comptabilite
- absence, justificatif, appel, infirmerie => vie_scolaire_infirmerie
- discipline CPE collège 5e/6e => cpe_5e6e ; 3e/4e => cpe_3e4e ; lycée => cpe_lycee
- secrétariat / bulletins école => admin_ecole ; collège => admin_college ; lycée => admin_lycee
- recours direction, plainte grave, décision directionnelle => direction_ecole / direction_college / direction_lycee (selon le pôle) — ces routes ne reçoivent jamais la demande directement, elles servent d'indicateur
- accueil, photocopieur panne côté accueil => accueil
- inscription / réinscription globale => admin_ecole ou admin_lycee selon le texte ; doute => corbeille
- si doute ou texte trop vague => corbeille avec confidence <= 0.4
Sujet: ${subject}
Demande: ${description}`;
  const res = await fetch(MISTRAL_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${mistralKey}`,
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      temperature: 0,
      response_format: { type: "json_object" },
      messages: [{ role: "user", content: prompt }],
    }),
  });
  if (!res.ok) return null;
  const data = await res.json();
  try {
    const parsed = JSON.parse(data?.choices?.[0]?.message?.content ?? "{}") as {
      routeId?: string;
      reason?: string;
      confidence?: number;
    };
    const rawId = typeof parsed.routeId === "string" ? parsed.routeId.trim() : "";
    const routeId = LEGACY_ROUTE_TO_BRANCH[rawId] ?? rawId;
    const def = map.get(routeId);
    if (!def) return null;
    const confidence = typeof parsed.confidence === "number" ? Math.max(0, Math.min(1, parsed.confidence)) : 0.65;
    const corb = map.get("corbeille");
    if (!corb) return null;
    let chosen = def;
    let suggestedRouteId: string | undefined;
    let reason = parsed.reason || "Routage IA Mistral.";
    if (confidence < ROUTING_CONFIDENCE_MIN && def.id !== "corbeille") {
      suggestedRouteId = def.id;
      chosen = corb;
      reason = `Confiance ${Math.round(confidence * 100)}% < seuil : corbeille. Hypothèse IA : ${suggestedRouteId}. ${parsed.reason || ""}`.trim();
    }
    return {
      category: chosen.category,
      assignedTo: materializeAssigned(chosen),
      source: "ai",
      confidence,
      reason,
      ...(suggestedRouteId ? { suggestedRouteId } : {}),
    };
  } catch {
    return null;
  }
}

export type ResolvedRequestRouting = {
  category: string;
  assignedTo: RequestRecord["assignedTo"];
  source: "ai" | "fallback";
  confidence: number;
  reason: string;
  suggestedRouteId?: string;
  directionHint?: RequestRecord["routing"]["directionHint"];
  routingMeta?: { assignmentId: string; taskId: string };
};

export async function resolveRequestRouting(subject: string, description: string): Promise<ResolvedRequestRouting> {
  const base = await resolveRoutingFromCatalog(subject, description);
  const gated = await applyDirectionGate(base);
  if (base.routingMeta) {
    return {
      ...gated,
      routingMeta: base.routingMeta,
    };
  }
  return gated;
}

export async function resolveRequestRouteById(routeId: string): Promise<ResolvedRequestRouting | null> {
  const raw = routeId.trim();
  const canonical = LEGACY_ROUTE_TO_BRANCH[raw] ?? raw;
  const def = await getRouteById(canonical);
  if (!def) return null;
  return {
    category: def.category,
    assignedTo: materializeAssigned(def),
    source: "fallback",
    confidence: 1,
    reason: `Réassignation manuelle vers ${canonical}.`,
  };
}

async function getMailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

export async function getPublicAppBaseUrl(): Promise<string> {
  const fromTenant = await getTenantAppUrl();
  if (fromTenant) return fromTenant;
  const v = process.env.VERCEL_URL?.trim();
  if (v) return `https://${v.replace(/^https?:\/\//, "")}`;
  return "";
}

export async function notifyRequestPendingVerification(
  email: string,
  firstName: string,
  confirmUrl: string,
): Promise<void> {
  const mail = await getMailer();
  if (!mail) throw new Error("SMTP non configuré");
  const { smtp, transporter } = mail;
  await transporter.sendMail({
    from: `"Demandes La Providence" <${smtp.user}>`,
    to: email,
    subject: "Confirmez votre demande — un clic sur le lien",
    text: [
      firstName ? `Bonjour ${firstName},` : "Bonjour,",
      "",
      "Merci de votre demande. Pour la transmettre à l'équipe, nous devons valider votre adresse e-mail.",
      "",
      "Ouvrez ce lien dans votre navigateur (une seule fois) :",
      confirmUrl,
      "",
      "Le lien est valable environ 72 heures. Si vous n'êtes pas à l'origine de ce message, vous pouvez l'ignorer.",
    ].join("\n"),
  });
}

export async function notifyRequestCreated(record: RequestRecord) {
  const mail = await getMailer();
  if (!mail) return;
  const { smtp, transporter } = mail;
  const { to, cc } = await staffMailTargets(record);
  const requestsLink = await tenantAbsolutePath("/requests");
  await transporter.sendMail({
    from: `"Demandes" <${smtp.user}>`,
    to,
    ...(cc ? { cc } : {}),
    subject: `Nouvelle demande (${record.category}) - ${record.requester.fullName}`,
    text: [
      `Une nouvelle demande a été créée.`,
      `ID: ${record.id}`,
      `Route: ${record.assignedTo.routeId ?? record.assignedTo.unit}`,
      record.routing.suggestedRouteId ? `Hypothèse IA (file de tri): ${record.routing.suggestedRouteId}` : "",
      `Demandeur: ${record.requester.fullName} (${record.requester.email}, ${record.requester.phone})`,
      `Sujet: ${record.subject}`,
      `Description: ${record.description}`,
      `Routage: ${record.assignedTo.roleLabel} (${record.routing.source}, confiance ${Math.round(record.routing.confidence * 100)}%)`,
      record.assignedTo.poolEmails?.length
        ? `File partagée: ${record.assignedTo.poolEmails.join(", ")} — premier qui prend la main la retire de la file des autres.`
        : "",
      `Motif: ${record.routing.reason}`,
      record.attachments?.length
        ? `Pièces jointes (${record.attachments.length}): ${record.attachments.map((a) => a.fileName).join(", ")}`
        : "",
      `Tableau des demandes: ${requestsLink}`,
    ].filter(Boolean).join("\n"),
  });
  await transporter.sendMail({
    from: `"Demandes" <${smtp.user}>`,
    to: record.requester.email,
    subject: `Votre demande a été enregistrée (${record.id})`,
    text: [
      `Bonjour ${record.requester.fullName},`,
      `Votre demande a bien été enregistrée.`,
      `ID: ${record.id}`,
      `Statut initial: ${record.status}`,
      `Service destinataire: ${record.assignedTo.roleLabel}`,
      `Sujet: ${record.subject}`,
      record.attachments?.length
        ? `Pièces jointes: ${record.attachments.length} fichier(s) — consultables par l’équipe sur la fiche demande.`
        : "",
      `Vous serez informé aux étapes importantes (mise en attente, clôture) ou si l’équipe vous répond directement.`,
    ].join("\n"),
  });
}

const NOTIFY_STATUSES: RequestStatus[] = ["EN_COURS", "EN_ATTENTE", "TERMINEE"];

export async function notifyRequestStatusMilestone(
  record: RequestRecord,
  previousStatus: RequestStatus,
  extraNote?: string,
  closureAttachments?: RequestAttachment[],
) {
  const mail = await getMailer();
  if (!mail) return;
  const now = record.status;
  if (now === previousStatus) return;
  if (!NOTIFY_STATUSES.includes(now)) return;
  const { smtp, transporter } = mail;
  const statusLabel =
    now === "EN_COURS" ? "prise en charge" : now === "TERMINEE" ? "clôture" : "en attente";
  const base = [ `Demande : ${record.id}`,`Évolution : ${previousStatus.replace("_", " ")} → ${now.replace("_", " ")}`,`Sujet : ${record.subject}`,extraNote ? `Précision : ${extraNote}` : ""].filter(Boolean).join("\n");
  const claimer = record.assignedTo.claimedBy?.name || record.assignedTo.claimedBy?.email;

  const mailAttachments: { filename: string; content: Buffer; contentType: string }[] = [];
  if (now === "TERMINEE" && closureAttachments?.length) {
    for (const att of closureAttachments) {
      const buf = await getObjectBytes(att.key);
      if (!buf?.length) continue;
      mailAttachments.push({
        filename: att.fileName,
        content: buf,
        contentType: att.contentType || "application/octet-stream",
      });
    }
  }

  await transporter.sendMail({
    from: `"Demandes" <${smtp.user}>`,
    to: record.requester.email,
    subject: `Votre demande — ${statusLabel} (${record.id})`,
    text: [
      `Bonjour ${record.requester.fullName},`,
      "",
      now === "EN_COURS" && claimer
        ? `Votre demande est prise en charge par ${claimer}.`
        : `L'équipe vous informe à l'occasion de cette étape.`,
      "",
      base,
      mailAttachments.length > 0
        ? `\nPièces jointes : ${mailAttachments.map((a) => a.filename).join(", ")}`
        : "",
    ].join("\n"),
    ...(mailAttachments.length > 0 ? { attachments: mailAttachments } : {}),
  });
  const { to: staffTo, cc: staffCc } = await staffMailTargets(record);
  await transporter.sendMail({
    from: `"Demandes" <${smtp.user}>`,
    to: staffTo,
    ...(staffCc ? { cc: staffCc } : {}),
    subject: `[Demande ${record.id}] ${now.replace("_", " ")}`,
    text: base,
    ...(mailAttachments.length > 0 ? { attachments: mailAttachments } : {}),
  });
}

export async function notifyRequesterOnly(record: RequestRecord, note: string) {
  const mail = await getMailer();
  if (!mail) return;
  if (!note.trim()) return;
  const { smtp, transporter } = mail;
  await transporter.sendMail({
    from: `"Demandes" <${smtp.user}>`,
    to: record.requester.email,
    subject: `Message concernant votre demande (${record.id})`,
    text: [
      `Bonjour ${record.requester.fullName},`,
      "",
      `Demande : ${record.id} — ${record.subject}`,
      "",
      note.trim(),
    ].join("\n"),
  });
}