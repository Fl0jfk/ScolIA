import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import { resolveEleveFolderName } from "@/app/lib/eleves-config";
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

function levenshtein(a: string, b: string): number {
  if (a === b) return 0;
  if (!a.length) return b.length;
  if (!b.length) return a.length;
  let prev = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const curr = [i];
    for (let j = 1; j <= b.length; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      curr[j] = Math.min(curr[j - 1] + 1, prev[j] + 1, prev[j - 1] + cost);
    }
    prev = curr;
  }
  return prev[b.length];
}

/**
 * Proximité 0→1 entre deux chaînes (1 = identiques).
 * Tolérance OCR stricte : max 1 caractère différent pour les noms courts,
 * 1-2 pour les plus longs. Seuil interne relevé à 0.88.
 */
function closeness(a: string, b: string): number {
  if (!a || !b) return 0;
  if (a === b) return 1;
  // Inclusion seulement si l'un est préfixe/suffixe ET assez long (évite "AR" dans "MARTIN")
  const shorter = a.length < b.length ? a : b;
  const longer = a.length < b.length ? b : a;
  if (shorter.length >= 4 && longer.startsWith(shorter)) return 0.88;
  const maxLen = Math.max(a.length, b.length);
  if (maxLen === 0) return 0;
  // Tolérance : 1 erreur maxi pour ≤6 chars, 2 erreurs maxi pour 7-12 chars, 3 pour les plus longs
  const dist = levenshtein(a, b);
  const maxAllowed = maxLen <= 6 ? 1 : maxLen <= 12 ? 2 : 3;
  if (dist > maxAllowed) return 0;
  return 1 - dist / maxLen;
}

/**
 * Score de similarité nom/prénom — strict.
 * Exige que NOM et PRÉNOM soient tous les deux présents et matchent chacun.
 * Score max = 4.0 (2 pts nom + 2 pts prénom). Seuil de match auto : >= 3.2.
 * Si un seul champ est disponible (sans l'autre), on ne score pas : trop risqué.
 */
