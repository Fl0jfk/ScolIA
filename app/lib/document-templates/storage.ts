import "server-only";

import { getJson, putJson, putObject, getObjectBytes } from "@/app/lib/s3-storage";
import type {
  DocumentOutputFormat,
  GeneratedDocument,
  GeneratedDocumentIndexEntry,
} from "@/app/lib/document-templates/types";

const INDEX_KEY = "documents/generated/index.json";
const MAX_INDEX = 80;

export function generatedMetaKey(id: string) {
  return `documents/generated/${id}.json`;
}

export function generatedPdfKey(id: string) {
  return `documents/generated/${id}.pdf`;
}

export function generatedFileKey(id: string, format: DocumentOutputFormat | "pdf") {
  if (format === "docx") return `documents/generated/${id}.docx`;
  if (format === "fillable-pdf") return `documents/generated/${id}-fillable.pdf`;
  return generatedPdfKey(id);
}

export function contentTypeForFormat(format: DocumentOutputFormat | "pdf"): string {
  if (format === "docx") {
    return "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
  }
  return "application/pdf";
}

export function extensionForFormat(format: DocumentOutputFormat | "pdf"): string {
  if (format === "docx") return "docx";
  return "pdf";
}

export async function loadGeneratedIndex(): Promise<GeneratedDocumentIndexEntry[]> {
  const hit = await getJson<GeneratedDocumentIndexEntry[]>(INDEX_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function saveGeneratedIndex(entries: GeneratedDocumentIndexEntry[]) {
  await putJson(INDEX_KEY, entries.slice(0, MAX_INDEX));
}

export async function saveGeneratedDocument(
  doc: GeneratedDocument,
  bytes: Uint8Array | Buffer,
): Promise<GeneratedDocument> {
  const format = doc.format || "pdf";
  const fileKey = doc.fileKey || generatedFileKey(doc.id, format);
  await putObject(fileKey, Buffer.from(bytes), contentTypeForFormat(format));
  const stored: GeneratedDocument = {
    ...doc,
    format,
    fileKey,
    pdfKey: format === "pdf" || format === "fillable-pdf" ? fileKey : doc.pdfKey,
  };
  await putJson(generatedMetaKey(doc.id), stored);
  const index = await loadGeneratedIndex();
  const entry: GeneratedDocumentIndexEntry = {
    id: stored.id,
    templateId: stored.templateId,
    templateLabel: stored.templateLabel,
    title: stored.title,
    createdAt: stored.createdAt,
    eleveIne: stored.eleveIne,
    format: stored.format,
  };
  await saveGeneratedIndex([entry, ...index.filter((e) => e.id !== doc.id)]);
  return stored;
}

export async function loadGeneratedDocument(id: string): Promise<GeneratedDocument | null> {
  const hit = await getJson<GeneratedDocument>(generatedMetaKey(id));
  if (!hit?.data) return null;
  const data = hit.data;
  // Rétrocompat docs phase 1–2 (pdfKey seul)
  if (!data.fileKey && data.pdfKey) {
    return { ...data, fileKey: data.pdfKey, format: data.format || "pdf" };
  }
  if (!data.format) return { ...data, format: "pdf", fileKey: data.fileKey || data.pdfKey || "" };
  return data;
}

export async function loadGeneratedFileBytes(id: string): Promise<Buffer | null> {
  const meta = await loadGeneratedDocument(id);
  if (!meta?.fileKey) return null;
  return getObjectBytes(meta.fileKey);
}

/** @deprecated alias */
export async function loadGeneratedPdfBytes(id: string): Promise<Buffer | null> {
  return loadGeneratedFileBytes(id);
}
