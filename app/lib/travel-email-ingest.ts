import { createHash } from "crypto";
import { S3Client, GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { publicS3UrlForKey } from "@/app/lib/travels-s3";
import { getDataS3ClientForTenantSlug, getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName } from "@/app/lib/tenant-config";
import { getTenant } from "@/app/lib/tenant-context";
import { resolveTenantBySlug } from "@/app/lib/tenant-registry";
import { providerNameFromEmail } from "@/app/lib/transport-providers";
import { ocrPdfBytes, type TripCandidateForMatch } from "@/app/lib/travel-devis-ocr";
import {
  analyzeTravelEmailWithMistral,
  type TransportEmailMessage,
  type TravelEmailAnalysis,
} from "@/app/lib/travel-email-intelligence";
import { sendTravelEmailUnmatchedAlert } from "@/app/lib/travel-email-alert";
import { resolveTenantSlugWithMistral } from "@/app/lib/travel-tenant-match";

const INCOMING_PREFIX = "devis-incoming/";
const UNMATCHED_EMAIL_KEY = "travels/email-devis-unmatched.json";
const INDEX_KEY = "travels/index.json";
const MAX_CANDIDATES = 80;
const MARKER_PREFIX = "travels/email-ingest-markers/";
const PENDING_STALE_MS = 15 * 60 * 1000;

const INACTIVE_STATUSES = new Set(["ANNULE", "REJETE", "SEANCE_ANNULEE"]);

/** Motifs = erreur temporaire : ne pas marquer le mail comme lu. */
const TRANSIENT_FAIL_REASONS = new Set([
  "missing_mistral_key",
  "erreur_http_mistral",
  "reponse_mistral_vide",
  "erreur_mistral_analyse",
  "erreur_traitement",
]);

type TravelEmailIngestInput = {
  s3Key?: string;
  fromEmail: string;
  subject?: string;
  snippet?: string;
  emailBodyPlain?: string;
  gmailMessageId: string;
  originalFilename?: string;
  /** Slug extrait du Reply-To / To (transport+slug@…). */
  tenantSlugHint?: string | null;
  /**
   * Le poller a déjà tenté le matching tenant IA sans succès :
   * ne pas rappeler Mistral, passer directement en tenant_introuvable_ia.
   */
  tenantUnresolved?: boolean;
};

export type TravelEmailIngestResult = {
  ok: boolean;
  completed: boolean;
  failed: boolean;
  matched: boolean;
  pending?: boolean;
  duplicate?: boolean;
  tripId?: string | null;
  reason?: string | null;
  devisId?: string | null;
  messageId?: string | null;
  detail?: string;
  /** true → le poller peut retirer UNREAD */
  safeToMarkRead: boolean;
};

type IngestMarker = {
  pending?: boolean;
  startedAt?: string;
  completed?: boolean;
  failed?: boolean;
  duplicate?: boolean;
  matched?: boolean;
  tripId?: string | null;
  reason?: string | null;
  devisId?: string | null;
  messageId?: string | null;
  gmailMessageId?: string;
  s3Key?: string;
  updatedAt?: string;
  alertSent?: boolean;
  tenantSlug?: string | null;
};

type IngestTarget = {
  client: S3Client;
  bucket: string;
  tenantSlug: string;
};

async function resolveIngestTarget(slugHint?: string | null): Promise<IngestTarget | null> {
  const hint = (slugHint || "").trim().toLowerCase();
  if (hint) {
    const tenant = await resolveTenantBySlug(hint);
    if (tenant) {
      return {
        client: await getDataS3ClientForTenantSlug(tenant.slug),
        bucket: tenant.dataBucket,
        tenantSlug: tenant.slug,
      };
    }
    return null;
  }
  return null;
}

/** Bucket pour markers / alerte quand aucun tenant n'a pu être résolu. */
async function resolveHoldingTarget(): Promise<IngestTarget> {
  try {
    const tenant = await getTenant();
    return {
      client: await getTenantDataS3Client(),
      bucket: tenant.dataBucket || (await getTenantBucketName()),
      tenantSlug: tenant.slug,
    };
  } catch {
    return {
      client: await getTenantDataS3Client(),
      bucket: await getTenantBucketName(),
      tenantSlug: "platform",
    };
  }
}

type UnmatchedItem = {
  id: string;
  s3Key?: string;
  fromEmail: string;
  subject: string;
  gmailMessageId: string;
  snippet?: string;
  originalFilename?: string;
  createdAt: string;
  extractedPrice?: string | null;
  extractedCompany?: string | null;
  guessedTripId?: string | null;
  matchMotif?: string | null;
  matchConfidence?: string | null;
  messageType?: string | null;
  reason?: string;
};

export function ingestSecretFromEnv(): string | undefined {
  const raw = process.env.TRAVEL_EMAIL_INGEST_SECRET?.trim() || process.env.INGEST_SECRET?.trim();
  return raw || undefined;
}

export function isAllowedIncomingKey(key: string): boolean {
  if (!key.startsWith(INCOMING_PREFIX) || key.includes("..")) return false;
  return key.length <= 2048;
}

function attachmentIdFromIncomingKey(s3Key: string): string | null {
  const m = s3Key.match(/^devis-incoming\/[^/]+\/([^_]+)_/);
  return m?.[1] ?? null;
}

function ingestMarkerKey(gmailMessageId: string, s3Key?: string): string {
  if (s3Key) {
    const attId = attachmentIdFromIncomingKey(s3Key);
    if (attId) return `${MARKER_PREFIX}${gmailMessageId}/${attId}.json`;
    const h = createHash("sha256").update(`${gmailMessageId}\0${s3Key}`, "utf8").digest("hex");
    return `${MARKER_PREFIX}${h}.json`;
  }
  return `${MARKER_PREFIX}msg/${gmailMessageId}.json`;
}

function resultFromMarker(m: IngestMarker, detail?: string): TravelEmailIngestResult {
  const reason = m.reason ?? null;
  const failed = Boolean(m.failed) || (reason != null && TRANSIENT_FAIL_REASONS.has(reason));
  const completed = Boolean(m.completed);
  return {
    ok: true,
    completed,
    failed,
    matched: Boolean(m.matched),
    duplicate: Boolean(m.duplicate),
    tripId: m.tripId ?? null,
    reason,
    devisId: m.devisId ?? null,
    messageId: m.messageId ?? null,
    detail,
    safeToMarkRead: completed && !failed,
  };
}

function devisAlreadyInTrip(
  received: unknown[],
  gmailMessageId: string,
  s3Key: string,
  originalFilename?: string,
): boolean {
  return received.some((d) => {
    if (!d || typeof d !== "object") return false;
    const row = d as {
      gmailMessageId?: string;
      s3KeyIncoming?: string;
      originalFilename?: string | null;
      source?: string;
    };
    if (row.gmailMessageId !== gmailMessageId) return false;
    if (row.s3KeyIncoming === s3Key) return true;
    if (row.source === "email" && originalFilename && row.originalFilename === originalFilename) return true;
    return false;
  });
}

function transportEmailAlreadyInTrip(
  messages: unknown[],
  gmailMessageId: string,
  s3Key?: string,
): boolean {
  return messages.some((m) => {
    if (!m || typeof m !== "object") return false;
    const row = m as { gmailMessageId?: string; s3KeyIncoming?: string };
    if (row.gmailMessageId !== gmailMessageId) return false;
    if (s3Key) return row.s3KeyIncoming === s3Key;
    return !row.s3KeyIncoming;
  });
}

async function readIngestMarker(client: S3Client, bucket: string, key: string): Promise<IngestMarker | null> {
  try {
    const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    return JSON.parse(raw) as IngestMarker;
  } catch {
    return null;
  }
}

async function writeIngestMarker(client: S3Client, bucket: string, key: string, data: IngestMarker) {
  await client.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: key,
      Body: JSON.stringify({ ...data, updatedAt: new Date().toISOString() }, null, 2),
      ContentType: "application/json",
    }),
  );
}

