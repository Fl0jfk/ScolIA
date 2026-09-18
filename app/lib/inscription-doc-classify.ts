import "server-only";

import { getMistralApiKey } from "@/app/lib/tenant-config";
import { runTextractForPdfBytes } from "@/app/lib/ocr-textract";
import {
  buildInscriptionDocumentTitle,
  guessInscriptionKindFromFileName,
  INSCRIPTION_DOC_KINDS,
  inscriptionDocKindLabel,
  isInscriptionDocKind,
  type InscriptionDocKind,
} from "@/app/lib/inscription-doc-kinds";

export type InscriptionDocClassification = {
  kind: InscriptionDocKind;
  kindLabel: string;
  detail: string | null;
  title: string;
  usedOcr: boolean;
  warning?: string;
};

function cleanField(v: unknown): string | null {
  if (typeof v !== "string") return null;
  const t = v.trim();
  if (!t || /^non[_\s-]?trouv/i.test(t) || t === "null" || t === "undefined") return null;
  return t;
}

function parseKind(raw: unknown, fileName: string): InscriptionDocKind {
  if (typeof raw === "string") {
    const slug = raw
      .trim()
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/[\s-]+/g, "_");
    if (isInscriptionDocKind(slug)) return slug;
    // Aliases courants renvoyés par le modèle
    if (/fiche|inscription/.test(slug)) return "fiche_inscription";
    if (/bulletin/.test(slug)) return "bulletin";
    if (/releve|notes/.test(slug)) return "releve_notes";
    if (/identite|cni|passeport/.test(slug)) return "piece_identite";
    if (/livret/.test(slug)) return "livret_famille";
    if (/domicile/.test(slug)) return "justificatif_domicile";
    if (/photo/.test(slug)) return "photo_identite";
    if (/assurance|mutuelle/.test(slug)) return "attestation_assurance";
    if (/scolarite/.test(slug)) return "certificat_scolarite";
    if (/radiation|exeat/.test(slug)) return "certificat_radiation";
    if (/vaccin|sante/.test(slug)) return "vaccinations";
    if (slug === "pap") return "pap";
    if (slug === "pai") return "pai";
    if (slug === "pps") return "pps";
    if (/gevasco/.test(slug)) return "gevasco";
    if (/jugement|garde|autorite/.test(slug)) return "jugement";
  }
  return guessInscriptionKindFromFileName(fileName);
}

async function mistralJsonCompletion(
  apiKey: string,
  messages: Array<{ role: "user" | "system"; content: string | unknown }>,
): Promise<Record<string, unknown>> {
  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      messages,
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`Mistral classify HTTP ${res.status}: ${body.slice(0, 240)}`);
  }
  const data = (await res.json()) as {
    choices?: Array<{ message?: { content?: string } }>;
  };
  let raw = data.choices?.[0]?.message?.content || "";
  raw = raw.trim().replace(/```json/gi, "").replace(/```/g, "").trim();
  const start = raw.indexOf("{");
  const end = raw.lastIndexOf("}");
  if (start === -1 || end === -1) throw new Error("Réponse IA sans JSON.");
  return JSON.parse(raw.slice(start, end + 1)) as Record<string, unknown>;
}

const KIND_LIST_FOR_PROMPT = INSCRIPTION_DOC_KINDS.join(", ");

function classifyPromptPreamble(): string {
  return `Tu classifies une pièce pour un dossier d'inscription scolaire français.
Réponds UNIQUEMENT en JSON :
{
  "kind": "<un parmi : ${KIND_LIST_FOR_PROMPT}>",
  "titre_document": "<titre court SANS nom ni prénom d'élève, ex. Bulletin 2e semestre 2024-2025>"
}
Interdit dans titre_document : Document, Fichier, PDF, ou un nom/prénom.
Si le type est ambigu : kind = "autre" et un titre précis si possible.`;
}

/** Classification à partir du texte OCR (élève déjà connu — pas de matching identité). */
export async function classifyInscriptionDocFromText(
  text: string,
  opts: { nom: string; prenom: string; fileName: string },
): Promise<InscriptionDocClassification> {
  const apiKey = await getMistralApiKey();
  if (!apiKey || !text.trim()) {
    const kind = guessInscriptionKindFromFileName(opts.fileName);
    return {
      kind,
      kindLabel: inscriptionDocKindLabel(kind),
      detail: null,
      title: buildInscriptionDocumentTitle({
        nom: opts.nom,
        prenom: opts.prenom,
        kind,
      }),
      usedOcr: false,
      warning: apiKey
        ? "Texte OCR vide — type estimé depuis le nom de fichier."
        : "IA non configurée — type estimé depuis le nom de fichier.",
    };
  }

  try {
    const extracted = await mistralJsonCompletion(apiKey, [
      {
        role: "user",
        content: `${classifyPromptPreamble()}

Texte OCR :
---
${text.slice(0, 12_000)}
---`,
      },
    ]);
    const kind = parseKind(extracted.kind, opts.fileName);
    const detail = cleanField(extracted.titre_document);
    return {
      kind,
      kindLabel: inscriptionDocKindLabel(kind),
      detail,
      title: buildInscriptionDocumentTitle({
        nom: opts.nom,
        prenom: opts.prenom,
        kind,
        detail,
      }),
      usedOcr: true,
    };
  } catch (err) {
    const kind = guessInscriptionKindFromFileName(opts.fileName);
    return {
      kind,
      kindLabel: inscriptionDocKindLabel(kind),
      detail: null,
      title: buildInscriptionDocumentTitle({
        nom: opts.nom,
        prenom: opts.prenom,
        kind,
      }),
      usedOcr: false,
      warning: err instanceof Error ? err.message : "Classification IA impossible.",
    };
  }
}

