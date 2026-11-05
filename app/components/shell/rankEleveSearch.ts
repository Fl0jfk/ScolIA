export type RankableEleve = {
  nom: string;
  prenom: string;
  classe?: string | null;
};

const fold = (s: string) =>
  s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .trim();

/**
 * Filtre et trie les élèves pour une recherche par nom.
 * Une classe peut porter le nom d'un enseignant (ex. « CE1 Martin ») : les
 * élèves qui s'appellent Martin passent avant les élèves de cette classe.
 * 0 = le nom commence par la saisie, 1 = le prénom, 2 = nom ou prénom la
 * contient, 3 = correspondance sur la classe seulement.
 * Accents et casse ignorés ; ordre alphabétique (nom, prénom) à rang égal.
 */
export function rankEleveSearch<T extends RankableEleve>(pool: T[], query: string, limit?: number): T[] {
  const needle = fold(query);
  if (needle.length < 2) return [];
  const rankOf = (e: T): number => {
    const nom = fold(e.nom || "");
    const prenom = fold(e.prenom || "");
    const prenomNom = `${prenom} ${nom}`;
    const nomPrenom = `${nom} ${prenom}`;
    if (nom.startsWith(needle) || nomPrenom.startsWith(needle)) return 0;
    if (prenom.startsWith(needle) || prenomNom.startsWith(needle)) return 1;
    if (prenomNom.includes(needle) || nomPrenom.includes(needle)) return 2;
    if (fold(e.classe || "").includes(needle)) return 3;
    return -1;
  };
  const ranked = pool
    .map((e) => ({ e, rank: rankOf(e) }))
    .filter((x) => x.rank >= 0)
    .sort(
      (a, b) =>
        a.rank - b.rank ||
        `${a.e.nom} ${a.e.prenom}`.localeCompare(`${b.e.nom} ${b.e.prenom}`, "fr", { sensitivity: "base" }),
    )
    .map((x) => x.e);
  return limit ? ranked.slice(0, limit) : ranked;
}