type IndexTrip = {
  id: string | number;
  status?: string;
  createdAt?: string;
  data?: {
    title?: string;
    destination?: string;
    etablissement?: string;
    date?: string | null;
    startDate?: string | null;
    endDate?: string | null;
    startTime?: string | null;
    endTime?: string | null;
    needsBus?: boolean;
    transportRequest?: Record<string, unknown>;
    classes?: string | string[];
    nbEleves?: number | string;
  };
};

function transportContextSnippet(tr: Record<string, unknown> | undefined): string | undefined {
  if (!tr) return undefined;
  const parts: string[] = [];
  for (const key of ["departure", "arrival", "aller", "retour", "from", "to", "lieuDepart", "lieuArrivee", "freeText"]) {
    const v = tr[key];
    if (typeof v === "string" && v.trim()) parts.push(v.trim().slice(0, 200));
  }
  return parts.length ? parts.join(" · ").slice(0, 400) : undefined;
}

async function loadTripCandidates(client: S3Client, bucket: string): Promise<TripCandidateForMatch[]> {
  try {
    const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: INDEX_KEY }));
    const raw = await res.Body?.transformToString();
    if (!raw) return [];
    const all = JSON.parse(raw) as IndexTrip[];
    if (!Array.isArray(all)) return [];
    const sorted = [...all].sort(
      (a, b) => new Date(b.createdAt || 0).getTime() - new Date(a.createdAt || 0).getTime(),
    );
    const active = sorted.filter((t) => !INACTIVE_STATUSES.has(String(t.status || "")));
    const withTransport = active.filter((t) => Boolean(t.data?.needsBus || t.data?.transportRequest));
    const pool = withTransport.length > 0 ? withTransport : active;
    return pool.slice(0, MAX_CANDIDATES).map((t) => {
      const d = t.data || {};
      const classes = Array.isArray(d.classes) ? d.classes.join(", ") : String(d.classes || "");
      return {
        id: String(t.id),
        title: String(d.title || ""),
        destination: String(d.destination || ""),
        startDate: d.startDate || d.date || undefined,
        endDate: d.endDate || d.date || undefined,
        startTime: d.startTime || undefined,
        endTime: d.endTime || undefined,
        status: t.status || "",
        classes,
        etablissement: d.etablissement ? String(d.etablissement) : undefined,
        needsBus: Boolean(d.needsBus || d.transportRequest),
        nbEleves: d.nbEleves != null ? String(d.nbEleves) : undefined,
        transportContext: transportContextSnippet(
          d.transportRequest && typeof d.transportRequest === "object" ? d.transportRequest : undefined,
        ),
      };
    });
  } catch {
    return [];
  }
}

