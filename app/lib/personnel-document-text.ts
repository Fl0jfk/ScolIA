import { runTextractForPdfBytes } from "@/app/lib/ocr-textract";

/** Chaînes lisibles dans les binaires Office / Excel (sans librairie dédiée). */
function scrapeBinaryText(bytes: Uint8Array, maxChars = 20_000): string {
  const buf = Buffer.from(bytes);
  const latin = buf.toString("latin1");
  const runs = latin.match(/[\x20-\x7E\u00C0-\u024F\u00DF\u0152\u0153]{4,}/g) || [];
  const uniq = Array.from(new Set(runs.map((s) => s.trim()).filter(Boolean)));
  return uniq.join("\n").slice(0, maxChars);
}

/** Extraction PDF via Mistral OCR. */
async function extractPdfText(bytes: Uint8Array): Promise<string> {
  const result = await runTextractForPdfBytes(bytes);
  const text = result.text;
  if (!text.trim()) throw new Error("Impossible de lire le texte du PDF.");
  return text;
}

export async function extractTextFromUpload(
  bytes: Uint8Array,
  fileName: string,
  mimeType: string,
): Promise<string> {
  const lower = fileName.toLowerCase();
  const mime = (mimeType || "").toLowerCase();

  if (lower.endsWith(".pdf") || mime.includes("pdf")) {
    return extractPdfText(bytes);
  }

  const scraped = scrapeBinaryText(bytes);
  if (scraped.trim().length < 8) {
    throw new Error(
      "Texte insuffisant pour l'analyse. Essayez un PDF ou un export plus lisible.",
    );
  }
  return scraped;
}
