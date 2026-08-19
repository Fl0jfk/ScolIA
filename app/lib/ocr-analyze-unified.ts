import "server-only";

import { analyzeDocMatchEleve } from "@/app/lib/ocr-analyze-eleve";
import {
  filterEnseignantsForSecteurs,
  loadEnseignantsRegistry,
} from "@/app/lib/enseignants-registry";
import { matchEnseignantFromDocument } from "@/app/lib/ocr-enseignant-match";
import {
  enseignantsSecteursFromCapabilities,
  findFluxBasePath,
  hasPersonnelFlux,
  type OcrUserCapabilities,
} from "@/app/lib/ocr-flux";
import { loadPersonnelEntriesForOcr } from "@/app/lib/ocr-personnel-pool";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";
import { oneDrivePathForEleve } from "@/app/lib/onedrive-eleves";
import { ocrTraceCtx, type OcrTraceCtx } from "@/app/lib/ocr-trace";
import { analyzeDocMatchPersonnelRh } from "@/app/lib/rh/ocr-analyze-personnel-rh";

type AnalyzeOptions = {
  segmentMode?: boolean;
  knownStudent?: { ine?: string; nom: string; prenom: string; folderName: string };
};

function studentLooksSettled(result: {
  oneDriveFolderPath?: string | null;
  matchDebug?: { decision?: string };
  matchCandidates?: unknown[];
}): boolean {
  if (result.oneDriveFolderPath) return true;
  const decision = result.matchDebug?.decision;
  if (decision === "review" && Array.isArray(result.matchCandidates) && result.matchCandidates.length > 0) {
    return true;
  }
  if (decision === "auto") return true;
  return false;
}

function tagStudentCandidates(result: Record<string, unknown>) {
  const candidates = Array.isArray(result.matchCandidates) ? result.matchCandidates : [];
  result.matchCandidates = candidates.map((c) =>
    c && typeof c === "object" ? { kind: "eleve", ...(c as object) } : c,
  );
  if (result.matchDebug && typeof result.matchDebug === "object") {
    const debug = result.matchDebug as Record<string, unknown>;
    if (Array.isArray(debug.candidates)) {
      debug.candidates = debug.candidates.map((c) =>
        c && typeof c === "object" ? { kind: "eleve", ...(c as object) } : c,
      );
    }
  }
}

/**
 * Pipeline OCR unique.
 * Si le compte n’a que des flux élèves (cas actuel lycée / collège), on appelle
 * strictement `analyzeDocMatchEleve` — aucun autre matching.
 * Enseignants / personnel ne sont tentés que si l’élève n’a pas été identifié
 * (pas d’auto, pas de shortlist à valider).
 */