export async function listUnmatchedEmailItems(): Promise<UnmatchedItem[]> {
  const bucket = await getTenantBucketName();
  const client = await getTenantDataS3Client();
  try {
    const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: UNMATCHED_EMAIL_KEY }));
    const raw = await res.Body?.transformToString();
    if (!raw) return [];
    const parsed = JSON.parse(raw) as { items?: UnmatchedItem[] };
    return Array.isArray(parsed.items) ? parsed.items : [];
  } catch {
    return [];
  }
}

async function appendUnmatched(client: S3Client, bucket: string, item: UnmatchedItem) {
  let items: UnmatchedItem[] = [];
  try {
    const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: UNMATCHED_EMAIL_KEY }));
    const raw = await res.Body?.transformToString();
    if (raw) {
      const parsed = JSON.parse(raw) as { items?: UnmatchedItem[] };
      items = Array.isArray(parsed.items) ? parsed.items : [];
    }
  } catch {
    /* absent */
  }
  items.unshift(item);
  const capped = items.slice(0, 200);
  await client.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: UNMATCHED_EMAIL_KEY,
      Body: JSON.stringify({ items: capped }, null, 2),
      ContentType: "application/json",
    }),
  );
}

function buildEmailAttachmentLabel(
  analysis: TravelEmailAnalysis,
  originalFilename: string | undefined,
  providerName: string,
): string {
  const file = originalFilename || "document.pdf";
  if (analysis.messageType === "confirmation_commande") {
    const who = analysis.company || providerName;
    return who ? `Confirmation transport — ${who}` : `Confirmation transport — ${file}`;
  }
  if (analysis.messageType === "info_transport") {
    return `Info transport — ${file}`;
  }
  return `E-mail transport — ${file}`;
}

