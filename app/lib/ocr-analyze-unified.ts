import "server-only";

import { analyzeDocMatchEleve } from "@/app/lib/ocr-analyze-eleve";
import {
  dedupeEnseignantsByFolder,
  filterEnseignantsForSecteurs,
  loadEnseignantsRegistry,
} from "@/app/lib/enseignants-registry";
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
import { matchEnseignantFromDocument } from "@/app/lib/ocr-enseignant-match";
import { analyzeDocMatchPersonnelRh } from "@/app/lib/rh/ocr-analyze-personnel-rh";

type AnalyzeOptions = {
  segmentMode?: boolean;
  knownStudent?: { ine?: string; nom: string; prenom: string; folderName: string };
};

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
 * Pipeline OCR unique — pool fusionné.
 *
 * On construit un pool selon les flux rattachés au compte :
 *   - élèves du/des secteur(s) activé(s)
 *   - enseignants des secteurs activés (collège, lycée, école selon caps)
 *   - personnel si le flux personnel est activé
 *
 * L'extraction nom/prénom/INE est faite par analyzeDocMatchEleve.
 * Ensuite on cherche le meilleur match dans le pool complet.
 * Le meilleur match (élève, enseignant ou personnel) détermine le dossier OneDrive.
 *
 * Si le compte n'a que des flux élèves, on délègue directement à analyzeDocMatchEleve
 * sans surcoût.
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
  const hasExtraFluxes = ensSecteurs.length > 0 || personnelEnabled;

  const runStudent =
    Boolean(options?.knownStudent) || elevesFluxes.length > 0 || Boolean(odProfile);

  // Cas simple : uniquement des flux élèves → délégation directe sans surcoût.
  if (runStudent && !hasExtraFluxes) {
    return analyzeDocMatchEleve(text, odProfile, trace, options);
  }

  // Étape 1 : extraction nom/prénom/INE via le pipeline élève (appel Mistral).
  // Le résultat élève peut déjà être définitif (knownStudent, match INE fort…).
  let studentResult: Record<string, unknown> | null = null;
  if (runStudent) {
    studentResult = await analyzeDocMatchEleve(text, odProfile, trace, options);
    tagStudentCandidates(studentResult);

    // Si l'identité était déjà connue (découpage ancré) → on garde le résultat élève tel quel.
    if (options?.knownStudent) {
      return { ...studentResult, subjectKind: "eleve" };
    }

    // Match élève très fort (INE ou dossier résolu) → on retourne directement.
    const eleveAutoWithFolder =
      studentResult.oneDriveFolderPath &&
      (studentResult.matchDebug as { matchedBy?: string } | undefined)?.matchedBy !== undefined;
    if (eleveAutoWithFolder) {
      const matchedBy = (studentResult.matchDebug as { matchedBy?: string })?.matchedBy ?? "";
      // Si le match est par INE, c'est très fiable : on ne cherche pas ailleurs.
      if (matchedBy === "ine" || matchedBy === "ine_fuzzy") {
        return { ...studentResult, subjectKind: "eleve" };
      }
    }
  }

  // Étape 2 : on récupère nom/prénom extraits pour alimenter les matchings enseignant/personnel.
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

  // Étape 3 : matching enseignants (flux activés pour ce compte uniquement).
  let bestEnseignant: {
    score: number;
    result: Record<string, unknown>;
  } | null = null;

  if (ensSecteurs.length > 0) {
    const enseignants = dedupeEnseignantsByFolder(
      filterEnseignantsForSecteurs(await loadEnseignantsRegistry(), ensSecteurs),
    );
    const enseignantsBase = findFluxBasePath(caps, "enseignants");
    const ensDecision = matchEnseignantFromDocument({
      text,
      extractedNom,
      extractedPrenom,
      enseignants,
      basePathFor: () => enseignantsBase,
    });

    ocrTraceCtx(trace, "classify", "enseignants", "matching enseignants (pool fusionné)", {
      decision: ensDecision.decision,
      reason: ensDecision.reason,
      pool: enseignants.length,
    });

    if (ensDecision.decision === "auto" && ensDecision.enseignant) {
      const e = ensDecision.enseignant;
      bestEnseignant = {
        score: ensDecision.confidence,
        result: {
          ...(studentResult || {}),
          nom: e.nom,
          prénom: e.prenom,
          fileName: String(
            (studentResult as { fileName?: string } | null)?.fileName || `${e.nom} ${e.prenom}`,
          ),
          oneDriveFolderPath: enseignantsBase
            ? oneDrivePathForEleve(enseignantsBase, e.folderName)
            : null,
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
        },
      };
    } else if (ensDecision.candidates.length > 0 && studentResult) {
      // Candidats enseignants non définitifs : on les ajoute à la shortlist.
      const existing = Array.isArray(studentResult.matchCandidates)
        ? studentResult.matchCandidates
        : [];
      studentResult = {
        ...studentResult,
        matchCandidates: [...existing, ...ensDecision.candidates],
        subjectKind: studentResult.subjectKind ?? "enseignant",
        matchDebug: {
          ...((studentResult as { matchDebug?: object } | null)?.matchDebug || {}),
          enseignantsCandidates: ensDecision.candidates.length,
        },
      };
    }
  }

  // Étape 4 : matching personnel (flux activé pour ce compte uniquement).
  let bestPersonnel: {
    score: number;
    result: Record<string, unknown>;
  } | null = null;

  if (personnelEnabled) {
    const entries = await loadPersonnelEntriesForOcr();
    const basePath = findFluxBasePath(caps, "personnel") || "Dossier personnel";
    const rh = await analyzeDocMatchPersonnelRh(text, basePath, entries, "document.pdf");
    const extracted = rh.extracted;

    ocrTraceCtx(trace, "classify", "personnel", "matching personnel OGEC (pool fusionné)", {
      matched: Boolean(rh.matchedEntry),
      pool: entries.length,
    });

    if (rh.oneDriveFolderPath && rh.matchedEntry) {
      const folder =
        rh.oneDriveFilePath?.replace(/\/[^/]+$/, "") || rh.oneDriveFolderPath;
      bestPersonnel = {
        score: rh.match.score ?? 0,
        result: {
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
        },
      };
    } else if (rh.match.candidates.length > 0 && studentResult) {
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

  // Étape 5 : arbitrage — on choisit le meilleur match toutes catégories confondues.
  // Priorité : enseignant auto > personnel auto > élève auto > revue collective.
  if (bestEnseignant && bestPersonnel) {
    return bestEnseignant.score >= bestPersonnel.score
      ? bestEnseignant.result
      : bestPersonnel.result;
  }
  if (bestEnseignant) return bestEnseignant.result;
  if (bestPersonnel) return bestPersonnel.result;

  // Pas de match auto sur enseignant/personnel → on revient au résultat élève
  // (peut contenir un auto-match élève ou une shortlist mixte à valider).
  if (studentResult) return studentResult;

  return {
    fileName: "Document",
    oneDriveFolderPath: null,
    matchCandidates: [],
    matchDebug: { decision: "none", reason: "aucun_flux" },
  };
}