function nameSimilarity(aNom: string, aPrenom: string, bNom: string, bPrenom: string): number {
  const an = normalize(aNom);
  const ap = normalize(aPrenom);
  const bn = normalize(bNom);
  const bp = normalize(bPrenom);

  // Exige que les deux champs soient non vides des deux côtés
  const hasNomBothSides = Boolean(an && bn);
  const hasPrenomBothSides = Boolean(ap && bp);
  if (!hasNomBothSides || !hasPrenomBothSides) return 0;

  const cNom = closeness(an, bn);
  const cPrenom = closeness(ap, bp);

  // Les deux doivent atteindre le seuil : un seul qui matche ne suffit pas
  if (cNom < 0.88 || cPrenom < 0.88) {
    // Tenter inversion nom/prénom (bulletins qui inversent parfois)
    const cNomInv = closeness(an, bp);
    const cPrenomInv = closeness(ap, bn);
    if (cNomInv >= 0.88 && cPrenomInv >= 0.88) {
      return (cNomInv + cPrenomInv); // score ≤ 2.0 → jamais auto-matché sans 2e appel IA
    }
    return 0;
  }

  return 2 * cNom + 2 * cPrenom;
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
      Analyse ce document scolaire (bulletin, relevé de notes, carte d'identité, certificat, attestation, diplôme, courrier, etc.)
      et extrais UNIQUEMENT les informations clairement présentes dans le texte.

      - titre_document : titre EXPLICITE et autonome pour nommer le fichier, SANS le nom/prénom de l'élève.
        Doit préciser de quoi il s'agit (pas un mot seul trop vague).
        Exemples selon le document :
        · "Bulletin scolaire 2ème semestre 2A"
        · "Relevé de notes Trimestre 1"
        · "Carte d'identité"
        · "Certificat de scolarité 2025-2026"
        · "Attestation d'assurance scolaire"
        · "Diplôme brevet 2024"
        Interdit : un seul mot trop générique du type "Document", "Fichier", "PDF".
      - type : catégorie courte (Bulletin, Relevé de notes, Carte d'identité, Certificat, Attestation, Diplôme, Courrier, Autre…)
      - detail : précision utile si pas déjà dans le titre (matière, organisme, année, motif…) sinon "non_trouvé"
      - Nom de famille de l'élève
      - Prénom de l'élève
      - INE (si présent)
      - Date de naissance (si présente)
      - Classe ou niveau tel qu'écrit (ex. "2A", "2GT-EO")
      - Période (ex. "2ème semestre", "Trimestre 1", "Année 2025-2026") si indiquée

      Si une information n'est PAS présente, écris exactement "non_trouvé" pour ce champ.
      Ne devine JAMAIS, n'invente JAMAIS.
      IMPORTANT : Réponds UNIQUEMENT avec du JSON valide, sans markdown ni commentaire.
      Texte du document :
      ---
      ${text}
      ---
      Format de réponse (JSON uniquement) :
      {
        "titre_document": "...",
        "type": "...",
        "detail": "...",
        "nom": "...",
        "prénom": "...",
        "ine": "...",
        "date_naissance": "...",
        "classe": "...",
        "période": "..."
      }
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
    matchDebug = { matchedBy: "segmentation-identity", folderName: knownStudent.folderName };
    ocrTraceCtx(trace, "classify", "match-prematched", "élève fourni par le découpage (pas de matching IA)", {
      folderName: knownStudent.folderName,
    });
  } else {
  try {
    const mefMap = await loadMefSecteurMap();
    const allEleves = await getElevesFromS3();
    const { eleves, secteurFilterApplied } = buildElevesPoolForOcrMatching(allEleves, odProfile, mefMap);
    matchDebug = {
      totalEleves: allEleves.length,
      elevesInPool: eleves.length,
      secteurFilterApplied,
      secteur: odProfile?.secteur ?? null,
      secteurLabel: odProfile?.label ?? null,
      mefCodesInTable: mefMap.size,
      hasOneDriveProfile: Boolean(odProfile),
    };
    const { ine, nom, prénom } = extracted;
    const hasNom = Boolean(nom && nom !== "non_trouvé" && normalize(nom).length >= 2);
    const hasPrenom = Boolean(prénom && prénom !== "non_trouvé" && normalize(prénom).length >= 2);
    const ineNorm = ine && ine !== "non_trouvé" ? normalizeIne(ine) : "";
    const elevesWithIne = allEleves.filter((e) => e.ine && normalizeIne(e.ine)).length;

    // 1) INE = identifiant national unique → recherché sur TOUTE la liste,
    //    indépendamment du filtre secteur (sinon un élève hors pool est perdu).
    let ineMatched = false;
    if (ineNorm) {
      const found = allEleves.find((e) => e.ine && normalizeIne(e.ine) === ineNorm);
      if (found) {
        matchedEleve = found;
        ineMatched = true;
        ocrTraceCtx(trace, "classify", "match-ine", "élève trouvé par INE", {
          ine: ineNorm,
          folderName: found.folderName,
        });
      } else {
        ocrTraceCtx(trace, "classify", "match-ine-miss", "INE extrait mais absent de eleves.json", {
          ine: ineNorm,
        }, "warn");
      }
    }

    // 2) Nom + prénom : les DEUX champs doivent être présents et dépasser le seuil.
    //    Si un seul champ est disponible, on ne matche pas automatiquement (trop risqué).
    let bestNameScore = 0;
    if (!matchedEleve && hasNom && hasPrenom) {
      const scoreList = (list: typeof eleves) =>
        list
          .map((e) => ({
            eleve: e,
            score: nameSimilarity(nom, prénom, e.nom, e.prenom),
          }))
          .filter((s) => s.score > 0)
          .sort((a, b) => b.score - a.score);

      let scored = scoreList(eleves).slice(0, 3);
      if (scored.length === 0 && eleves.length !== allEleves.length) {
        scored = scoreList(allEleves).slice(0, 3);
      }
      bestNameScore = scored[0]?.score ?? 0;
      ocrTraceCtx(trace, "classify", "match-name-shortlist", "shortlist nom/prénom", {
        candidates: scored.map((s) => ({
          nom: s.eleve.nom,
          prenom: s.eleve.prenom,
          score: Math.round(s.score * 100) / 100,
        })),
      });

      // Auto-match (sans 2e appel IA) : score >= 3.2/4.0 = les deux champs matchent fortement
      const AUTO_MATCH_THRESHOLD = 3.2;
      if (scored.length > 0 && scored[0].score >= AUTO_MATCH_THRESHOLD) {
        matchedEleve = scored[0].eleve;
        ocrTraceCtx(trace, "classify", "match-name-auto", "élève retenu par score strict (sans 2e appel Mistral)", {
          score: Math.round(scored[0].score * 100) / 100,
          folderName: matchedEleve.folderName,
        });
      } else if (scored.length > 0 && scored[0].score > 0) {
        // Score insuffisant pour l'auto-match : on soumet à Mistral AVEC instruction stricte
        const shortlist = scored.map((s) => s.eleve);
        const shortlistDescription = shortlist
          .map((e, idx) => `${idx + 1}. INE: ${e.ine || "?"}, Nom: ${e.nom}, Prénom: ${e.prenom}`)
          .join("\n");
        const selectionPrompt = `Tu es un système de classement de documents scolaires. Tu DOIS être très strict.

Texte OCR du document :
---
${text.slice(0, 3000)}
---

Élèves candidats (shortlist) :
${shortlistDescription}

Règles STRICTES :
- Le NOM DE FAMILLE et le PRÉNOM extraits du document doivent correspondre exactement (ou quasi-exactement avec 1 caractère d'écart max) à ceux d'un élève de la liste.
- Une simple similitude partielle (même début de nom, même lettre initiale) ne suffit PAS.
- Si tu as le moindre doute, réponds 0.
- Si aucun élève ne correspond avec certitude, réponds 0.
- Ne tente pas de deviner.

Réponds UNIQUEMENT avec ce JSON valide :
{"index": 0}`;

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
          let selectedIndex = 0;
          try {
            selectedIndex = JSON.parse(content).index ?? 0;
          } catch {
            selectedIndex = 0;
          }
          if (selectedIndex > 0 && selectedIndex <= shortlist.length) {
            matchedEleve = shortlist[selectedIndex - 1];
            ocrTraceCtx(trace, "classify", "match-name-mistral", "élève choisi par Mistral shortlist", {
              selectedIndex,
              folderName: matchedEleve.folderName,
            });
          } else {
            ocrTraceCtx(trace, "classify", "match-name-miss", "Mistral n'a pas retenu d'élève (doute ou aucun match)", {
              selectedIndex,
            }, "warn");
          }
        } else {
          ocrTraceCtx(trace, "classify", "match-name-http-fail", "appel Mistral sélection élève échoué", {
            status: selectionResponse.status,
          }, "warn");
        }
      } else {
        ocrTraceCtx(trace, "classify", "match-name-no-candidate", "aucun candidat avec nom+prénom valides", {
          hasNom,
          hasPrenom,
        }, "warn");
      }
    } else if (!matchedEleve && (hasNom || hasPrenom)) {
      ocrTraceCtx(trace, "classify", "match-name-skip", "matching nom/prénom ignoré : un seul champ disponible", {
        hasNom,
        hasPrenom,
        nom: hasNom ? nom : null,
        prenom: hasPrenom ? prénom : null,
      }, "warn");
    }
    matchDebug = {
      ...matchDebug,
      ineProvided: Boolean(ineNorm),
      elevesWithIne,
      ineMatched,
      bestNameScore: Math.round(bestNameScore * 100) / 100,
      matchedBy: matchedEleve ? (ineMatched ? "ine" : "name") : null,
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
    matchDebug,
  };
}