function attachmentAlreadyInTrip(attachments: unknown[], gmailMessageId: string, s3Key: string): boolean {
  return attachments.some((a) => {
    if (!a || typeof a !== "object") return false;
    const row = a as { gmailMessageId?: string; s3Key?: string };
    return row.s3Key === s3Key || (row.gmailMessageId === gmailMessageId && row.s3Key === s3Key);
  });
}

function appendEmailPdfToAttachments(
  tripFresh: Record<string, unknown>,
  input: {
    fileUrl: string;
    s3Key: string;
    gmailMessageId: string;
    label: string;
  },
): boolean {
  const data = (tripFresh.data && typeof tripFresh.data === "object" ? tripFresh.data : {}) as Record<
    string,
    unknown
  >;
  const attachments = Array.isArray(data.attachments) ? data.attachments : [];
  if (attachmentAlreadyInTrip(attachments, input.gmailMessageId, input.s3Key)) return false;
  data.attachments = [
    ...attachments,
    {
      name: input.label,
      url: input.fileUrl,
      s3Key: input.s3Key,
      source: "email",
      gmailMessageId: input.gmailMessageId,
    },
  ];
  tripFresh.data = data;
  return true;
}

function buildTransportEmailMessage(
  analysis: TravelEmailAnalysis,
  input: {
    gmailMessageId: string;
    fromEmail: string;
    subject: string;
    s3Key?: string;
    pdfUrl?: string;
    originalFilename?: string;
  },
  now: string,
): TransportEmailMessage {
  return {
    id: `${Date.now()}-${input.gmailMessageId.slice(-8)}`,
    gmailMessageId: input.gmailMessageId,
    fromEmail: input.fromEmail.trim(),
    subject: input.subject || "",
    messageType: analysis.messageType,
    summary: analysis.summary || input.subject || "Message transport",
    driverName: analysis.driverName,
    driverPhone: analysis.driverPhone,
    details: analysis.confirmationDetails,
    pdfUrl: input.pdfUrl ?? null,
    s3KeyIncoming: input.s3Key ?? null,
    originalFilename: input.originalFilename ?? null,
    receivedAt: now,
    matchConfidence: analysis.matchConfidence,
    matchMotif: analysis.matchMotif,
    source: "email",
  };
}

