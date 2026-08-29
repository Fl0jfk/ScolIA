import type { FdCatalogueChoix, FdEtapeKind } from "@/db/schema-fiches-dialogue";

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
  description: string;
  etapes: FdTemplateEtapeDef[];
  catalogue: FdCatalogueChoix;
};

const CATALOGUE_COLLEGE: FdCatalogueChoix = {
  destinations: [
    { id: "6e", label: "6ᵉ", niveauCible: "6e" },
    { id: "5e", label: "5ᵉ", niveauCible: "5e" },
    { id: "4e", label: "4ᵉ", niveauCible: "4e" },
    { id: "3e", label: "3ᵉ", niveauCible: "3e" },
  ],
  options: [
    { id: "anglais", label: "Anglais", kind: "lv" },
    { id: "espagnol", label: "Espagnol", kind: "lv" },
    { id: "allemand", label: "Allemand", kind: "lv" },
    { id: "latin", label: "Latin", kind: "option_interne" },
    { id: "grec", label: "Grec", kind: "option_interne" },
  ],
  fields: [
    {
      id: "destination",
      type: "select",
      label: "Classe / niveau souhaité l’année prochaine",
      required: true,
      optionsFrom: "destinations",
    },
    {
      id: "options",
      type: "multiselect",
      label: "Options / langues souhaitées",
      required: false,
      optionsFrom: "options",
    },
    {
      id: "commentaire_famille",
      type: "textarea",
      label: "Commentaire de la famille",
      required: false,
      helpText: "Précisez toute information utile pour le conseil de classe.",
    },
  ],
};

const CATALOGUE_LYCEE: FdCatalogueChoix = {
  destinations: [
    { id: "2nde", label: "2ᵈᵉ générale et technologique", niveauCible: "2nde" },
    { id: "1ere_g", label: "1ʳᵉ générale", niveauCible: "1ere" },
    { id: "1ere_techno", label: "1ʳᵉ technologique", niveauCible: "1ere" },
    { id: "terminale_g", label: "Terminale générale", niveauCible: "tle" },
    { id: "terminale_techno", label: "Terminale technologique", niveauCible: "tle" },
  ],
  options: [
    { id: "spe_maths", label: "Spécialité Mathématiques", kind: "specialite" },
    { id: "spe_physique", label: "Spécialité Physique-Chimie", kind: "specialite" },
    { id: "spe_svt", label: "Spécialité SVT", kind: "specialite" },
    { id: "spe_hggsp", label: "Spécialité HGGSP", kind: "specialite" },
    { id: "spe_ses", label: "Spécialité SES", kind: "specialite" },
    { id: "spe_llce", label: "Spécialité LLCE", kind: "specialite" },
    { id: "spe_nsi", label: "Spécialité NSI", kind: "specialite" },
    { id: "anglais", label: "Anglais", kind: "lv" },
    { id: "espagnol", label: "Espagnol", kind: "lv" },
    { id: "allemand", label: "Allemand", kind: "lv" },
  ],
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
      label: "Spécialités / options souhaitées",
      required: true,
      optionsFrom: "options",
      helpText: "Sélectionnez les spécialités ou options demandées.",
    },
    {
      id: "commentaire_famille",
      type: "textarea",
      label: "Commentaire de la famille",
      required: false,
    },
  ],
};

/** Templates prêts à l’emploi — entièrement surchargeables à la création. */
export const FD_CAMPAGNE_TEMPLATES: FdCampagneTemplate[] = [
  {
    key: "college_trimestriel",
    label: "Collège — trimestres (3 échanges)",
    calendrierMode: "trimestre",
    description:
      "Vœux famille → 1ᵉʳ conseil → vœux → 2ᵉ conseil → vœux → conseil final → acceptation famille (appel si refus).",
    catalogue: CATALOGUE_COLLEGE,
    etapes: [
      {
        kind: "saisie_famille",
        label: "Vœux famille — avant 1ᵉʳ conseil",
        description: "La famille indique la destination et les options souhaitées.",
      },
      {
        kind: "conseil",
        label: "1ᵉʳ conseil de classe",
        description: "Avis du conseil ; signatures PP et direction.",
      },
      {
        kind: "saisie_famille",
        label: "Vœux famille — avant 2ᵉ conseil",
        description: "La famille confirme ou ajuste ses vœux.",
      },
      {
        kind: "conseil",
        label: "2ᵉ conseil de classe",
        description: "Nouvel avis du conseil.",
      },
      {
        kind: "saisie_famille",
        label: "Vœux famille — avant conseil final",
        description: "Derniers vœux avant la décision définitive.",
      },
      {
        kind: "decision_finale_conseil",
        label: "Conseil de classe — décision définitive",
        description: "Décision finale du conseil ; signatures PP et direction.",
      },
      {
        kind: "acceptation_famille",
        label: "Acceptation famille",
        description: "La famille accepte ou refuse la décision finale.",
      },
      {
        kind: "appel",
        label: "Procédure d’appel",
        description: "Activée uniquement si la famille refuse la décision.",
        optionnelle: true,
      },
    ],
  },
  {
    key: "lycee_semestriel",
    label: "Lycée — semestres",
    calendrierMode: "semestre",
    description:
      "Vœux → 1ᵉʳ semestre → choix définitifs → conseil final → acceptation famille (appel si refus).",
    catalogue: CATALOGUE_LYCEE,
    etapes: [
      {
        kind: "saisie_famille",
        label: "Vœux famille — avant 1ᵉʳ semestre",
        description: "Orientation et spécialités souhaitées.",
      },
      {
        kind: "conseil",
        label: "Conseil — 1ᵉʳ semestre",
        description: "Avis du conseil de classe.",
      },
      {
        kind: "choix_definitifs",
        label: "Choix définitifs de la famille",
        description:
          "La famille confirme ou force ses choix malgré l’avis du 1ᵉʳ semestre.",
      },
      {
        kind: "decision_finale_conseil",
        label: "Conseil — décision définitive",
        description: "Décision finale du conseil.",
      },
      {
        kind: "acceptation_famille",
        label: "Acceptation famille",
        description: "Acceptation ou refus de la décision finale.",
      },
      {
        kind: "appel",
        label: "Procédure d’appel",
        description: "Activée uniquement en cas de refus.",
        optionnelle: true,
      },
    ],
  },
  {
    key: "personnalise",
    label: "Personnalisé (vide)",
    calendrierMode: "personnalise",
    description: "Vous définissez librement les étapes et le catalogue.",
    catalogue: { destinations: [], options: [], fields: [] },
    etapes: [
      {
        kind: "saisie_famille",
        label: "Vœux famille",
      },
      {
        kind: "decision_finale_conseil",
        label: "Décision définitive du conseil",
      },
      {
        kind: "acceptation_famille",
        label: "Acceptation famille",
      },
      {
        kind: "appel",
        label: "Procédure d’appel",
        optionnelle: true,
      },
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
