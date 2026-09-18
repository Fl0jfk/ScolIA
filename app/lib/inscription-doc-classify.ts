import "server-only";

import { getMistralApiKey } from "@/app/lib/tenant-config";
import { runTextractForPdfBytes } from "@/app/lib/ocr-textract";
import {
  buildInscriptionDocumentTitle,
  guessInscriptionKindFromFileName,
  inscriptionDocKindLabel,
  inscriptionKindFromAiType,
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

/**
 * Même esprit que `analyzeDocMatchEleve` : décrire le document, ne pas inventer.
 * Différence : l’élève du dossier est déjà connu → pas de nom / prénom.
 */
function classifyExtractionPrompt(): string {
  return `Analyse ce document scolaire ou administratif.
Extrais UNIQUEMENT ce qui est clairement présent. Ne devine JAMAIS.

L'élève du dossier est déjà connu : n'extrais PAS de nom ni de prénom.

- titre_document : titre EXPLICITE pour nommer le fichier, SANS nom/prénom.
  Ex. "Bulletin scolaire 2ème semestre 2A", "Carte d'identité", "Attestation d'assurance scolaire", "Certificat de scolarité".
  Interdit : "Document", "Fichier", "PDF".
- type : Bulletin, Relevé de notes, Carte d'identité, Certificat de scolarité, Certificat de radiation, Livret de famille, Justificatif de domicile, Photo d'identité, Attestation d'assurance, Fiche d'inscription, Vaccinations, PAP, PAI, PPS, GEVASCO, Jugement, Autre
- detail : précision utile sinon "non_trouvé"
- origine : "interne" si document de l'établissement (bulletin, relevé, certificat de scolarité, Pronote, Charlemagne),
  "externe" si CNI, passeport, CAF, mutuelle, médecin, assurance, organisme extérieur.

JSON uniquement :
{
  "titre_document": "...",
  "type": "...",
  "detail": "...",
  "origine": "interne"
}
Si un champ est absent : "non_trouvé".`;
}

async function mistralJsonCompletion(
  apiKey: string,
  messages: Array<{ role: "user" | "system"; content: string | unknown }>,
  model: string,
): Promise<Record<string, unknown>> {
  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model,
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

function buildResult(opts: {
  nom: string;
  prenom: string;
  kind: InscriptionDocKind;
  detail: string | null;
  usedOcr: boolean;
  warning?: string;
}): InscriptionDocClassification {
  return {
    kind: opts.kind,
    kindLabel: inscriptionDocKindLabel(opts.kind),
    detail: opts.detail,
    title: buildInscriptionDocumentTitle({
      nom: opts.nom,
      prenom: opts.prenom,
      kind: opts.kind,
      detail: opts.detail,
    }),
    usedOcr: opts.usedOcr,
    warning: opts.warning,
  };
}

function classificationFromExtracted(
  extracted: Record<string, unknown>,
  opts: { nom: string; prenom: string; fileName: string },
): InscriptionDocClassification {
  const titre = cleanField(extracted.titre_document);
  const type = cleanField(extracted.type);
  const detailRaw = cleanField(extracted.detail);
  const origine = cleanField(extracted.origine);
  const kind = inscriptionKindFromAiType({
    type,
    titre,
    detail: detailRaw,
    origine,
    fileName: opts.fileName,
  });
  // Préférer le titre explicite IA ; sinon détail ; sinon label du kind.
  const detail =
    titre ||
    (detailRaw && !/^non[_\s-]?trouv/i.test(detailRaw) ? detailRaw : null) ||
    null;
  return buildResult({
    nom: opts.nom,
    prenom: opts.prenom,
    kind,
    detail,
    usedOcr: true,
  });
}

function fallbackFromFileName(
  opts: { nom: string; prenom: string; fileName: string },
  warning: string,
): InscriptionDocClassification {
  const kind = guessInscriptionKindFromFileName(opts.fileName);
  return buildResult({
    nom: opts.nom,
    prenom: opts.prenom,
    kind,
    detail: null,
    usedOcr: false,
    warning,
  });
}

/** Classification à partir du texte OCR (élève déjà connu — pas de matching identité). */
export async function classifyInscriptionDocFromText(
  text: string,
  opts: { nom: string; prenom: string; fileName: string },
): Promise<InscriptionDocClassification> {
  const apiKey = await getMistralApiKey();
  if (!apiKey || !text.trim()) {
    return fallbackFromFileName(
      opts,
      apiKey
        ? "Texte OCR vide — type estimé depuis le nom de fichier."
        : "IA non configurée — type estimé depuis le nom de fichier.",
    );
  }

  try {
    const extracted = await mistralJsonCompletion(
      apiKey,
      [
        {
          role: "user",
          content: `${classifyExtractionPrompt()}

Texte :
---
${text.slice(0, 12_000)}
---`,
        },
      ],
      "mistral-medium",
    );
    return classificationFromExtracted(extracted, opts);
  } catch (err) {
    return fallbackFromFileName(
      opts,
      err instanceof Error ? err.message : "Classification IA impossible.",
    );
  }
}

async function classifyInscriptionImageBytes(
  bytes: Uint8Array,
  mimeType: string,
  opts: { nom: string; prenom: string; fileName: string },
): Promise<InscriptionDocClassification> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) {
    return fallbackFromFileName(opts, "IA non configurée — type estimé depuis le nom de fichier.");
  }

  const mime = mimeType.startsWith("image/") ? mimeType : "image/jpeg";
  const b64 = Buffer.from(bytes).toString("base64");
  try {
    const extracted = await mistralJsonCompletion(
      apiKey,
      [
        {
          role: "user",
          content: [
            { type: "text", text: classifyExtractionPrompt() },
            {
              type: "image_url",
              image_url: { url: `data:${mime};base64,${b64}` },
            },
          ],
        },
      ],
      "mistral-medium",
    );
    return classificationFromExtracted(extracted, opts);
  } catch (err) {
    return fallbackFromFileName(
      opts,
      err instanceof Error ? err.message : "Classification image impossible.",
    );
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
      return fallbackFromFileName(
        opts,
        err instanceof Error ? err.message : "OCR PDF impossible.",
      );
    }
  }

  if (isImage(opts.fileName, opts.mimeType)) {
    return classifyInscriptionImageBytes(bytes, opts.mimeType, opts);
  }

  return fallbackFromFileName(opts, "Format non OCR — type estimé depuis le nom de fichier.");
}
