import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import { normalizeEleveDateNaissance, resolveEleveFolderName } from "@/app/lib/eleves-config";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import {
  buildElevesPoolForOcrMatching,
  oneDrivePathForEleve,
} from "@/app/lib/onedrive-eleves";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { ocrTraceCtx, type OcrTraceCtx } from "@/app/lib/ocr-trace";
import type { KnownStudent } from "@/app/lib/ocr-segmentation";
import {
  matchEleveFromDocument,
  type EleveMatchCandidateView,
  type ExtractedIdentity,
} from "@/app/lib/ocr-eleve-match";

async function getElevesFromS3(): Promise<EleveConfig[]> {
  return loadElevesRegistry();
}

/**
 * Liste des élèves connus, pré-normalisée pour le découpage ancré identité.
 * Filtrée par secteur (réduit les faux positifs de nom) quand un profil OneDrive est fourni.
 */
export async function loadKnownStudentsForSegmentation(
  odProfile: OneDriveUserProfile | null,
): Promise<KnownStudent[]> {
  try {
    const mefMap = await loadMefSecteurMap();
    const allEleves = await getElevesFromS3();
    const { eleves } = buildElevesPoolForOcrMatching(allEleves, odProfile, mefMap);
    const pool = eleves.length > 0 ? eleves : allEleves;
    return pool
      .map((e) => ({
        ine: e.ine ? normalizeIne(e.ine) : "",
        nom: e.nom ?? "",
        prenom: e.prenom ?? "",
        folderName: resolveEleveFolderName(e),
        normNom: normalize(e.nom ?? ""),
        normPrenom: normalize(e.prenom ?? ""),
      }))
      .filter((s) => s.normNom || s.ine);
  } catch {
    return [];
  }
}

function normalize(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function normalizeIne(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "");
}

type OcrAnalyzeResult = {
  fileName: string;
  oneDriveFolderPath: string | null;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
};

type AnalyzeDocMatchOptions = {
  /**
   * Segment déjà découpé : texte OCR du morceau uniquement.
   * 1 appel Mistral (small), matching local si score fort, nommage sans 2e/3e appel IA.
   */
  segmentMode?: boolean;
  /**
   * Élève déjà identifié au découpage (ancrage identité) : on saute TOUT le matching
   * (INE/nom/shortlist Mistral) et on range directement dans son dossier.
   */
  knownStudent?: { ine?: string; nom: string; prenom: string; folderName: string };
};

/** Valeur exploitable (non vide, non "non_trouvé"). */
function cleanFieldValue(v: unknown): string {
  if (typeof v !== "string") return "";
  const s = v.trim();
  return !s || s === "non_trouvé" ? "" : s;
}

/** Nom de famille : entièrement en majuscules. */
function formatLastName(str: string): string {
  return str.trim().toUpperCase();
}

