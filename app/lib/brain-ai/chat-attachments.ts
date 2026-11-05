/**
 * Pièces jointes du chat ScolIA : types acceptés, clés S3 propres à l'utilisateur,
 * lecture du contenu pour l'analyse par le modèle (images → vision, PDF → OCR Mistral,
 * texte → lecture directe). Partagé entre la route d'upload et run-chat.
 */

export const CHAT_MAX_ATTACHMENTS = 10;

/** Taille max acceptée à l'upload (octets). Les photos sont réduites côté navigateur. */
export const CHAT_MAX_FILE_BYTES = 20 * 1024 * 1024;
export const CHAT_MAX_IMAGE_BYTES = 6 * 1024 * 1024;

export const CHAT_IMAGE_TYPES = new Set(["image/jpeg", "image/png", "image/webp", "image/gif"]);
export const CHAT_TEXT_TYPES = new Set(["text/plain", "text/csv", "text/markdown"]);
export const CHAT_PDF_TYPE = "application/pdf";

export type ChatAttachmentKind = "image" | "pdf" | "text";

export function chatAttachmentKind(contentType: string): ChatAttachmentKind | null {
  const ct = contentType.trim().toLowerCase();
  if (ct === CHAT_PDF_TYPE) return "pdf";
  if (CHAT_IMAGE_TYPES.has(ct)) return "image";
  if (CHAT_TEXT_TYPES.has(ct)) return "text";
  return null;
}

/** Dossier S3 des dépôts d'un utilisateur — sert aussi de contrôle d'appartenance. */
export function chatUploadPrefix(userId: string): string {
  const safe = String(userId || "").replace(/[^a-zA-Z0-9_-]/g, "_").slice(0, 80) || "anonyme";
  return `brain-ai/uploads/${safe}`;
}

export type ChatAttachmentRef = { key: string; fileName: string; contentType: string };

/** Garde seulement les pièces valides, de l'utilisateur, dans la limite autorisée. */
export function sanitizeChatAttachments(raw: unknown, userId: string): ChatAttachmentRef[] {
  if (!Array.isArray(raw)) return [];
  const prefix = `${chatUploadPrefix(userId)}/`;
  const out: ChatAttachmentRef[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const r = item as Record<string, unknown>;
    const key = String(r.key || "").trim();
    const contentType = String(r.contentType || "").trim().toLowerCase();
    if (!key.startsWith(prefix) || key.includes("..") || key.includes("\0")) continue;
    if (!chatAttachmentKind(contentType)) continue;
    out.push({ key, fileName: String(r.fileName || "fichier").slice(0, 120), contentType });
    if (out.length >= CHAT_MAX_ATTACHMENTS) break;
  }
  return out;
}

const MAX_TEXT_PER_FILE = 20_000;
const MAX_TEXT_TOTAL = 60_000;
/** Poids total des photos envoyées au modèle par appel (les photos réduites font ~0,5 Mo). */
const MAX_IMAGE_BYTES_PER_CALL = 12 * 1024 * 1024;

/** UTF-8, sinon Windows-1252 (CSV exportés par Excel en français). */
function decodeText(bytes: Buffer): string {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes).replace(/^﻿/, "");
  } catch {
    return new TextDecoder("windows-1252").decode(bytes);
  }
}

export type ChatAttachmentContent = {
  /** Texte à ajouter au message utilisateur (contenu des PDF / fichiers texte, erreurs). */
  text: string;
  /** Images en data URL pour la vision du modèle. */
  images: Array<{ fileName: string; dataUrl: string }>;
  /** Textes extraits, à garder pour les questions suivantes. */
  docTexts: Array<{ key: string; fileName: string; text: string }>;
};

async function readObjectBytes(key: string): Promise<Buffer | null> {
  const { GetObjectCommand } = await import("@aws-sdk/client-s3");
  const { getTenantDataS3Client } = await import("@/app/lib/s3-clients");
  const { getBucketName } = await import("@/app/lib/s3-storage");
  try {
    const s3 = await getTenantDataS3Client();
    const res = await s3.send(new GetObjectCommand({ Bucket: await getBucketName(), Key: key }));
    const bytes = await res.Body?.transformToByteArray();
    return bytes ? Buffer.from(bytes) : null;
  } catch (err) {
    console.warn("[brain-ai] pièce jointe illisible", { key, err: String(err) });
    return null;
  }
}

