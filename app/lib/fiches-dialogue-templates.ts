import type {
  FdCatalogueChoix,
  FdEtapeKind,
  FdStarterMode,
} from "@/db/schema-fiches-dialogue";

export type FdTemplateEtapeDef = {
  kind: FdEtapeKind;
  label: string;
  description?: string;
  optionnelle?: boolean;
};

export type FdCampagneTemplate = {
  key: string;
  label: string;
  calendrierMode: "trimestre" | "semestre" | "personnalise";
  /** Qui commence — surchargeable à la création. */
  starterMode: FdStarterMode;
  description: string;
  etapes: FdTemplateEtapeDef[];
  catalogue: FdCatalogueChoix;
};

const SPE_GEN: FdCatalogueChoix["options"] = [
  { id: "spe_hggsp", label: "Hist-géo, géopolitique et sciences politiques", kind: "specialite" },
  { id: "spe_hlp", label: "Humanités, littérature et philosophie", kind: "specialite" },
  { id: "spe_llce", label: "Langues, littératures et cultures étrangères (AGL)", kind: "specialite" },
  { id: "spe_maths", label: "Mathématiques", kind: "specialite" },
  { id: "spe_physique", label: "Physique-Chimie", kind: "specialite" },
  { id: "spe_svt", label: "Sciences de la Vie et de la Terre", kind: "specialite" },
  { id: "spe_si", label: "Sciences de l’Ingénieur", kind: "specialite" },
  { id: "spe_ses", label: "Sciences Economiques et Sociales", kind: "specialite" },
];

const OPT_COLLEGE_BASE: FdCatalogueChoix["options"] = [
  { id: "lv1_anglais", label: "LV1 Anglais", kind: "lv" },
  { id: "lv1_allemand", label: "Option LV1 Allemand", kind: "lv" },
  { id: "lv2_allemand", label: "LV2 Allemand", kind: "lv" },
  { id: "lv2_espagnol", label: "LV2 Espagnol", kind: "lv" },
  { id: "latin", label: "Latin", kind: "option_interne" },
  { id: "lce_anglais", label: "LCE Anglais", kind: "option_interne" },
  { id: "ebep", label: "EBEP", kind: "option_interne" },
];

function catalogueCollege(destId: string, destLabel: string, extraOptions: FdCatalogueChoix["options"] = []): FdCatalogueChoix {
  return {
    destinations: [
      { id: destId, label: destLabel, niveauCible: destId, interne: true },
      { id: "maintien", label: "Maintien dans la classe actuelle", interne: true },
      { id: "ailleurs", label: "Autre établissement", interne: false },
      { id: "autre_parcours", label: "Autre parcours", interne: false },
    ],
    options: [...OPT_COLLEGE_BASE, ...extraOptions],
    fields: [
      {
        id: "destination",
        type: "select",
        label: "Orientation souhaitée l’année prochaine",
        required: true,
        optionsFrom: "destinations",
      },
      {
        id: "options",
        type: "multiselect",
        label: "Options / langues",
        required: false,
        optionsFrom: "options",
        showWhen: { fieldId: "destination", equals: [destId, "maintien"] },
      },
      {
        id: "etablissements",
        type: "etablissement_multi",
        label: "Établissement(s) souhaité(s)",
        required: false,
        helpText: "Chez nous par défaut, ou choisissez dans le référentiel national.",
        showWhen: { fieldId: "destination", equals: [destId, "ailleurs"] },
      },
      {
        id: "autre_parcours_detail",
        type: "textarea",
        label: "Précisez l’autre parcours",
        required: false,
        showWhen: { fieldId: "destination", equals: "autre_parcours" },
        helpText: "Compagnons du devoir, arrêt de scolarité, etc.",
      },
    ],
    voiesOuverture: [{ niveauActuel: destId.replace(/e$/, "e"), destinationsIds: [destId, "maintien", "ailleurs", "autre_parcours"] }],
  };
}

const CATALOGUE_6E = catalogueCollege("5e", "5ᵉ", []);
const CATALOGUE_5E = catalogueCollege("4e", "4ᵉ", [
  { id: "ose", label: "OSE (Options sciences expérimentales)", kind: "option_interne" },
]);
const CATALOGUE_4E = catalogueCollege("3e", "3ᵉ", [
  { id: "ose", label: "OSE (Options sciences expérimentales)", kind: "option_interne" },
  { id: "odp", label: "ODP (Option découverte professionnelle)", kind: "option_interne" },
  { id: "3e_prepa", label: "3ᵉ prépa-métiers", kind: "autre" },
]);