function applyTransportInfoToTripData(
  tripFresh: Record<string, unknown>,
  analysis: TravelEmailAnalysis,
  msg: TransportEmailMessage,
) {
  const data = (tripFresh.data && typeof tripFresh.data === "object" ? tripFresh.data : {}) as Record<
    string,
    unknown
  >;
  const existing = Array.isArray(data.transportEmailMessages) ? data.transportEmailMessages : [];
  data.transportEmailMessages = [msg, ...existing].slice(0, 50);

  if (analysis.messageType === "confirmation_commande") {
    data.transportProviderConfirmation = {
      receivedAt: msg.receivedAt,
      summary: msg.summary,
      fromEmail: msg.fromEmail,
      providerName: analysis.company,
      pdfUrl: msg.pdfUrl ?? null,
      s3KeyIncoming: msg.s3KeyIncoming ?? null,
      originalFilename: msg.originalFilename ?? null,
      gmailMessageId: msg.gmailMessageId,
    };
  }

  if (analysis.driverName || analysis.driverPhone || analysis.summary) {
    const tr = (data.transportRequest && typeof data.transportRequest === "object"
      ? data.transportRequest
      : {}) as Record<string, unknown>;
    const lines: string[] = [];
    if (analysis.driverName) lines.push(`Chauffeur : ${analysis.driverName}`);
    if (analysis.driverPhone) lines.push(`Tél. chauffeur : ${analysis.driverPhone}`);
    if (analysis.summary) lines.push(analysis.summary);
    const block = lines.join("\n");
    const phoneKey = analysis.driverPhone || "";
    const existingText = String(tr.freeText || "");
    if (phoneKey && !existingText.includes(phoneKey)) {
      tr.freeText = existingText
        ? `${existingText}\n\n--- Info transport (e-mail) ---\n${block}`
        : block;
      data.transportRequest = tr;
    } else if (!phoneKey && analysis.summary && !existingText.includes(analysis.summary.slice(0, 40))) {
      tr.freeText = existingText
        ? `${existingText}\n\n--- Info transport (e-mail) ---\n${block}`
        : block;
      data.transportRequest = tr;
    }
  }

  tripFresh.data = data;
  const history = Array.isArray(tripFresh.history) ? tripFresh.history : [];
  history.unshift({
    date: msg.receivedAt,
    user: "Système (e-mail)",
    action:
      analysis.messageType === "confirmation_commande"
        ? "Confirmation transport reçue"
        : analysis.messageType === "devis_pdf"
          ? "Devis reçu par e-mail"
          : "Message transport reçu par e-mail",
    note: analysis.summary || msg.subject,
  });
  tripFresh.history = history.slice(0, 200);
}

async function maybeAlertUnmatched(
  client: S3Client,
  bucket: string,
  markerKey: string,
  marker: IngestMarker,
  payload: {
    gmailMessageId: string;
    fromEmail: string;
    subject: string;
    snippet?: string;
    reason?: string | null;
    matchMotif?: string | null;
    messageType?: string | null;
    guessedTripId?: string | null;
    tenantSlug?: string | null;
  },
): Promise<IngestMarker> {
  if (marker.matched || marker.failed || marker.alertSent) return marker;
  if (!marker.reason || TRANSIENT_FAIL_REASONS.has(marker.reason)) return marker;

  const alert = await sendTravelEmailUnmatchedAlert({
    gmailMessageId: payload.gmailMessageId,
    fromEmail: payload.fromEmail,
    subject: payload.subject,
    snippet: payload.snippet,
    reason: payload.reason ?? marker.reason,
    matchMotif: payload.matchMotif,
    messageType: payload.messageType,
    guessedTripId: payload.guessedTripId,
    tenantSlug: payload.tenantSlug ?? marker.tenantSlug,
  });
  if (!alert.sent) return marker;
  const withAlert = { ...marker, alertSent: true };
  await writeIngestMarker(client, bucket, markerKey, withAlert);
  return withAlert;
}

