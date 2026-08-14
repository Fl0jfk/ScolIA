import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  collectEleveParentEmails,
  normalizeParentEmail,
} from "@/app/lib/eleves-parent-emails";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import {
  inferSecteurFromFolderName,
  resolveEleveSecteur,
  type Secteur,
} from "@/app/lib/onedrive-eleves";

export type RequestEleveHit = {
  ine: string;
  nom: string;
  prenom: string;
  classe?: string;
  secteur: Secteur | null;
  matchVia: "nom" | "email_parent";
};

export type RequestEleveContext = {
  hits: RequestEleveHit[];
  /** Secteur majoritaire parmi les hits (si cohérent). */
  suggestedSecteur: Secteur | null;
  /** Indices texte détectés (lycée / collège / école). */
  textSecteurHints: Secteur[];
  summary: string;
};

function norm(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9@.\s]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function resolveSecteurForRequest(
  eleve: EleveConfig,
  mefMap: Map<string, Secteur> | null,
): Secteur | null {
  const fromRegistry = resolveEleveSecteur(eleve, mefMap);
  if (fromRegistry) return fromRegistry;
  if (eleve.classe) {
    const fromClasse = inferSecteurFromFolderName(eleve.classe);
    if (fromClasse) return fromClasse;
  }
  return inferSecteurFromFolderName(
    `${eleve.folderName || ""} ${eleve.classe || ""} ${eleve.mef || ""}`,
  );
}

function detectTextSecteurHints(textNorm: string): Secteur[] {
  const hints: Secteur[] = [];
  if (/\b(lycee|lyc[eé]en|terminale|\b2nde\b|\b2de\b|seconde|\b1re\b|premiere)\b/.test(textNorm)) {
    hints.push("lycee");
  }
  if (
    /\b(college|collegien|\b6e\b|\b5e\b|\b4e\b|\b3e\b|sixieme|cinquieme|quatrieme|troisieme)\b/.test(
      textNorm,
    )
  ) {
    hints.push("college");
  }
  if (
    /\b(ecole|elementaire|primaire|\bcp\b|\bce1\b|\bce2\b|\bcm1\b|\bcm2\b|maternelle)\b/.test(
      textNorm,
    )
  ) {
    hints.push("ecole");
  }
  return [...new Set(hints)];
}

function nameAppearsInText(textNorm: string, nom: string, prenom: string): boolean {
  const n = norm(nom);
  const p = norm(prenom);
  if (!n || !p || n.length < 2 || p.length < 2) return false;
  const a = `${p} ${n}`;
  const b = `${n} ${p}`;
  return textNorm.includes(a) || textNorm.includes(b);
}

function majoritySecteur(hits: RequestEleveHit[]): Secteur | null {
  const counts: Record<Secteur, number> = { ecole: 0, college: 0, lycee: 0 };
  for (const h of hits) {
    if (h.secteur) counts[h.secteur] += 1;
  }
  const ordered = (Object.entries(counts) as [Secteur, number][]).sort((a, b) => b[1] - a[1]);
  if (!ordered[0] || ordered[0][1] === 0) return null;
  if (ordered[1] && ordered[1][1] === ordered[0][1]) return null; // tie → pas de consensus
  return ordered[0][0];
}

/**
 * Cherche dans eleves.json des élèves / e-mails parents mentionnés dans la demande,
 * pour déduire le cycle (école / collège / lycée) à rattacher.
 */
export async function buildRequestEleveContext(
  subject: string,
  description: string,
): Promise<RequestEleveContext> {
  const text = `${subject}\n${description}`;
  const textNorm = norm(text);
  const textSecteurHints = detectTextSecteurHints(textNorm);

  let eleves: EleveConfig[] = [];
  try {
    eleves = await loadElevesRegistry();
  } catch {
    eleves = [];
  }

  const mefMap = await loadMefSecteurMap().catch(() => null);
  const hits: RequestEleveHit[] = [];
  const seen = new Set<string>();

  // E-mails parents présents dans le texte
  const emailMatches = text.toLowerCase().match(/[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/g) || [];
  for (const rawEmail of emailMatches) {
    const key = normalizeParentEmail(rawEmail);
    for (const e of eleves) {
      if (!collectEleveParentEmails(e).includes(key)) continue;
      const id = e.ine || `${e.nom}|${e.prenom}`;
      if (seen.has(id)) continue;
      seen.add(id);
      hits.push({
        ine: e.ine || "",
        nom: e.nom,
        prenom: e.prenom,
        classe: e.classe,
        secteur: resolveSecteurForRequest(e, mefMap),
        matchVia: "email_parent",
      });
    }
  }

  // Noms d'élèves (prénom + nom) dans le texte — limité pour perf
  for (const e of eleves) {
    if (hits.length >= 8) break;
    if (!nameAppearsInText(textNorm, e.nom, e.prenom)) continue;
    const id = e.ine || `${e.nom}|${e.prenom}`;
    if (seen.has(id)) continue;
    seen.add(id);
    hits.push({
      ine: e.ine || "",
      nom: e.nom,
      prenom: e.prenom,
      classe: e.classe,
      secteur: resolveSecteurForRequest(e, mefMap),
      matchVia: "nom",
    });
  }

  const suggestedSecteur =
    majoritySecteur(hits) ||
    (textSecteurHints.length === 1 ? textSecteurHints[0]! : null);

  const parts: string[] = [];
  if (hits.length > 0) {
    parts.push(
      `Élève(s) reconnu(s) : ${hits
        .map(
          (h) =>
            `${h.prenom} ${h.nom}${h.classe ? ` (${h.classe})` : ""}${
              h.secteur ? ` → ${h.secteur}` : ""
            }`,
        )
        .join(" ; ")}`,
    );
  }
  if (suggestedSecteur) {
    parts.push(`Cycle suggéré : ${suggestedSecteur}`);
  } else if (textSecteurHints.length > 1) {
    parts.push(`Indices texte ambigus : ${textSecteurHints.join(", ")}`);
  }

  return {
    hits,
    suggestedSecteur,
    textSecteurHints,
    summary: parts.join(". ") || "Aucun élève / cycle détecté dans eleves.json.",
  };
}

/** Tags / mots qui rattachent une personne à un cycle. */
const SECTEUR_TAG_ALIASES: Record<Secteur, string[]> = {
  ecole: ["ecole", "école", "elementaire", "élémentaire", "primaire", "maternelle", "secretariat ecole", "secrétariat école", "admin ecole", "admin école"],
  college: ["college", "collège", "collegien", "secretariat college", "secrétariat collège", "admin college", "admin collège"],
  lycee: ["lycee", "lycée", "lyceen", "lycéen", "secretariat lycee", "secrétariat lycée", "admin lycee", "admin lycée"],
};

export function tagsMatchSecteur(tags: string[], secteur: Secteur): string[] {
  const aliases = SECTEUR_TAG_ALIASES[secteur].map(norm);
  const matched: string[] = [];
  for (const tag of tags) {
    const t = norm(tag);
    if (!t) continue;
    if (aliases.some((a) => t === a || t.includes(a) || a.includes(t))) {
      matched.push(tag);
    }
  }
  return matched;
}