const CATALOGUE_2NDE: FdCatalogueChoix = {
  destinations: [
    { id: "1ere_g", label: "1ʳᵉ générale", niveauCible: "1ere", interne: true },
    { id: "1ere_st2s", label: "1ʳᵉ ST2S", niveauCible: "1ere", interne: true },
    { id: "1ere_techno_autre", label: "1ʳᵉ technologique (autre série)", niveauCible: "1ere", interne: false },
    { id: "maintien", label: "Maintien en 2ⁿᵈᵉ", interne: true },
    { id: "ailleurs", label: "Autre établissement", interne: false },
    { id: "autre_parcours", label: "Autre parcours", interne: false },
  ],
  options: SPE_GEN,
  fields: [
    {
      id: "destination",
      type: "select",
      label: "Orientation souhaitée l’année prochaine",
      required: true,
      optionsFrom: "destinations",
    },
    {
      id: "specialites",
      type: "multiselect",
      label: "3 enseignements de spécialité",
      required: true,
      optionsFrom: "options",
      maxSelect: 3,
      showWhen: { fieldId: "destination", equals: "1ere_g" },
      helpText: "Choisissez exactement 3 spécialités.",
    },
    {
      id: "serie_techno",
      type: "text",
      label: "Série technologique (si autre que ST2S)",
      required: false,
      showWhen: { fieldId: "destination", equals: "1ere_techno_autre" },
    },
    {
      id: "etablissements",
      type: "etablissement_multi",
      label: "Établissement(s) souhaité(s)",
      required: false,
      showWhen: {
        fieldId: "destination",
        equals: ["1ere_g", "1ere_st2s", "1ere_techno_autre", "ailleurs"],
      },
    },
    {
      id: "autre_parcours_detail",
      type: "textarea",
      label: "Précisez l’autre parcours",
      required: false,
      showWhen: { fieldId: "destination", equals: "autre_parcours" },
    },
  ],
};

const CATALOGUE_1ERE_GEN: FdCatalogueChoix = {
  destinations: [
    { id: "tle_g", label: "Terminale générale", niveauCible: "tle", interne: true },
    { id: "tle_st2s", label: "Terminale ST2S", niveauCible: "tle", interne: true },
    { id: "maintien", label: "Maintien en 1ʳᵉ", interne: true },
    { id: "ailleurs", label: "Autre établissement", interne: false },
    { id: "autre_parcours", label: "Autre parcours", interne: false },
  ],
  options: [
    ...SPE_GEN,
    { id: "opt_dgemc", label: "Droits et grands enjeux du monde contemporain", kind: "autre" },
    { id: "opt_maths_exp", label: "Mathématiques expertes", kind: "autre" },
    { id: "opt_maths_comp", label: "Mathématiques complémentaires", kind: "autre" },
  ],
  fields: [
    {
      id: "destination",
      type: "select",
      label: "Orientation souhaitée",
      required: true,
      optionsFrom: "destinations",
    },
    {
      id: "specialites",
      type: "multiselect",
      label: "2 spécialités conservées en terminale",
      required: true,
      optionsFrom: "options",
      maxSelect: 2,
      showWhen: { fieldId: "destination", equals: "tle_g" },
    },
    {
      id: "option_terminale",
      type: "select",
      label: "Option de terminale (1 seule)",
      required: false,
      inlineOptions: [
        { id: "opt_dgemc", label: "Droits et grands enjeux du monde contemporain" },
        { id: "opt_maths_exp", label: "Mathématiques expertes" },
        { id: "opt_maths_comp", label: "Mathématiques complémentaires" },
      ],
      showWhen: { fieldId: "destination", equals: "tle_g" },
    },
    {
      id: "etablissements",
      type: "etablissement_multi",
      label: "Établissement(s) souhaité(s)",
      required: false,
    },
    {
      id: "autre_parcours_detail",
      type: "textarea",
      label: "Précisez l’autre parcours",
      required: false,
      showWhen: { fieldId: "destination", equals: "autre_parcours" },
    },
  ],
};

const CATALOGUE_1ERE_ST2S: FdCatalogueChoix = {
  destinations: [
    { id: "tle_st2s", label: "Terminale ST2S", niveauCible: "tle", interne: true },
    { id: "maintien", label: "Maintien en 1ʳᵉ ST2S", interne: true },
    { id: "ailleurs", label: "Autre établissement", interne: false },
    { id: "autre_parcours", label: "Autre parcours", interne: false },
  ],
  options: [],
  fields: [
    {
      id: "destination",
      type: "select",
      label: "Orientation souhaitée",
      required: true,
      optionsFrom: "destinations",
    },
    {
      id: "etablissements",
      type: "etablissement_multi",
      label: "Établissement(s) souhaité(s)",
      required: false,
    },
    {
      id: "autre_parcours_detail",
      type: "textarea",
      label: "Précisez l’autre parcours",
      required: false,
      showWhen: { fieldId: "destination", equals: "autre_parcours" },
    },
  ],
};