async function runIngestJob(p: {
  client: S3Client;
  bucket: string;
  tenantSlug: string;
  markerKey: string;
  s3Key?: string;
  fromEmail: string;
  subject: string;
  snippet: string;
  emailBodyPlain?: string;
  gmailMessageId: string;
  originalFilename?: string;
  candidates: TripCandidateForMatch[];
  providerName: string;
  hasPdfAttachment: boolean;
}): Promise<IngestMarker> {
  const {
    client,
    bucket,
    tenantSlug,
    markerKey,
    s3Key,
    fromEmail,
    subject,
    snippet,
    emailBodyPlain,
    gmailMessageId,
    originalFilename,
    candidates,
    providerName,
    hasPdfAttachment,
  } = p;

  const finish = async (m: IngestMarker): Promise<IngestMarker> => {
    const final = { ...m, pending: false, completed: true, tenantSlug };
    await writeIngestMarker(client, bucket, markerKey, final);
    return final;
  };

  const finishUnmatched = async (
    item: UnmatchedItem,
    reason: string,
    tripId: string | null = null,
  ): Promise<IngestMarker> => {
    await appendUnmatched(client, bucket, item);
    let marker = await finish({ matched: false, reason, tripId });
    marker = await maybeAlertUnmatched(client, bucket, markerKey, marker, {
      gmailMessageId,
      fromEmail,
      subject,
      snippet,
      reason,
      matchMotif: item.matchMotif,
      messageType: item.messageType,
      guessedTripId: item.guessedTripId,
      tenantSlug,
    });
    return marker;
  };

  try {
    let ocrText = "";
    if (s3Key) {
      try {
        const obj = await client.send(new GetObjectCommand({ Bucket: bucket, Key: s3Key }));
        const bytes = await obj.Body?.transformToByteArray();
        if (bytes && bytes.length > 0) {
          ocrText = await ocrPdfBytes(Buffer.from(bytes));
        }
      } catch (e) {
        console.error("[travel-email-ingest] OCR:", e);
      }
    }

    const analysis = await analyzeTravelEmailWithMistral({
      subject: subject || "",
      snippet: snippet || "",
      bodyPlain: emailBodyPlain,
      ocrText,
      fromEmail,
      hasPdfAttachment,
      candidates,
    });

    if (analysis.matchMotif && TRANSIENT_FAIL_REASONS.has(analysis.matchMotif)) {
      return finish({
        failed: true,
        matched: false,
        reason: analysis.matchMotif,
        tripId: null,
        gmailMessageId,
        s3Key,
      });
    }

    const now = new Date().toISOString();
    const tripId = analysis.matchedTripId;

    const unmatchedBase = () => ({
      id: `${Date.now()}-${gmailMessageId.slice(-8)}`,
      s3Key,
      fromEmail,
      subject: subject || "",
      gmailMessageId,
      snippet,
      originalFilename,
      createdAt: now,
      extractedPrice: analysis.price,
      extractedCompany: analysis.company,
      guessedTripId: analysis.suggestedTripId,
      matchMotif: analysis.matchMotif,
      matchConfidence: analysis.matchConfidence,
      messageType: analysis.messageType,
    });

    if (!candidates.length) {
      return finishUnmatched({ ...unmatchedBase(), reason: "aucun_voyage_liste" }, "aucun_voyage_liste");
    }

    if (!tripId) {
      return finishUnmatched({ ...unmatchedBase(), reason: "pas_de_correspondance_ia" }, "pas_de_correspondance_ia");
    }

    const tripKey = `travels/${tripId}.json`;
    let tripFresh: Record<string, unknown>;
    try {
      const tripRes = await client.send(new GetObjectCommand({ Bucket: bucket, Key: tripKey }));
      const tripRaw = await tripRes.Body?.transformToString();
      tripFresh = tripRaw ? JSON.parse(tripRaw) : {};
    } catch {
      return finishUnmatched(
        { ...unmatchedBase(), guessedTripId: tripId, reason: "voyage_introuvable_s3" },
        "voyage_introuvable_s3",
        tripId,
      );
    }

    let devisId: string | null = null;
    let messageId: string | null = null;
    let attachmentAdded = false;
    let duplicate = false;
    const fileViewUrl = s3Key ? await publicS3UrlForKey(s3Key) : undefined;
    const isDevis = analysis.messageType === "devis_pdf";

    if (isDevis && hasPdfAttachment && s3Key) {
      const received = Array.isArray(tripFresh.receivedDevis) ? tripFresh.receivedDevis : [];
      if (devisAlreadyInTrip(received, gmailMessageId, s3Key, originalFilename)) {
        duplicate = true;
      } else {
        const newDevis = {
          id: Date.now().toString(),
          providerName: analysis.company || providerName,
          fileUrl: fileViewUrl,
          providerEmail: fromEmail.trim(),
          createdAt: now,
          source: "email",
          gmailMessageId,
          originalFilename: originalFilename ?? null,
          extractedPrice: analysis.price,
          extractedCompany: analysis.company,
          s3KeyIncoming: s3Key,
          matchMethod: "mistral_email_ia",
          matchConfidence: analysis.matchConfidence,
          matchMotif: analysis.matchMotif,
          matchReviewRequired: analysis.matchReviewRequired,
          extractedContactEmail: analysis.contactEmail,
        };
        devisId = newDevis.id;
        tripFresh.receivedDevis = [...received, newDevis];
      }
    }

    const data = (tripFresh.data && typeof tripFresh.data === "object" ? tripFresh.data : {}) as Record<
      string,
      unknown
    >;
    const transportMsgs = Array.isArray(data.transportEmailMessages) ? data.transportEmailMessages : [];
    const transportDuplicate = transportEmailAlreadyInTrip(transportMsgs, gmailMessageId, s3Key);

    const shouldStoreTransportMsg = !transportDuplicate && analysis.messageType !== "non_lie";

    if (shouldStoreTransportMsg) {
      const msg = buildTransportEmailMessage(
        analysis,
        {
          gmailMessageId,
          fromEmail,
          subject,
          s3Key,
          pdfUrl: fileViewUrl,
          originalFilename,
        },
        now,
      );
      messageId = msg.id;
      applyTransportInfoToTripData(tripFresh, analysis, msg);
    }

    if (hasPdfAttachment && s3Key && fileViewUrl && !isDevis && analysis.messageType !== "non_lie") {
      attachmentAdded = appendEmailPdfToAttachments(tripFresh, {
        fileUrl: fileViewUrl,
        s3Key,
        gmailMessageId,
        label: buildEmailAttachmentLabel(analysis, originalFilename, providerName),
      });
      if (attachmentAdded) {
        const history = Array.isArray(tripFresh.history) ? tripFresh.history : [];
        history.unshift({
          date: now,
          user: "Système (e-mail)",
          action: "Document ajouté au dossier",
          note: buildEmailAttachmentLabel(analysis, originalFilename, providerName),
        });
        tripFresh.history = history.slice(0, 200);
      }
    }

    if (devisId || messageId || attachmentAdded) {
      await client.send(
        new PutObjectCommand({
          Bucket: bucket,
          Key: tripKey,
          Body: JSON.stringify(tripFresh),
          ContentType: "application/json",
        }),
      );
      return finish({
        matched: true,
        tripId,
        devisId,
        messageId,
        duplicate: duplicate && !messageId,
      });
    }

    if (duplicate || transportDuplicate) {
      return finish({
        matched: true,
        tripId,
        duplicate: true,
        reason: "deja_enregistre",
      });
    }

    return finishUnmatched(
      { ...unmatchedBase(), reason: "type_message_non_traite" },
      "type_message_non_traite",
      tripId,
    );
  } catch (e) {
    console.error("[travel-email-ingest] job:", e);
    const failed: IngestMarker = {
      pending: false,
      completed: true,
      failed: true,
      reason: e instanceof Error ? e.message : "erreur_traitement",
      gmailMessageId,
      s3Key,
      tenantSlug,
    };
    await writeIngestMarker(client, bucket, markerKey, failed);
    return failed;
  }
}

