import { niveauFromClasse } from "@/app/lib/internat-level";
import { isFdNiveau } from "@/app/lib/fiches-dialogue-templates";

/**
 * Une cible est soit un niveau exact (« 6e », « 3e », « 2nde », « 1re », « Tle ») qui couvre
 * toutes ses divisions (6A…6F), soit un nom de classe (« 3E », « 1 A ») comparé sans casse ni espaces.
 * « 3e » (niveau) et « 3E » (classe E de 3ᵉ) sont deux cibles différentes.
 */
export function matchFdClasse(classe: string, cibles: string[]): boolean {
  if (!cibles.length) return true;
  const squash = (s: string) => s.trim().toLowerCase().replace(/[\s._-]+/g, "");
  const c = squash(classe);
  return cibles.some((t) => {
    const raw = t.trim();
    if (!raw) return false;
    if (isFdNiveau(raw)) return niveauFromClasse(classe) === raw;
    return c === squash(raw);
  });
}

/** Niveaux distincts couverts par une liste de cibles (niveaux ou noms de classes). */
export function niveauxDesCibles(cibles: string[]): string[] {
  const out = new Set<string>();
  for (const c of cibles) {
    const raw = c.trim();
    if (!raw) continue;
    const niv = isFdNiveau(raw) ? raw : niveauFromClasse(raw);
    if (niv) out.add(niv);
  }
  return [...out];
}
