/**
 * Famille d'un interne : lue dans le dossier élève (foyers), jamais recopiée dans la fiche internat.
 * Module pur (client + serveur + tests).
 */
import { normalizeInternatPersonPart } from "@/app/lib/internat-dedupe";

export type InternatFamilleResponsable = {
  id: string;
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  lien: string | null;
  autoriteParentale: boolean;
  contactUrgence: boolean;
  peutRecuperer: boolean;
  rang: number;
};

export type InternatFamilleFoyer = {
  id: string;
  label: string;
  relation: string;
  adresse: string | null;
  codePostal: string | null;
  ville: string | null;
  responsables: InternatFamilleResponsable[];
};

export type InternatFamille = {
  /** null = élève introuvable (ou homonymes impossibles à départager) dans les dossiers. */
  eleveId: string | null;
  foyers: InternatFamilleFoyer[];
};

export type InternatEleveCandidate = {
  id: string;
  ine: string | null;
  nom: string;
  prenom: string;
  folderName: string | null;
  classe: string | null;
  status: string | null;
};

type InternatRefLike = {
  classe?: string;
  eleveRef: { ine?: string; folderName?: string; nom: string; prenom: string };
};

function normIne(raw: string | null | undefined): string {
  return String(raw ?? "").trim().toUpperCase();
}

function nameBag(nom: string, prenom: string): string {
  return `${normalizeInternatPersonPart(nom)} ${normalizeInternatPersonPart(prenom)}`
    .split(" ")
    .filter(Boolean)
    .sort()
    .join("|");
}

const isFormer = (e: InternatEleveCandidate) => e.status === "ancien" || e.status === "archive";

/**
 * Rattache un interne à UN dossier élève : par l'INE, sinon par nom + prénom, la classe départageant.
 * En cas de doute, on ne rattache rien : afficher la famille (ou écrire aux parents) d'un autre
 * élève serait pire que ne rien afficher.
 */
export function matchEleveForInternat(
  student: InternatRefLike,
  eleves: InternatEleveCandidate[],
): InternatEleveCandidate | null {
  const ine = normIne(student.eleveRef.ine);
  const classe = normalizeInternatPersonPart(student.classe);
  const folder = normalizeInternatPersonPart(student.eleveRef.folderName);
  const nom = normalizeInternatPersonPart(student.eleveRef.nom);
  const prenom = normalizeInternatPersonPart(student.eleveRef.prenom);

  const sameClasse = (e: InternatEleveCandidate) =>
    !!classe && normalizeInternatPersonPart(e.classe) === classe;

  /** Un seul candidat restant, après avoir écarté les anciens puis comparé le nom de dossier. */
  const only = (list: InternatEleveCandidate[]): InternatEleveCandidate | null => {
    const current = list.filter((e) => !isFormer(e));
    const pool = current.length ? current : list;
    if (pool.length === 1) return pool[0]!;
    const byFolder = folder
      ? pool.filter((e) => normalizeInternatPersonPart(e.folderName) === folder)
      : [];
    return byFolder.length === 1 ? byFolder[0]! : null;
  };

  if (ine) {
    const byIne = eleves.filter((e) => normIne(e.ine) === ine);
    if (byIne.length) {
      const inClasse = byIne.filter(sameClasse);
      return only(inClasse.length ? inClasse : byIne);
    }
  }

  // Deux INE renseignés et différents = deux élèves différents.
  const compatible = ine ? eleves.filter((e) => !normIne(e.ine)) : eleves;
  const bag = nameBag(student.eleveRef.nom, student.eleveRef.prenom);
  if (!bag) return null;
  const sameName = compatible.filter((e) => nameBag(e.nom, e.prenom) === bag);

  const inClasse = sameName.filter(sameClasse);
  if (inClasse.length) return only(inClasse);

  // Sans classe concordante : élève présent, nom et prénom dans le même ordre.
  return only(
    sameName.filter(
      (e) =>
        !isFormer(e) &&
        normalizeInternatPersonPart(e.nom) === nom &&
        normalizeInternatPersonPart(e.prenom) === prenom,
    ),
  );
}

/**
 * Adresses des responsables ayant l'autorité parentale (2 au plus, format des sorties).
 * Les responsables de rang 1 de chaque foyer passent avant les rangs 2 (parents séparés).
 */
export function parentEmailsFromFamille(famille: InternatFamille): string[] {
  const rows = famille.foyers.flatMap((f, foyerIndex) =>
    f.responsables.map((r) => ({ r, foyerIndex })),
  );
  rows.sort((a, b) => a.r.rang - b.r.rang || a.foyerIndex - b.foyerIndex);
  const seen = new Set<string>();
  const out: string[] = [];
  for (const { r } of rows) {
    const email = String(r.email ?? "").trim();
    if (!r.autoriteParentale || !email.includes("@")) continue;
    const key = email.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(email);
  }
  return out.slice(0, 2);
}
