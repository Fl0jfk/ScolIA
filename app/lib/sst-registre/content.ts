/** Contenu du registre SST numérique — ce que le personnel lit avant d’émarger. */

export const SST_PRESENTATION = {
  title: "Présentation du registre",
  paragraphs: [
    "Le présent registre de sécurité et santé au travail est l’outil officiel de l’établissement pour tracer les signalements, favoriser la prévention des risques professionnels et améliorer les conditions de travail.",
    "Il remplace le classeur papier conservé à l’accueil. Chaque membre du personnel doit en connaître l’existence et le mode d’emploi, et le signer au moins une fois par année scolaire.",
    "Toute observation (risque, incident, presque-accident, dysfonctionnement d’un dispositif de sécurité) ou toute suggestion d’amélioration peut y être consignés via une fiche numérique. Le chef d’établissement, le référent sécurité, les membres du CSE et l’organisme de gestion en assurent le suivi.",
  ],
};

export const SST_SOMMAIRE = [
  "Présentation du registre",
  "Réglementation",
  "Notice d’utilisation",
  "Informations sur l’établissement",
  "Émargement de l’information aux personnels",
  "Visas des consultations",
  "Tableau de suivi et fiches du registre",
];

export const SST_REGLEMENTATION = {
  title: "Réglementation",
  intro:
    "Ce registre est établi conformément aux textes suivants, qui fondent l’obligation de sécurité de l’employeur et le cadre de prévention dans l’enseignement :",
  texts: [
    "Code du travail : articles L.4121-1 à L.4121-5 relatifs à l’obligation de sécurité de l’employeur",
    "Code de l’éducation",
    "Décret n°82-453 du 28 mai 1982 relatif à l’hygiène et à la sécurité",
    "Circulaires ministérielles en vigueur",
    "Références internes à l’établissement",
  ],
  objectifsTitle: "Objectifs du registre",
  objectifs: [
    "Assurer la traçabilité des signalements",
    "Favoriser la prévention des risques professionnels",
    "Améliorer les conditions de travail",
  ],
};

export const SST_NOTICE = {
  title: "Notice d’utilisation du registre",
  qui: "Tout personnel de l’établissement (enseignant, personnel OGEC, administratif, maintenance, vie scolaire, etc.).",
  comment:
    "En remplissant une fiche du registre (bouton « Déposer une fiche »). La fiche est numérotée automatiquement et transmise au circuit de suivi.",
  ou: "Dans le module RH de ScolIA — accessible à tout moment depuis votre espace personnel. Plus besoin de chercher un classeur à l’accueil.",
  quiConsulte:
    "Le chef d’établissement, le référent sécurité, les membres du CSE (ex-CHSCT), l’organisme de gestion.",
  quandIntro: "Dès lors qu’un personnel observe :",
  quand: [
    "un risque encouru éventuel ;",
    "un accident, un incident ou un presque-accident vu ou vécu ;",
    "un dysfonctionnement ou un non-fonctionnement d’une installation ou d’un dispositif de sécurité ;",
    "toute suggestion relative à la prévention des risques et à l’amélioration des conditions de travail.",
  ],
  commentCompleterTitle: "Comment compléter une fiche",
  commentCompleter: [
    "Nom et prénom (préremplis), fonction et signature manuscrite ;",
    "La date et l’heure de l’observation ;",
    "Le lieu (service, poste ou emplacement concerné) ;",
    "Les observations : risques ou dangers encourus, circonstances détaillées d’un fait, incident ou accident, facteurs matériels et humains ;",
    "Les propositions de solutions envisageables, selon vous (facultatif).",
  ],
  quiInformerTitle: "Qui doit être informé ?",
  quiInformer: "Le responsable hiérarchique, via le circuit de suivi de la fiche (référent sécurité / direction).",
  frequence: "En continu, à chaque signalement ou évolution. L’émargement de prise de connaissance, lui, est demandé au moins une fois par année scolaire.",
};

export const SST_EMARGEMENT_TEXTE =
  "Je soussigné(e) reconnais avoir pris connaissance de l’existence du présent registre de sécurité et santé au travail, de son contenu (présentation, réglementation, notice d’utilisation, informations établissement) et de son mode d’emploi.";

export const SST_URGENCES = [
  { label: "Pompiers", value: "18" },
  { label: "SAMU", value: "15" },
  { label: "Police", value: "17" },
];

export type SstEtablissementInfo = {
  nom: string;
  adresse: string;
  telephone: string;
  directions: Array<{ label: string; directorName: string }>;
  noteReferents: string;
};

export function buildSstEtablissementInfo(params: {
  siteName: string;
  addressFull?: string;
  phoneDisplay?: string;
  establishments: Array<{
    label: string;
    directorName?: string;
    active?: boolean;
  }>;
}): SstEtablissementInfo {
  const directions = params.establishments
    .filter((e) => e.active !== false && e.directorName?.trim())
    .map((e) => ({
      label: e.label,
      directorName: e.directorName!.trim(),
    }));

  return {
    nom: params.siteName.trim() || "Établissement",
    adresse: params.addressFull?.trim() || "Adresse non renseignée dans les paramètres du site.",
    telephone: params.phoneDisplay?.trim() || "",
    directions,
    noteReferents:
      "Le référent sécurité et le médecin du travail sont désignés par la direction. En cas de doute, adressez-vous à l’accueil ou à votre responsable hiérarchique.",
  };
}