/** Prénom(s) : première lettre de chaque mot en majuscule, reste en minuscule (gère composés). */
function formatFirstName(str: string): string {
  return str
    .trim()
    .toLowerCase()
    .replace(/(^|[\s\-'])(\p{L})/gu, (_m, sep: string, ch: string) => sep + ch.toUpperCase());
}

function capitalizePhrase(str: string): string {
  const s = str.trim().replace(/\s+/g, " ");
  if (!s) return "";
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function alreadyCovered(haystack: string, needle: string): boolean {
  if (!needle || needle.length < 3) return false;
  const h = haystack
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
  const n = needle
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
  return h.includes(n);
}

/**
 * Titre explicite du document (sans le nom de l'élève) + NOM + Prénom.
 * Ex. "Bulletin scolaire 2ème semestre 2A BERBEZY Juliette"
 *     "Relevé de notes Trimestre 1 Maths DUPONT Marie"
 *     "Carte d'identité MARTIN Paul"
 */
function buildFinalFileName(opts: {
  titre?: unknown;
  type?: unknown;
  detail?: unknown;
  periode?: unknown;
  classe?: unknown;
  nom?: unknown;
  prenom?: unknown;
}): string {
  const titre = capitalizePhrase(cleanFieldValue(opts.titre));
  const typePart = capitalizePhrase(cleanFieldValue(opts.type));
  const detailPart = capitalizePhrase(cleanFieldValue(opts.detail));
  const periodePart = capitalizePhrase(cleanFieldValue(opts.periode));
  const classePart = capitalizePhrase(cleanFieldValue(opts.classe));
  const nomPart = formatLastName(cleanFieldValue(opts.nom));
  const prenomPart = formatFirstName(cleanFieldValue(opts.prenom));

  const head: string[] = [];
  if (titre) {
    head.push(titre);
  } else {
    if (typePart) head.push(typePart);
    if (detailPart && !alreadyCovered(head.join(" "), detailPart)) head.push(detailPart);
    if (periodePart && !alreadyCovered(head.join(" "), periodePart)) head.push(periodePart);
    if (classePart && !alreadyCovered(head.join(" "), classePart)) head.push(classePart);
  }

  // Si le titre IA est trop vague (un seul mot générique), compléter avec période / classe / détail.
  if (titre && titre.split(/\s+/).length <= 2) {
    if (detailPart && !alreadyCovered(head.join(" "), detailPart)) head.push(detailPart);
    if (periodePart && !alreadyCovered(head.join(" "), periodePart)) head.push(periodePart);
    if (classePart && !alreadyCovered(head.join(" "), classePart)) head.push(classePart);
  }

  const raw = [...head, nomPart, prenomPart].filter(Boolean).join(" ").trim();
  if (!raw) return "Document";
  // Limite raisonnable pour OneDrive / explorateurs
  const clipped = raw.length > 180 ? raw.slice(0, 180).trim() : raw;
  return clipped.replace(/[<>:"/\\|?*]/g, "_").replace(/_+/g, "_").replace(/^_|_$/g, "");
}

export async function analyzeDocMatchEleve(
  text: string,
  odProfile: OneDriveUserProfile | null,
  trace?: OcrTraceCtx,
  options?: AnalyzeDocMatchOptions,
): Promise<OcrAnalyzeResult> {
  const segmentMode = Boolean(options?.segmentMode);
  const extractModel = segmentMode ? "mistral-small-latest" : "mistral-medium";
  ocrTraceCtx(trace, "classify", "start", "analyzeDocMatchEleve", {
    textChars: text.length,
    odSecteur: odProfile?.secteur ?? null,
  });

  const mistralKey = await getMistralApiKey();
  if (!mistralKey) {
    ocrTraceCtx(trace, "classify", "no-key", "MISTRAL_API_KEY manquante", undefined, "error");
    throw new Error("Service IA non configuré (MISTRAL_API_KEY).");
  }

  const extractionPrompt = `
      Analyse ce document scolaire ou administratif concernant un élève
      (bulletin, relevé, carte d'identité, certificat, attestation, diplôme, courrier, CAF, mutuelle…).
      Extrais UNIQUEMENT ce qui est clairement présent. Ne devine JAMAIS.

      Distingue bien l'ÉLÈVE (enfant scolarisé) des PARENTS / ASSURÉS / TITULAIRES du document.

      - titre_document : titre EXPLICITE pour nommer le fichier, SANS nom/prénom.
        Ex. "Bulletin scolaire 2ème semestre 2A", "Carte d'identité", "Attestation d'assurance scolaire".
        Interdit : "Document", "Fichier", "PDF".
      - type : Bulletin, Relevé de notes, Carte d'identité, Certificat, Attestation, Diplôme, Courrier, Autre
      - detail : précision utile sinon "non_trouvé"
      - origine : "interne" si document de l'établissement (bulletin, relevé, certificat de scolarité, Pronote, Charlemagne),
        "externe" si CNI, passeport, CAF, mutuelle, médecin, assurance, organisme extérieur.
      - eleve : identité de l'enfant (pas du parent)
      - parents : titulaires / responsables s'ils apparaissent, sinon []
      - INE, date de naissance, classe, période si présents

      Texte :
      ---
      ${text}
      ---
      JSON uniquement :
      {
        "titre_document": "...",
        "type": "...",
        "detail": "...",
        "origine": "interne",
        "nom": "...",
        "prénom": "...",
        "ine": "...",
        "date_naissance": "...",
        "classe": "...",
        "période": "...",
        "parents": [{"nom": "...", "prénom": "..."}]
      }
      Si un champ est absent : "non_trouvé" (sauf parents = []).
    `;

  ocrTraceCtx(trace, "classify", "mistral-extract", "appel Mistral extraction", {
    model: extractModel,
    segmentMode,
    textChars: text.length,
  });

  const extractionResponse = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${mistralKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: extractModel,
      messages: [{ role: "user", content: extractionPrompt }],
      temperature: 0,
      response_format: { type: "json_object" },
    }),
  });
  if (!extractionResponse.ok) {
    const errText = await extractionResponse.text();
    ocrTraceCtx(trace, "classify", "mistral-extract-fail", "Mistral extraction HTTP erreur", {
      status: extractionResponse.status,
      body: errText.slice(0, 300),
    }, "error");
    throw new Error(`Erreur Mistral extraction: ${errText}`);
  }

  const extractionData = await extractionResponse.json();
  let extractionResult = extractionData.choices?.[0]?.message?.content || "";
  extractionResult = extractionResult.trim().replace(/`{3}json/gi, "").replace(/`{3}/g, "");
  extractionResult = extractionResult.replace(/\n/g, " ").trim();
  extractionResult = extractionResult
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/\/\/.*/g, "")
    .replace(/,\s*\*\(.*?\)\*/g, "");
  const jsonStartIndex = extractionResult.indexOf("{");
  const jsonEndIndex = extractionResult.lastIndexOf("}");
  if (jsonStartIndex === -1 || jsonEndIndex === -1) {
    ocrTraceCtx(trace, "classify", "extract-parse-fail", "pas de JSON dans réponse Mistral", {
      raw: extractionResult.slice(0, 200),
    }, "error");
    throw new Error("Aucun JSON trouvé dans la réponse Mistral");
  }
  const cleanJson = extractionResult.substring(jsonStartIndex, jsonEndIndex + 1);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let extracted: any;
  try {
    extracted = JSON.parse(cleanJson);
  } catch (parseError) {
    const superCleanJson = cleanJson.replace(/\s+/g, " ").replace(/,\s*}/g, "}").replace(/,\s*]/g, "]");
    try {
      extracted = JSON.parse(superCleanJson);
    } catch {
      throw new Error(`JSON invalide après extraction: ${String(parseError)}`);
    }
  }

  ocrTraceCtx(trace, "classify", "extracted", "champs extraits du document", {
    titre_document: extracted.titre_document,
    type: extracted.type,
    detail: extracted.detail,
    nom: extracted.nom,
    prenom: extracted.prénom,
    ine: extracted.ine,
    classe: extracted.classe,
    periode: extracted.période,
  });

  let oneDriveFolderPath: string | null = null;
  let matchedEleve: { ine: string; nom: string; prenom: string; folderName: string } | null = null;
  let matchCandidates: EleveMatchCandidateView[] = [];
  let matchDebug: Record<string, unknown> = {};

  const knownStudent = options?.knownStudent;
  if (knownStudent && odProfile) {
    // Élève déjà identifié au découpage : aucun appel IA de matching.
    matchedEleve = {
      ine: knownStudent.ine ?? "",
      nom: knownStudent.nom,
      prenom: knownStudent.prenom,
      folderName: knownStudent.folderName,
    };
    oneDriveFolderPath = oneDrivePathForEleve(odProfile.basePath, resolveEleveFolderName(matchedEleve));
    matchDebug = { matchedBy: "segmentation-identity", folderName: knownStudent.folderName, decision: "auto" };
    ocrTraceCtx(trace, "classify", "match-prematched", "élève fourni par le découpage (pas de matching IA)", {
      folderName: knownStudent.folderName,
    });
  } else {
  try {
    const mefMap = await loadMefSecteurMap();
    const allEleves = await getElevesFromS3();
    const { eleves, secteurFilterApplied } = buildElevesPoolForOcrMatching(allEleves, odProfile, mefMap);
    const pool = eleves.length > 0 ? eleves : allEleves;
    const extractedIdentity: ExtractedIdentity = {
      nom: cleanFieldValue(extracted.nom),
      prenom: cleanFieldValue(extracted.prénom ?? extracted.prenom),
      ine: cleanFieldValue(extracted.ine),
      classe: cleanFieldValue(extracted.classe),
      dateNaissance: normalizeEleveDateNaissance(cleanFieldValue(extracted.date_naissance)),
      origine:
        extracted.origine === "interne" || extracted.origine === "externe" ? extracted.origine : undefined,
      parents: Array.isArray(extracted.parents)
        ? extracted.parents
            .map((p: { nom?: string; prénom?: string; prenom?: string }) => ({
              nom: cleanFieldValue(p?.nom),
              prenom: cleanFieldValue(p?.prénom ?? p?.prenom),
            }))
            .filter((p: { nom?: string; prenom?: string }) => p.nom || p.prenom)
        : [],
    };

    let decision = matchEleveFromDocument({
      text,
      eleves: pool,
      extracted: extractedIdentity,
    });
    if (decision.decision === "none" && pool.length !== allEleves.length) {
      decision = matchEleveFromDocument({
        text,
        eleves: allEleves,
        extracted: extractedIdentity,
      });
    }

    const withPaths = (list: EleveMatchCandidateView[]): EleveMatchCandidateView[] =>
      list.map((c) => ({
        ...c,
        folderPath: odProfile ? oneDrivePathForEleve(odProfile.basePath, c.folderName) : undefined,
      }));

    matchCandidates = withPaths(decision.candidates);

    if (decision.decision === "review" && !segmentMode && decision.candidates.length > 0 && mistralKey) {
      const shortlistDescription = decision.candidates
        .map(
          (c, idx) =>
            `${idx + 1}. INE: ${c.ine || "?"}, Nom: ${c.nom}, Prénom: ${c.prenom}, Classe: ${c.classe || "?"}, score=${c.score}, via=${c.matchedBy}`,
        )
        .join("\n");
      const selectionPrompt = `Tu valides un classement de document scolaire. Sois strict.

Texte OCR :
---
${text.slice(0, 3000)}
---

Identité extraite : élève ${extractedIdentity.nom || "?"} ${extractedIdentity.prenom || "?"} / classe ${extractedIdentity.classe || "?"} / INE ${extractedIdentity.ine || "?"} / origine ${decision.origin}

Candidats :
${shortlistDescription}

Règles :
- Ne choisis un index que si NOM + PRÉNOM correspondent vraiment à l'élève du document (pas un parent).
- En cas de doute, réponds 0.
JSON uniquement : {"index":0,"confidence":0}`;
      try {
        const selectionResponse = await fetch("https://api.mistral.ai/v1/chat/completions", {
          method: "POST",
          headers: {
            Authorization: `Bearer ${mistralKey}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            model: "mistral-medium",
            messages: [{ role: "user", content: selectionPrompt }],
            temperature: 0,
            response_format: { type: "json_object" },
          }),
        });
        if (selectionResponse.ok) {
          const selectionData = await selectionResponse.json();
          const content = (selectionData.choices?.[0]?.message?.content || "").trim();
          const parsed = JSON.parse(content) as { index?: number; confidence?: number };
          const selectedIndex = Number(parsed.index ?? 0);
          const confidence = Number(parsed.confidence ?? 0);
          if (selectedIndex > 0 && selectedIndex <= decision.candidates.length && confidence >= 0.9) {
            const chosen = decision.candidates[selectedIndex - 1];
            const found =
              allEleves.find((e) => e.folderName === chosen.folderName) ||
              allEleves.find(
                (e) =>
                  e.nom.toLowerCase() === chosen.nom.toLowerCase() &&
                  e.prenom.toLowerCase() === chosen.prenom.toLowerCase(),
              );
            if (found) {
              decision = {
                ...decision,
                decision: "auto",
                eleve: found,
                confidence,
                matchedBy: "mistral_shortlist",
                reason: "mistral_valide_shortlist",
              };
            }
          }
        }
      } catch {
        ocrTraceCtx(trace, "classify", "match-name-http-fail", "arbitrage Mistral shortlist ignoré", undefined, "warn");
      }
    }

    if (decision.decision === "auto" && decision.eleve) {
      matchedEleve = decision.eleve;
      ocrTraceCtx(trace, "classify", "match-auto", "élève retenu par le moteur", {
        matchedBy: decision.matchedBy,
        reason: decision.reason,
        origin: decision.origin,
        folderName: decision.eleve.folderName,
      });
    } else {
      ocrTraceCtx(trace, "classify", "match-pending", "pas d'auto-match", {
        decision: decision.decision,
        reason: decision.reason,
        origin: decision.origin,
        candidates: decision.candidates.length,
      }, "warn");
    }

    matchDebug = {
      totalEleves: allEleves.length,
      elevesInPool: pool.length,
      secteurFilterApplied,
      secteur: odProfile?.secteur ?? null,
      secteurLabel: odProfile?.label ?? null,
      mefCodesInTable: mefMap.size,
      hasOneDriveProfile: Boolean(odProfile),
      ineProvided: Boolean(extractedIdentity.ine),
      extractedClass: extractedIdentity.classe || null,
      origin: decision.origin,
      decision: decision.decision,
      matchedBy: decision.matchedBy,
      reason: decision.reason,
      confidence: decision.confidence,
      candidates: matchCandidates,
    };
    if (matchedEleve && odProfile) {
      oneDriveFolderPath = oneDrivePathForEleve(odProfile.basePath, resolveEleveFolderName(matchedEleve));
    }
  } catch (e) {
    ocrTraceCtx(trace, "classify", "match-error", "erreur matching élève", {
      error: e instanceof Error ? e.message : String(e),
    }, "error");
    matchDebug = { ...matchDebug, matchingError: e instanceof Error ? e.message : String(e) };
  }
  }

  ocrTraceCtx(trace, "classify", "match-summary", "résumé matching", matchDebug);

  // Titre explicite (tous types de docs) + NOM + Prénom.
  const fileName = buildFinalFileName({
    titre: extracted.titre_document,
    type: extracted.type,
    detail: extracted.detail,
    periode: extracted.période,
    classe: extracted.classe,
    nom: matchedEleve?.nom ?? extracted.nom,
    prenom: matchedEleve?.prenom ?? extracted.prénom,
  });
  ocrTraceCtx(trace, "classify", "naming", "nom de fichier (titre explicite + NOM + Prénom)", {
    fileName,
    titre: cleanFieldValue(extracted.titre_document) || null,
    type: cleanFieldValue(extracted.type) || null,
    detail: cleanFieldValue(extracted.detail) || null,
    periode: cleanFieldValue(extracted.période) || null,
    classe: cleanFieldValue(extracted.classe) || null,
    nomSource: matchedEleve?.nom ? "eleve" : "extrait",
    prenomSource: matchedEleve?.prenom ? "eleve" : "extrait",
  });

  ocrTraceCtx(trace, "classify", "done", "analyse terminée", {
    fileName: fileName || null,
    oneDriveFolderPath,
    matchedEleve: matchedEleve
      ? { nom: matchedEleve.nom, prenom: matchedEleve.prenom, folderName: matchedEleve.folderName }
      : null,
  });

  return {
    ...extracted,
    eleve: {
      nom: extracted.nom !== "non_trouvé" ? extracted.nom : null,
      prénom: extracted.prénom !== "non_trouvé" ? extracted.prénom : null,
      classe: extracted.classe !== "non_trouvé" ? extracted.classe : null,
      ine: extracted.ine !== "non_trouvé" ? extracted.ine : null,
      date_naissance: extracted.date_naissance !== "non_trouvé" ? extracted.date_naissance : null,
    },
    fileName,
    rawExtraction: extracted,
    oneDriveFolderPath,
    matchedEleve,
    matchCandidates,
    matchDebug,
  };
}
