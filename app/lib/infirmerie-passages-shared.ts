/** Constantes & types passages infirmerie — safe client & serveur. */

export const INFIRMERIE_SUITES = [
  "repos",
  "retour_cours",
  "renvoi_famille",
  "urgence",
  "autre",
] as const;

export type InfirmerieSuite = (typeof INFIRMERIE_SUITES)[number];

export const INFIRMERIE_SUITE_LABELS: Record<InfirmerieSuite, string> = {
  repos: "Repos à l'infirmerie",
  retour_cours: "Retour en cours",
  renvoi_famille: "Renvoi à la famille",
  urgence: "Urgence",
  autre: "Autre",
};

export type InfirmeriePassageRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  arrivee: string;
  sortie: string | null;
  motifCourt: string;
  suite: InfirmerieSuite | null;
  soinsNotes: string | null;
  signalVieScolaire: boolean;
  auteurNom: string | null;
};

export function isInfirmerieSuite(v: string | null | undefined): v is InfirmerieSuite {
  return !!v && (INFIRMERIE_SUITES as readonly string[]).includes(v);
}