async function classifyInscriptionImageBytes(
  bytes: Uint8Array,
  mimeType: string,
  opts: { nom: string; prenom: string; fileName: string },
): Promise<InscriptionDocClassification> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) {
    const kind = guessInscriptionKindFromFileName(opts.fileName);
    return {
      kind,
      kindLabel: inscriptionDocKindLabel(kind),
      detail: null,
      title: buildInscriptionDocumentTitle({
        nom: opts.nom,
        prenom: opts.prenom,
        kind,
      }),
      usedOcr: false,
      warning: "IA non configurée — type estimé depuis le nom de fichier.",
    };
  }

  const mime = mimeType.startsWith("image/") ? mimeType : "image/jpeg";
  const b64 = Buffer.from(bytes).toString("base64");
  try {
    const extracted = await mistralJsonCompletion(apiKey, [
      {
        role: "user",
        content: [
          { type: "text", text: classifyPromptPreamble() },
          {
            type: "image_url",
            image_url: { url: `data:${mime};base64,${b64}` },
          },
        ],
      },
    ]);
    const kind = parseKind(extracted.kind, opts.fileName);
    const detail = cleanField(extracted.titre_document);
    return {
      kind,
      kindLabel: inscriptionDocKindLabel(kind),
      detail,
      title: buildInscriptionDocumentTitle({
        nom: opts.nom,
        prenom: opts.prenom,
        kind,
        detail,
      }),
      usedOcr: true,
    };
  } catch (err) {
    const kind = guessInscriptionKindFromFileName(opts.fileName);
    return {
      kind,
      kindLabel: inscriptionDocKindLabel(kind),
      detail: null,
      title: buildInscriptionDocumentTitle({
        nom: opts.nom,
        prenom: opts.prenom,
        kind,
      }),
      usedOcr: false,
      warning: err instanceof Error ? err.message : "Classification image impossible.",
    };
  }
}

function isPdf(fileName: string, mimeType: string): boolean {
  return (
    fileName.toLowerCase().endsWith(".pdf") ||
    mimeType.toLowerCase().includes("pdf")
  );
}

function isImage(fileName: string, mimeType: string): boolean {
  const mime = mimeType.toLowerCase();
  if (mime.startsWith("image/")) return true;
  return /\.(jpe?g|png|webp|gif)$/i.test(fileName);
}

/**
 * OCR + classification d’une pièce d’inscription.
 * L’élève est déjà connu : on ne cherche pas de nom dans le document.
 */
export async function classifyInscriptionDocumentBytes(
  bytes: Uint8Array,
  opts: { nom: string; prenom: string; fileName: string; mimeType: string },
): Promise<InscriptionDocClassification> {
  if (isPdf(opts.fileName, opts.mimeType)) {
    try {
      const ocr = await runTextractForPdfBytes(bytes);
      return classifyInscriptionDocFromText(ocr.text, opts);
    } catch (err) {
      const kind = guessInscriptionKindFromFileName(opts.fileName);
      return {
        kind,
        kindLabel: inscriptionDocKindLabel(kind),
        detail: null,
        title: buildInscriptionDocumentTitle({
          nom: opts.nom,
          prenom: opts.prenom,
          kind,
        }),
        usedOcr: false,
        warning: err instanceof Error ? err.message : "OCR PDF impossible.",
      };
    }
  }

  if (isImage(opts.fileName, opts.mimeType)) {
    return classifyInscriptionImageBytes(bytes, opts.mimeType, opts);
  }

  const kind = guessInscriptionKindFromFileName(opts.fileName);
  return {
    kind,
    kindLabel: inscriptionDocKindLabel(kind),
    detail: null,
    title: buildInscriptionDocumentTitle({
      nom: opts.nom,
      prenom: opts.prenom,
      kind,
    }),
    usedOcr: false,
    warning: "Format non OCR — type estimé depuis le nom de fichier.",
  };
}