const CATALOGUE_TERMINALE: FdCatalogueChoix = {
  destinations: [
    { id: "intentions_sup", label: "Intentions d’inscription dans l’enseignement supérieur", interne: false },
  ],
  options: [],
  fields: [
    {
      id: "universite",
      type: "checkbox",
      label: "Filières universitaires",
      required: false,
    },
    {
      id: "universite_detail",
      type: "text",
      label: "Précisions (filières)",
      required: false,
      showWhen: { fieldId: "universite", equals: "true" },
    },
    {
      id: "cpge",
      type: "checkbox",
      label: "CPGE",
      required: false,
    },
    {
      id: "cpge_detail",
      type: "text",
      label: "Précisions CPGE",
      required: false,
    },
    {
      id: "bts_dut",
      type: "checkbox",
      label: "BTS / BUT",
      required: false,
    },
    {
      id: "bts_detail",
      type: "text",
      label: "Précisions BTS / BUT",
      required: false,
    },
    {
      id: "ecoles",
      type: "checkbox",
      label: "Autres filières sélectives (écoles post-bac)",
      required: false,
    },
    {
      id: "ecoles_detail",
      type: "text",
      label: "Précisions écoles",
      required: false,
    },
    {
      id: "autre_inscription",
      type: "textarea",
      label: "Autre(s) inscription(s) envisagée(s)",
      required: false,
    },
  ],
};

const ETAPES_COLLEGE: FdTemplateEtapeDef[] = [
  {
    kind: "conseil",
    label: "2ᵉ trimestre — proposition du conseil (provisoire)",
    description: "Le conseil propose ; signatures PP puis direction.",
  },
  {
    kind: "saisie_famille",
    label: "2ᵉ trimestre — réponse de la famille",
    description: "La famille répond à la proposition provisoire.",
  },
  {
    kind: "decision_finale_conseil",
    label: "3ᵉ trimestre — avis définitif du conseil",
    description: "Décision définitive ; signatures PP puis direction.",
  },
  {
    kind: "acceptation_famille",
    label: "3ᵉ trimestre — réponse définitive de la famille",
    description: "Acceptation ou refus (appel si refus).",
  },
  {
    kind: "appel",
    label: "Procédure d’appel",
    description: "Activée uniquement si la famille refuse.",
    optionnelle: true,
  },
];

const ETAPES_LYCEE: FdTemplateEtapeDef[] = [
  {
    kind: "saisie_famille",
    label: "1ᵉʳ semestre — demande de la famille",
    description: "La famille formule ses vœux.",
  },
  {
    kind: "conseil",
    label: "1ᵉʳ semestre — avis provisoire du conseil",
    description: "Avis du conseil ; signatures PP puis direction.",
  },
  {
    kind: "choix_definitifs",
    label: "2ⁿᵈ semestre — demande définitive de la famille",
    description: "La famille confirme ou ajuste.",
  },
  {
    kind: "decision_finale_conseil",
    label: "2ⁿᵈ semestre — proposition du conseil",
    description: "Proposition définitive ; signatures PP puis direction.",
  },
  {
    kind: "acceptation_famille",
    label: "2ⁿᵈ semestre — réponse définitive de la famille",
    description: "Acceptation ou refus (appel si refus).",
  },
  {
    kind: "appel",
    label: "Procédure d’appel",
    description: "Activée uniquement en cas de refus.",
    optionnelle: true,
  },
];

const ETAPES_TERMINALE: FdTemplateEtapeDef[] = [
  {
    kind: "saisie_famille",
    label: "1ᵉʳ semestre — intentions élève / famille",
    description: "Intentions d’inscription dans le supérieur.",
  },
  {
    kind: "conseil",
    label: "1ᵉʳ semestre — avis du conseil",
    description: "Avis valant conseil.",
  },
  {
    kind: "choix_definitifs",
    label: "2ⁿᵈ semestre — intentions actualisées",
    description: "Mise à jour des intentions.",
  },
  {
    kind: "conseil",
    label: "2ⁿᵈ semestre — avis du conseil",
    description: "Avis valant conseil.",
  },
];

