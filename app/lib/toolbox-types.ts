import type { RentreeEstablishmentPage, RentreeLinksByLevel } from "@/app/lib/rentree-types";
import { RENTREE_LINKS } from "@/app/lib/rentree-defaults";
import type { FournituresToolConfig } from "@/app/lib/fournitures-types";
import { DEFAULT_FOURNITURES_CONFIG } from "@/app/lib/fournitures-types";

export type { FournituresToolConfig } from "@/app/lib/fournitures-types";

export type ToolboxToolId =
  | "qrcreator"
  | "secret-santa"
  | "rentree"
  | "simulateur-tarifs"
  | "simulateur-fournitures"
  | "portes-ouvertes"
  | "repartition-classes"
  | "covoiturage";

export type RepartitionClassesToolConfig = {
  enabled: boolean;
  label: string;
};

export type CovoiturageToolConfig = {
  enabled: boolean;
  label: string;
};

export type TarifsNiveau = "maternelle" | "elementaire" | "college" | "lycee";

export type SimulateurTarifsConfig = {
  enabled: boolean;
  schoolYear: string;
  enseignement: Record<TarifsNiveau, number[]>;
  demiPension: Record<TarifsNiveau, Record<string, number>>;
  pensionAnnuel: number;
  garderie: Record<string, number>;
};

export type PortesOuvertesSlot = {
  id: string;
  label: string;
  startAt: string;
  endAt: string;
  maxPlaces?: number;
};

export type PortesOuvertesToolConfig = {
  enabled: boolean;
  title: string;
  intro: string;
  address: string;
  mapsUrl?: string;
  notifyEmail?: string;
  slots: PortesOuvertesSlot[];
  consentLabel: string;
};

export type SecretSantaToolConfig = {
  enabled: boolean;
  title: string;
  budgetHint: string;
  participantNames: string[];
};

export type RentreeToolConfig = {
  enabled: boolean;
  title: string;
  schoolYear: string;
  showSimulateurTarifs: boolean;
  showSimulateurFournitures: boolean;
  pages: RentreeEstablishmentPage[];
  /** Ancien format — utilisé pour migration vers `pages`. */
  links?: RentreeLinksByLevel[];
};

export type ToolboxConfig = {
  tools: {
    qrcreator: { enabled: boolean };
    "secret-santa": SecretSantaToolConfig;
    rentree: RentreeToolConfig;
    "simulateur-tarifs": SimulateurTarifsConfig;
    "simulateur-fournitures": FournituresToolConfig;
    "portes-ouvertes": PortesOuvertesToolConfig;
    "repartition-classes": RepartitionClassesToolConfig;
    covoiturage: CovoiturageToolConfig;
  };
};

const DEFAULT_TARIFS: SimulateurTarifsConfig = {
  enabled: false,
  schoolYear: "2026 / 2027",
  enseignement: {
    maternelle: [132, 117, 109, 87, 35],
    elementaire: [139, 124, 118, 97, 35],
    college: [164, 149, 140, 112, 45],
    lycee: [164, 149, 140, 112, 55],
  },
  demiPension: {
    maternelle: { "1": 21, "2": 43, "3": 64, "4": 85 },
    elementaire: { "1": 25, "2": 50, "3": 74, "4": 99 },
    college: { "1": 25, "2": 50, "3": 75, "4": 100, "5": 125 },
    lycee: { "1": 26, "2": 52, "3": 77, "4": 103, "5": 129 },
  },
  pensionAnnuel: 589,
  garderie: { "1": 15.5, "2": 31, "3": 46.5, "4": 62 },
};

export function defaultToolboxConfig(): ToolboxConfig {
  return {
    tools: {
      qrcreator: { enabled: true },
      "secret-santa": {
        enabled: false,
        title: "Secret Santa",
        budgetHint: "Environ 15 €",
        participantNames: [],
      },
      rentree: {
        enabled: false,
        title: "Préparation de la rentrée",
        schoolYear: "2026 / 2027",
        showSimulateurTarifs: true,
        showSimulateurFournitures: true,
        pages: [],
        links: RENTREE_LINKS,
      },
      "simulateur-tarifs": { ...DEFAULT_TARIFS },
      "simulateur-fournitures": { enabled: false, ...DEFAULT_FOURNITURES_CONFIG },
      "portes-ouvertes": {
        enabled: false,
        title: "Portes ouvertes",
        intro: "Inscrivez-vous pour visiter l'établissement et rencontrer l'équipe.",
        address: "",
        mapsUrl: "",
        notifyEmail: "",
        slots: [],
        consentLabel:
          "J'accepte que mes coordonnées soient utilisées pour organiser ma visite et me recontacter si besoin.",
      },
      "repartition-classes": {
        enabled: true,
        label: "Répartition des classes",
      },
      covoiturage: {
        enabled: false,
        label: "Covoiturage",
      },
    },
  };
}