function clip(text: string, max: number): string {
  return text.length > max ? `${text.slice(0, max)}\n[… texte tronqué]` : text;
}

/**
 * Lit les pièces jointes pour le modèle. `cachedTexts` évite de refaire l'OCR
 * d'un PDF déjà lu dans la conversation.
 */
export async function loadChatAttachmentContent(
  attachments: ChatAttachmentRef[],
  cachedTexts: Array<{ key: string; fileName: string; text: string }> = [],
): Promise<ChatAttachmentContent> {
  const cache = new Map(cachedTexts.map((d) => [d.key, d]));
  const results = await Promise.all(
    attachments.map(async (a, i) => {
      const label = `Fichier ${i + 1} — ${a.fileName}`;
      const kind = chatAttachmentKind(a.contentType);
      const cached = cache.get(a.key);
      if (kind !== "image" && cached) {
        return { label, kind, text: cached.text, doc: cached };
      }
      const bytes = await readObjectBytes(a.key);
      if (!bytes) return { label, kind, error: "fichier introuvable ou illisible" };
      if (kind === "image") {
        if (bytes.length > CHAT_MAX_IMAGE_BYTES) return { label, kind, error: "image trop lourde" };
        return {
          label,
          kind,
          imageBytes: bytes.length,
          image: { fileName: a.fileName, dataUrl: `data:${a.contentType};base64,${bytes.toString("base64")}` },
        };
      }
      if (kind === "text") {
        const text = clip(decodeText(bytes), MAX_TEXT_PER_FILE);
        return { label, kind, text, doc: { key: a.key, fileName: a.fileName, text } };
      }
      try {
        const { runTextractForPdfBytes } = await import("@/app/lib/ocr-textract");
        const ocr = await runTextractForPdfBytes(bytes);
        const text = clip(ocr.text.trim(), MAX_TEXT_PER_FILE);
        return { label, kind, text, doc: { key: a.key, fileName: a.fileName, text } };
      } catch (err) {
        console.warn("[brain-ai] OCR pièce jointe", { key: a.key, err: String(err) });
        // Mis en cache aussi : on ne relance pas l'OCR à chaque question.
        const text = "(lecture du PDF impossible : scan illisible ou fichier protégé)";
        return { label, kind, text, doc: { key: a.key, fileName: a.fileName, text } };
      }
    }),
  );

  const parts: string[] = [];
  const images: ChatAttachmentContent["images"] = [];
  const docTexts: ChatAttachmentContent["docTexts"] = [];
  let budget = MAX_TEXT_TOTAL;
  let imageBudget = MAX_IMAGE_BYTES_PER_CALL;
  for (const r of results) {
    if ("error" in r && r.error) {
      parts.push(`### ${r.label}\n(${r.error})`);
    } else if ("image" in r && r.image) {
      const size = "imageBytes" in r && typeof r.imageBytes === "number" ? r.imageBytes : 0;
      if (size > imageBudget) {
        parts.push(`### ${r.label}\n(photo non transmise : trop de photos lourdes dans le même envoi)`);
        continue;
      }
      imageBudget -= size;
      images.push(r.image);
      parts.push(`### ${r.label}\n(photo / image jointe ci-dessous, n° ${images.length})`);
    } else if ("text" in r && typeof r.text === "string") {
      if (budget <= 0) {
        parts.push(`### ${r.label}\n(non lu : trop de texte dans ce lot, posez la question sur ce fichier seul)`);
        continue;
      }
      const body = clip(r.text, budget);
      budget -= body.length;
      parts.push(`### ${r.label}\n${body || "(aucun texte détecté)"}`);
      // Cache du texte lu ; jamais d'un texte vide faute de budget.
      if (r.doc) docTexts.push({ ...r.doc, text: body });
    }
  }
  return {
    text: parts.length ? `Fichiers joints (${results.length}) :\n${parts.join("\n\n")}\n` : "",
    images,
    docTexts,
  };
}