/**
 * Ingestion synchrone d'un e-mail (corps ± PDF) : OCR + Mistral → rattachement séjour.
 * Utilisée par le poller in-app et par POST /api/travels/ingest-from-email.
 */
export async function ingestTravelEmail(input: TravelEmailIngestInput): Promise<TravelEmailIngestResult> {
  const {
    s3Key,
    fromEmail,
    subject,
    snippet,
    emailBodyPlain,
    gmailMessageId,
    originalFilename,
    tenantSlugHint,
    tenantUnresolved,
  } = input;
  if (!fromEmail?.trim() || !gmailMessageId?.trim()) {
    return {
      ok: false,
      completed: true,
      failed: true,
      matched: false,
      reason: "fromEmail_et_gmailMessageId_requis",
      safeToMarkRead: false,
    };
  }

  const hasPdf = Boolean(s3Key);
  const textContent = (emailBodyPlain || snippet || "").trim();
  if (!hasPdf && textContent.length < 8) {
    return {
      ok: false,
      completed: true,
      failed: true,
      matched: false,
      reason: "contenu_insuffisant",
      safeToMarkRead: false,
    };
  }
  if (s3Key && !isAllowedIncomingKey(s3Key)) {
    return {
      ok: false,
      completed: true,
      failed: true,
      matched: false,
      reason: "cle_s3_non_autorisee",
      safeToMarkRead: false,
    };
  }

  let resolvedSlug = (tenantSlugHint || "").trim().toLowerCase() || null;
  let tenantResolveMotif: string | null = null;

  if (!resolvedSlug && tenantUnresolved) {
    tenantResolveMotif = "tenant_introuvable_ia";
  } else if (!resolvedSlug) {
    const tenantMatch = await resolveTenantSlugWithMistral({
      subject: subject ?? "",
      bodyPlain: emailBodyPlain,
      snippet: snippet ?? "",
      fromEmail,
    });
    resolvedSlug = tenantMatch.slug;
    tenantResolveMotif = tenantMatch.motif;
  }

  if (!resolvedSlug) {
    const holding = await resolveHoldingTarget();
    const markerKey = ingestMarkerKey(gmailMessageId, s3Key);
    const existing = await readIngestMarker(holding.client, holding.bucket, markerKey);
    if (existing?.completed) return resultFromMarker(existing, "deja_traite");

    const reason = "tenant_introuvable_ia";
    let marker: IngestMarker = {
      pending: false,
      completed: true,
      matched: false,
      failed: false,
      reason,
      tripId: null,
      gmailMessageId,
      s3Key,
      tenantSlug: null,
    };
    await writeIngestMarker(holding.client, holding.bucket, markerKey, marker);
    await appendUnmatched(holding.client, holding.bucket, {
      id: `${Date.now()}-${gmailMessageId.slice(-8)}`,
      s3Key,
      fromEmail,
      subject: subject ?? "",
      gmailMessageId,
      snippet: snippet ?? "",
      originalFilename,
      createdAt: new Date().toISOString(),
      reason,
      matchMotif: tenantResolveMotif,
    });
    marker = await maybeAlertUnmatched(holding.client, holding.bucket, markerKey, marker, {
      gmailMessageId,
      fromEmail,
      subject: subject ?? "",
      snippet: snippet ?? "",
      reason,
      matchMotif: tenantResolveMotif,
      tenantSlug: null,
    });
    return resultFromMarker(marker, "tenant_non_resolu");
  }

  const target = await resolveIngestTarget(resolvedSlug);
  if (!target) {
    return {
      ok: false,
      completed: true,
      failed: true,
      matched: false,
      reason: "tenant_slug_inconnu",
      safeToMarkRead: false,
    };
  }

  const { client, bucket, tenantSlug } = target;
  const markerKey = ingestMarkerKey(gmailMessageId, s3Key);
  const existing = await readIngestMarker(client, bucket, markerKey);

  if (existing?.completed) {
    return resultFromMarker(existing, "deja_traite");
  }

  if (existing?.pending) {
    const age = existing.startedAt ? Date.now() - new Date(existing.startedAt).getTime() : 0;
    if (!Number.isNaN(age) && age < PENDING_STALE_MS) {
      return {
        ok: true,
        completed: false,
        failed: false,
        matched: false,
        pending: true,
        detail: "Traitement déjà en cours pour ce message.",
        safeToMarkRead: false,
      };
    }
  }

  await writeIngestMarker(client, bucket, markerKey, {
    pending: true,
    startedAt: new Date().toISOString(),
    gmailMessageId,
    s3Key,
    tenantSlug,
  });

  const candidates = await loadTripCandidates(client, bucket);
  const providerName = (await providerNameFromEmail(fromEmail)) ?? "Transporteur (e-mail)";

  const marker = await runIngestJob({
    client,
    bucket,
    tenantSlug,
    markerKey,
    s3Key,
    fromEmail,
    subject: subject ?? "",
    snippet: snippet ?? "",
    emailBodyPlain,
    gmailMessageId,
    originalFilename,
    candidates,
    providerName,
    hasPdfAttachment: hasPdf,
  });

  return resultFromMarker(marker);
}