/** Templates prêts à l’emploi — entièrement surchargeables à la création. */
export const FD_CAMPAGNE_TEMPLATES: FdCampagneTemplate[] = [
  {
    key: "college_6e",
    label: "Collège — après 6ᵉ",
    calendrierMode: "trimestre",
    starterMode: "conseil_dabord",
    description: "Conseil propose d’abord, puis réponse famille (papier 6ᵉ).",
    catalogue: CATALOGUE_6E,
    etapes: ETAPES_COLLEGE,
  },
  {
    key: "college_5e",
    label: "Collège — après 5ᵉ",
    calendrierMode: "trimestre",
    starterMode: "conseil_dabord",
    description: "Conseil propose d’abord, puis réponse famille (papier 5ᵉ).",
    catalogue: CATALOGUE_5E,
    etapes: ETAPES_COLLEGE,
  },
  {
    key: "college_4e",
    label: "Collège — après 4ᵉ",
    calendrierMode: "trimestre",
    starterMode: "conseil_dabord",
    description: "Conseil propose d’abord, puis réponse famille (papier 4ᵉ).",
    catalogue: CATALOGUE_4E,
    etapes: ETAPES_COLLEGE,
  },
  {
    key: "college_trimestriel",
    label: "Collège — générique (trimestres)",
    calendrierMode: "trimestre",
    starterMode: "conseil_dabord",
    description: "Modèle collège : conseil d’abord. Adaptez le catalogue aux options de l’établissement.",
    catalogue: CATALOGUE_5E,
    etapes: ETAPES_COLLEGE,
  },
  {
    key: "lycee_2nde",
    label: "Lycée — après 2ⁿᵈᵉ",
    calendrierMode: "semestre",
    starterMode: "famille_dabord",
    description: "Famille demande d’abord (1ʳᵉ générale / ST2S / ailleurs).",
    catalogue: CATALOGUE_2NDE,
    etapes: ETAPES_LYCEE,
  },
  {
    key: "lycee_1ere_gen",
    label: "Lycée — après 1ʳᵉ générale",
    calendrierMode: "semestre",
    starterMode: "famille_dabord",
    description: "Famille demande d’abord ; 2 spécialités + option de terminale.",
    catalogue: CATALOGUE_1ERE_GEN,
    etapes: ETAPES_LYCEE,
  },
  {
    key: "lycee_1ere_st2s",
    label: "Lycée — après 1ʳᵉ ST2S",
    calendrierMode: "semestre",
    starterMode: "famille_dabord",
    description: "Passage terminale ST2S / maintien / autre.",
    catalogue: CATALOGUE_1ERE_ST2S,
    etapes: ETAPES_LYCEE,
  },
  {
    key: "lycee_semestriel",
    label: "Lycée — générique (semestres)",
    calendrierMode: "semestre",
    starterMode: "famille_dabord",
    description: "Modèle lycée : famille d’abord.",
    catalogue: CATALOGUE_2NDE,
    etapes: ETAPES_LYCEE,
  },
  {
    key: "terminale_intentions",
    label: "Terminale — intentions post-bac",
    calendrierMode: "semestre",
    starterMode: "famille_dabord",
    description: "Intentions d’inscription dans le supérieur (avis valant conseil).",
    catalogue: CATALOGUE_TERMINALE,
    etapes: ETAPES_TERMINALE,
  },
  {
    key: "personnalise",
    label: "Personnalisé (vide)",
    calendrierMode: "personnalise",
    starterMode: "famille_dabord",
    description: "Vous définissez librement les étapes, le catalogue et qui commence.",
    catalogue: { destinations: [], options: [], fields: [] },
    etapes: [
      { kind: "saisie_famille", label: "Vœux famille" },
      { kind: "decision_finale_conseil", label: "Décision définitive du conseil" },
      { kind: "acceptation_famille", label: "Acceptation famille" },
      { kind: "appel", label: "Procédure d’appel", optionnelle: true },
    ],
  },
];

export function getFdTemplate(key: string): FdCampagneTemplate | undefined {
  return FD_CAMPAGNE_TEMPLATES.find((t) => t.key === key);
}

export const FD_ETAPE_KIND_LABELS: Record<FdEtapeKind, string> = {
  saisie_famille: "Saisie famille",
  conseil: "Conseil de classe",
  choix_definitifs: "Choix définitifs famille",
  decision_finale_conseil: "Décision finale conseil",
  acceptation_famille: "Acceptation famille",
  appel: "Appel",
};

export const FD_STARTER_MODE_LABELS: Record<FdStarterMode, string> = {
  conseil_dabord: "Le conseil de classe commence",
  famille_dabord: "La famille commence",
};
