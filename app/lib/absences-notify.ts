import { getAbsenceDocumentKeys } from "@/app/lib/absences-documents";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { getObjectBytes } from "@/app/lib/s3-storage";
import { fetchTravelsPdfBytes, resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";

type AbsenceMailAttachment = {
  filename: string;
  content: Buffer;
  contentType: string;
};

function guessContentType(filename: string) {
  const lower = filename.toLowerCase();
  if (lower.endsWith(".pdf")) return "application/pdf";
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
  return "application/octet-stream";
}

function safeFilename(name: string, fallback: string) {
  const cleaned = String(name || "")
    .replace(/[^\w.\-àâäéèêëïîôùûüç\s]/gi, "_")
    .trim();
  return cleaned || fallback;
}

/** Pièces jointes à inclure dans le mail compta / secrétariat après validation. */
export async function loadAbsenceValidationAttachments(
  record: AbsenceRecord,
): Promise<AbsenceMailAttachment[]> {
  const out: AbsenceMailAttachment[] = [];
  const seen = new Set<string>();

  const docKeys = getAbsenceDocumentKeys(record);
  for (let i = 0; i < docKeys.length; i += 1) {
    const key = docKeys[i]!;
    if (seen.has(key)) continue;
    seen.add(key);
    try {
      const buf = await getObjectBytes(key);
      if (!buf?.length) continue;
      const basename = key.split("/").pop() || `document-${i + 1}.pdf`;
      const filename = safeFilename(basename, `document-${i + 1}.pdf`);
      out.push({ filename, content: buf, contentType: guessContentType(filename) });
    } catch (e) {
      console.error(`Absences mail attachment (doc ${key}):`, e);
    }
  }

  const justUrl = record.justification?.fileUrl?.trim();
  if (justUrl) {
    try {
      const justKey = await resolveTravelsS3ObjectKey(justUrl);
      if (justKey && seen.has(justKey)) {
        return out;
      }

      let buf: Buffer | null = null;
      if (justKey) {
        seen.add(justKey);
        buf = await getObjectBytes(justKey);
      }
      if (!buf?.length) {
        buf = await fetchTravelsPdfBytes(justUrl, justKey);
      }
      if (buf?.length) {
        const filename = safeFilename(record.justification?.fileName || "", "justificatif.pdf");
        out.push({ filename, content: buf, contentType: guessContentType(filename) });
      }
    } catch (e) {
      console.error("Absences mail attachment (justificatif):", e);
    }
  }

  return out;
}

export function formatJustificatifMailLine(
  record: AbsenceRecord,
  attachments: AbsenceMailAttachment[],
) {
  if (attachments.length > 0) {
    return `Pièces jointes (${attachments.length}) : ${attachments.map((a) => a.filename).join(", ")}`;
  }
  if (record.justification?.fileName) {
    return `Justificatif déposé (${record.justification.fileName}) — fichier non récupéré pour la pièce jointe.`;
  }
  return "Justificatif : non déposé";
}
