import "server-only";

import {
  extractPersonnelFieldsFromText,
  type PersonnelExtracted,
} from "@/app/lib/personnel-match";
import { mapExtractedDocCategory } from "@/app/lib/personnel-rh-cycles";
import {
  matchRhPersonnelFromIndex,
  type RhPersonnelMatchResult,
} from "@/app/lib/rh/rh-personnel-match";
import type { RhDocCategory, RhPersonnelIndexEntry } from "@/app/lib/rh/types";
import { rhDocumentPath, rhPersonnelFolderPath, type RhDocSubfolder } from "@/app/lib/rh/paths";

function personnelCategoryToRhSubfolder(cat: string): RhDocSubfolder {
  if (cat === "medecine") return "medecine";
  if (cat === "formation") return "formations";
  if (cat === "habilitation") return "habilitations";
  if (cat === "contrat" || cat === "onboarding" || cat === "entretien") return "contrats";
  return "divers";
}

function rhDocCategoryFromExtracted(typeDocument?: string): RhDocCategory {
  const mapped = mapExtractedDocCategory(typeDocument);
  if (mapped === "medecine") return "medecine";
  if (mapped === "formation") return "formation";
  if (mapped === "habilitation") return "habilitation";
  if (mapped === "contrat" || mapped === "onboarding" || mapped === "entretien") return "contrat";
  return "autre";
}

function buildRhFileName(extracted: PersonnelExtracted, originalName: string): string {
  const base = [extracted.type_document, extracted.nom, extracted.prenom]
    .filter(Boolean)
    .join(" ")
    .replace(/[^a-zA-Z0-9._\-\sàâäéèêëïîôùûüç]/gi, "")
    .trim();
  const ext = originalName.includes(".") ? originalName.slice(originalName.lastIndexOf(".")) : ".pdf";
  if (base) return `${base}${ext}`;
  return originalName.replace(/[^a-zA-Z0-9._-]/g, "_") || `document${ext}`;
}

type RhOcrAnalyzeResult = {
  fileName: string;
  oneDriveFolderPath: string | null;
  oneDriveFilePath: string | null;
  matchedEntry: RhPersonnelIndexEntry | null;
  extracted: PersonnelExtracted;
  match: RhPersonnelMatchResult;
  docCategory: RhDocCategory;
  matchDebug?: Record<string, unknown>;
};

export async function analyzeDocMatchPersonnelRh(
  text: string,
  basePath: string,
  indexEntries: RhPersonnelIndexEntry[],
  originalFileName: string,
): Promise<RhOcrAnalyzeResult> {
  const extracted = await extractPersonnelFieldsFromText(text);
  const match = matchRhPersonnelFromIndex(extracted, indexEntries);

  const docCategory = rhDocCategoryFromExtracted(extracted.type_document);
  const subfolder = personnelCategoryToRhSubfolder(mapExtractedDocCategory(extracted.type_document));
  const fileName = buildRhFileName(extracted, originalFileName);

  if (!match.entry) {
    return {
      fileName,
      oneDriveFolderPath: null,
      oneDriveFilePath: null,
      matchedEntry: null,
      extracted,
      match,
      docCategory,
      matchDebug: { reason: "no_match", candidates: match.candidates.map((c) => c.displayName) },
    };
  }

  const folderPath = rhPersonnelFolderPath(match.entry.folderName, basePath);
  const filePath = rhDocumentPath(match.entry.folderName, subfolder, fileName, basePath);

  return {
    fileName,
    oneDriveFolderPath: folderPath,
    oneDriveFilePath: filePath,
    matchedEntry: match.entry,
    extracted,
    match,
    docCategory,
    matchDebug: { matchedBy: match.matchedBy, score: match.score },
  };
}