export async function analyzeDocForOcr(
  text: string,
  odProfile: OneDriveUserProfile | null,
  caps: OcrUserCapabilities | null,
  trace?: OcrTraceCtx,
  options?: AnalyzeOptions,
): Promise<Record<string, unknown>> {
  const elevesFluxes = caps?.fluxes.filter((f) => f.kind === "eleves") ?? [];
  const ensSecteurs = enseignantsSecteursFromCapabilities(caps);
  const personnelEnabled = hasPersonnelFlux(caps);
  const extraFluxes = ensSecteurs.length > 0 || personnelEnabled;

  const runStudent =
    Boolean(options?.knownStudent) || elevesFluxes.length > 0 || Boolean(odProfile);

  if (runStudent && !extraFluxes) {
    return analyzeDocMatchEleve(text, odProfile, trace, options);
  }

  let studentResult: Record<string, unknown> | null = null;
  if (runStudent) {
    studentResult = await analyzeDocMatchEleve(text, odProfile, trace, options);
    tagStudentCandidates(studentResult);
    if (options?.knownStudent || studentLooksSettled(studentResult)) {
      return { ...studentResult, subjectKind: studentResult.oneDriveFolderPath ? "eleve" : "eleve" };
    }
  }

  const extractedNom = String(
    (studentResult as { nom?: string; eleve?: { nom?: string } } | null)?.eleve?.nom ??
      (studentResult as { nom?: string } | null)?.nom ??
      "",
  );
  const extractedPrenom = String(
    (studentResult as { prénom?: string; prenom?: string; eleve?: { prénom?: string } } | null)?.eleve
      ?.prénom ??
      (studentResult as { prénom?: string; prenom?: string } | null)?.prénom ??
      (studentResult as { prenom?: string } | null)?.prenom ??
      "",
  );

  if (ensSecteurs.length > 0) {
    const enseignants = filterEnseignantsForSecteurs(await loadEnseignantsRegistry(), ensSecteurs);
    const ensDecision = matchEnseignantFromDocument({
      text,
      extractedNom,
      extractedPrenom,
      enseignants,
      basePathFor: (secteur) => findFluxBasePath(caps, "enseignants", secteur),
    });
    ocrTraceCtx(trace, "classify", "enseignants", "matching enseignants", {
      decision: ensDecision.decision,
      reason: ensDecision.reason,
      pool: enseignants.length,
    });
    if (ensDecision.decision === "auto" && ensDecision.enseignant) {
      const e = ensDecision.enseignant;
      const basePath = findFluxBasePath(caps, "enseignants", e.secteur);
      return {
        ...(studentResult || {}),
        nom: e.nom,
        prénom: e.prenom,
        fileName: String(
          (studentResult as { fileName?: string } | null)?.fileName || `${e.nom} ${e.prenom}`,
        ),
        oneDriveFolderPath: basePath ? oneDrivePathForEleve(basePath, e.folderName) : null,
        matchedEnseignant: e,
        matchCandidates: ensDecision.candidates,
        subjectKind: "enseignant",
        matchDebug: {
          ...((studentResult as { matchDebug?: object } | null)?.matchDebug || {}),
          subjectKind: "enseignant",
          decision: "auto",
          matchedBy: ensDecision.reason,
          enseignantsPool: enseignants.length,
        },
      };
    }
    if (ensDecision.candidates.length > 0 && !(Array.isArray(studentResult?.matchCandidates) && studentResult.matchCandidates.length)) {
      studentResult = {
        ...(studentResult || {}),
        matchCandidates: ensDecision.candidates,
        subjectKind: "enseignant",
        matchDebug: {
          ...((studentResult as { matchDebug?: object } | null)?.matchDebug || {}),
          subjectKind: "enseignant",
          decision: ensDecision.decision,
          reason: ensDecision.reason,
        },
      };
    } else if (ensDecision.candidates.length > 0 && studentResult) {
      const existing = Array.isArray(studentResult.matchCandidates)
        ? studentResult.matchCandidates
        : [];
      studentResult = {
        ...studentResult,
        matchCandidates: [...existing, ...ensDecision.candidates],
      };
    }
  }

  if (personnelEnabled) {
    const entries = await loadPersonnelEntriesForOcr();
    const basePath = findFluxBasePath(caps, "personnel") || "Dossier personnel";
    const rh = await analyzeDocMatchPersonnelRh(text, basePath, entries, "document.pdf");
    const extracted = rh.extracted;
    ocrTraceCtx(trace, "classify", "personnel", "matching personnel OGEC", {
      matched: Boolean(rh.matchedEntry),
      pool: entries.length,
    });
    if (rh.oneDriveFolderPath && rh.matchedEntry) {
      const folder =
        rh.oneDriveFilePath?.replace(/\/[^/]+$/, "") || rh.oneDriveFolderPath;
      return {
        ...(studentResult || {}),
        nom: extracted.nom || rh.matchedEntry.displayName,
        prénom: extracted.prenom || "",
        fileName: rh.fileName,
        oneDriveFolderPath: folder,
        matchedPersonnel: rh.matchedEntry,
        subjectKind: "personnel",
        matchCandidates: rh.match.candidates.map((c) => ({
          kind: "personnel",
          nom: c.displayName,
          prenom: "",
          folderName: c.folderName,
          folderPath: oneDrivePathForEleve(basePath, c.folderName),
          score: rh.match.score,
          matchedBy: rh.match.matchedBy || "name",
        })),
        matchDebug: {
          ...((studentResult as { matchDebug?: object } | null)?.matchDebug || {}),
          subjectKind: "personnel",
          decision: "auto",
          matchedBy: rh.match.matchedBy,
        },
      };
    }
    if (rh.match.candidates.length > 0) {
      const extra = rh.match.candidates.map((c) => ({
        kind: "personnel",
        nom: c.displayName,
        prenom: "",
        folderName: c.folderName,
        folderPath: oneDrivePathForEleve(basePath, c.folderName),
        score: rh.match.score,
        matchedBy: rh.match.matchedBy || "name",
      }));
      const existing = Array.isArray(studentResult?.matchCandidates)
        ? studentResult!.matchCandidates
        : [];
      studentResult = {
        ...(studentResult || {}),
        matchCandidates: [...existing, ...extra],
        matchDebug: {
          ...((studentResult as { matchDebug?: object } | null)?.matchDebug || {}),
          personnelCandidates: extra.length,
        },
      };
    }
  }

  return studentResult || {
    fileName: "Document",
    oneDriveFolderPath: null,
    matchCandidates: [],
    matchDebug: { decision: "none", reason: "aucun_flux" },
  };
}
