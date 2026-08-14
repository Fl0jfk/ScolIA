import type { CertificateLine } from "@/app/lib/certificates-types";

type CertificateLineInput = {
  title?: string;
  period?: string;
  description?: string;
};

export function normalizeCertificateLine(raw: unknown): CertificateLine | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const id = typeof o.id === "string" ? o.id.trim() : "";
  if (!id) return null;

  const legacyText = String(o.text || "").trim();
  const title = String(o.title || "").trim();
  const description = String(o.description || "").trim();
  const period = String(o.period || "").trim() || undefined;

  if (!title && !description && !legacyText) return null;

  return {
    id,
    title: title || "Réalisation",
    period,
    description: description || legacyText || title,
    text: legacyText || undefined,
    addedBy: String(o.addedBy || "").trim(),
    addedByName: String(o.addedByName || "").trim() || "Enseignant",
    addedAt: String(o.addedAt || new Date().toISOString()),
  };
}

export function parseCertificateLineInput(input: unknown): CertificateLineInput | { error: string } {
  if (typeof input === "string") {
    const description = input.trim();
    if (!description) return { error: "Description requise." };
    return { description };
  }
  if (!input || typeof input !== "object") {
    return { error: "Ligne invalide." };
  }
  const o = input as Record<string, unknown>;
  const title = String(o.title || "").trim();
  const period = String(o.period || "").trim();
  const description = String(o.description || "").trim();
  if (!title && !description) {
    return { error: "Titre ou description requis." };
  }
  return {
    title: title || undefined,
    period: period || undefined,
    description: description || title,
  };
}

export function buildCertificateLineFromInput(
  input: CertificateLineInput,
  meta: { addedBy: string; addedByName: string; id: string; addedAt: string },
): CertificateLine {
  const title = String(input.title || "").trim() || "Réalisation";
  const description = String(input.description || "").trim() || title;
  const period = String(input.period || "").trim() || undefined;
  return {
    id: meta.id,
    title,
    period,
    description,
    addedBy: meta.addedBy,
    addedByName: meta.addedByName,
    addedAt: meta.addedAt,
  };
}

export function certificateLineIsComplete(line: CertificateLine): boolean {
  return Boolean(line.title?.trim() && line.description?.trim());
}
