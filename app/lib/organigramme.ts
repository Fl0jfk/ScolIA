/**
 * Organigramme nominatif La Providence (seed uniquement).
 * Le runtime construit les slots direction depuis les établissements.
 */
import { scolaImageUrl } from "@/app/lib/scola-image";
import { SCHOOL } from "@/app/lib/school";

export type OrganigramPerson = {
  id: string;
  firstName?: string;
  lastName?: string;
  role: string;
  photoUrl?: string | null;
  email?: string;
  missions: string[];
};

export type OrganigramBlock = {
  id: string;
  title: string;
  description?: string;
  people: OrganigramPerson[];
};

export type OrganigramPole = {
  id: string;
  label: string;
  blocks: OrganigramBlock[];
};

export const ORGANIGRAM_DIRECTORS: OrganigramPerson[] = [
  {
    id: "dir-ecole",
    firstName: "Elise",
    lastName: "PLANTEC",
    role: "Directrice de l'école",
    photoUrl: scolaImageUrl("organigram/Plantec-Elise-1.png"),
    email: SCHOOL.ecole.emailHref.replace(/^mailto:/, ""),
    missions: [
      "Pilotage pédagogique et éducatif de l'école (maternelle & élémentaire).",
      "Recouvrement des créances.",
      "Photocopieurs",

    ],
  },
  {
    id: "dir-college",
    firstName: "Anne-Sophie",
    lastName: "DUMOUCHEL",
    role: "Directrice du collège",
    photoUrl:scolaImageUrl("organigram/Direction_5-removebg-preview.png"),
    email: SCHOOL.college.emailHref.replace(/^mailto:/, ""),
    missions: [
      "Pilotage pédagogique et éducatif du collège. ",
      "Gestion des espaces verts",
      "Gestion des salles de permanence, du CDI et des salles d'EPS.",
      "Gestion du matériel",
      "Gestion de l'internat",
    ],
  },
  {
    id: "dir-lycee",
    firstName: "Anne-Marie",
    lastName: "DONA",
    role: "Directrice du lycée",
    photoUrl:scolaImageUrl("organigram/mme-dona-directrice-lycee-la-providence-nicolas-barre-mesnil-esnard.png"),
    email: SCHOOL.lycee.emailHref.replace(/^mailto:/, ""),
    missions: [
      "Pilotage pédagogique et éducatif du lycée. ",
      "Coordination de l'ensemble de l'établissement. ",
      "Gestion des laboratoires",
      "Gestion de la sécurité",
      "Gestion du matériel informatique",
      "Gestion du chauffage",
      "Gestion du CSE",
      "Gestion de la communication externe",
    ],
  },
];

export const ORGANIGRAM_ADMIN: OrganigramBlock = {
  id: "admin-transverse",
  title: "Administration",
  description:
    "Missions larges : secrétariats, relations rectorat, remplacements, inscriptions, communication, outils numériques, etc. Les intitulés ci-dessous sont des exemples à personnaliser.",
  people: [
    {
      id: "admin-1",
      firstName: "Pauline",
      lastName: "LEBLOND",
      photoUrl:scolaImageUrl("organigram/LEBLOND+Pauline.jpg"),
      role: "Gestion administrative & secrétariat école",
      missions: [
        "Secrétariat de l'école. ",
        "Inscriptions et réinscriptions des élèves. ",
        "Gestion de la comptabilité sur la plateforme ZeenDoc",
        "Gestion des heures de vol pour le BIA ",
      ],
    },
    {
      id: "admin-2",
      firstName: "Sarah",
      lastName: "VILLIER",
      photoUrl:scolaImageUrl("organigram/BUNO+SARAH.jpg"),
      role: "Gestion administrative & secrétariat collège",
      missions: [
        "Secrétariat du collège. ",
        "Interface avec le rectorat (contractuels, remplacements, dossiers administratifs enseignants).",
        "Gestion des enseignants collèges-lycées (HSE,absences, administratifs, mutations, voeux, etc.)"
      ],
    },
    {
      id: "admin-3",
      firstName: "Florian",
      lastName: "HACQUEVILLE-MATHI",
      photoUrl:scolaImageUrl("organigram/FHM.jpeg"),
      role: "Gestion administrative & secrétariat lycée",
      missions: [
        "Secrétariat du lycée. ",
        "Inscriptions et réinscriptions des élèves. ",
        "Sorties / autorisations de sortie, organisation de transports pour sorties scolaires.",
      ],
    },
  ],
};

export const ORGANIGRAM_ACCOUNTING: OrganigramBlock = {
  id: "comptabilite",
  title: "Comptabilité",
  description: "Trois profils complémentaires — regroupés pour la lisibilité de l'organigramme.",
  people: [
    {
      id: "compta-1",
      firstName: "Valérie",
      lastName: "VASSEUR",
      photoUrl:"",
      role: "Attachée de gestion",
      missions: ["Suivi des factures et règlements. ", "Interfaces avec les services de l'établissement."],
    },
    {
      id: "compta-2",
      firstName: "Anaïs",
      lastName: "BOUTIGNY",
      photoUrl:scolaImageUrl("organigram/BOUTIGNY+Ana%C3%AFs.jpg"),
      role: "Comptable",
      missions: ["Suivi budgétaire. ", "Tableaux de pilotage pour la direction et l'OGEC."],
    },
    {
      id: "compta-3",
      firstName: "Cécile",
      lastName: "DOUAGLIN",
      photoUrl:scolaImageUrl("organigram/CECILE+DOUAGLIN.jpg"),
      role: "Comptable",
      missions: ["Déclarations et liens avec les organismes. ", "À préciser selon l'organisation réelle."],
    },
  ],
};

export const ORGANIGRAM_RECEPTION: OrganigramBlock = {
  id: "accueil-standard",
  title: "Accueil & standard téléphonique",
  description:
    "Premier contact téléphonique et orientation vers les services.",
  people: [
    {
      id: "standard",
      firstName: "Karine",
      lastName: "PERRIER",
      photoUrl:scolaImageUrl("organigram/PERRIER+Karine.jpg"),
      role: "Standard & accueil",
      missions: [
        "Orientation des appels vers les bons interlocuteurs. ",
        "Premier contact avec les familles et partenaires. ",
      ],
    },
  ],
};

export const ORGANIGRAM_HEALTH: OrganigramBlock = {
  id: "pole-sante",
  title: "Pôle santé & accompagnement",
  description:
    "Professionnels de santé et spécialistes auprès des élèves. D'autres profils (ergothérapie, etc.) pourront compléter ce pôle.",
  people: [
    {
      id: "infirmiere",
      firstName: "Ludmila",
      lastName: "BERBRA",
      photoUrl:scolaImageUrl("organigram/Ludmila+BERBRA.jpeg"),
      role: "Infirmière scolaire",
      missions: [
        "Accueil des élèves, soins, prévention et liaison avec les familles. ",
        "Allègement du service de vie scolaire sur le volet santé au lycée. ",
      ],
    },
    {
      id: "psychologue",
      firstName: "Thomas",
      lastName: "PEREZ",
      photoUrl:scolaImageUrl("organigram/Thomas+PEREZ.jpg"),
      role: "Psychologue scolaire",
      missions: [
        "Accompagnement psychologique des élèves et appui aux équipes. ",
        "Liaison avec les familles et les partenaires extérieurs selon les protocoles. ",
      ],
    }
  ],
};

export const ORGANIGRAM_MAINTENANCE: OrganigramBlock = {
  id: "pole-maintenance",
  title: "Pôle maintenance & bâtiments",
  description:
    "Entretien des locaux, suivi des interventions et lien avec les prestataires. Complétez les noms et le détail des missions ci-dessous.",
  people: [
    {
      id: "maint-1",
      firstName: "Jérôme",
      lastName: "LAINE",
      photoUrl:scolaImageUrl("organigram/LAINE+JEROME.jpg"),
      role: "Responsable maintenance",
      missions: [
        "Entretien courant des locaux et des espaces communs. ",
        "Traitement des demandes via la plateforme de signalement maintenance. ",
        "À compléter : périmètre précis (électricité, plomberie, espaces verts, etc.). ",
      ],
    },
    {
      id: "maint-2",
      firstName: "Fabien",
      lastName: "BENBATTA",
      photoUrl:"",
      role: "Agent de maintenance",
      missions: [
        "Appui aux interventions et suivi des petits travaux. ",
        "Coordination avec la direction pour les urgences et la sécurité des biens. ",
        "À compléter : astreinte, clés, fournitures, liaisons externes. ",
      ],
    },
  ],
};

export const ORGANIGRAM_POLES: OrganigramPole[] = [
  {
    id: "pole-ecole",
    label: SCHOOL.ecole.label,
    blocks: [
      {
        id: "vs-ecole",
        title: "Vie scolaire & accompagnement",
        people: [
          {
            id: "cfe-ecole",
            firstName: "Pauline",
            lastName: "LEBLOND",
            photoUrl:scolaImageUrl("organigram/LEBLOND+Pauline.jpg"),
            role: "Référente vie scolaire",
            missions: ["Suivi des élèves, liaison familles–école. "],
          }
        ],
      },
    ],
  },
  {
    id: "pole-college",
    label: SCHOOL.college.label,
    blocks: [
      {
        id: "vs-college",
        title: "Vie scolaire — équipe & CPE",
        description: "Plusieurs CPE selon l'organisation du collège. ",
        people: [
          { id: "cfe-col-1", 
            firstName: "Gaëlle",
            lastName: "CORIOU",
            photoUrl:scolaImageUrl("organigram/CORIOU+Gaelle.jpg"),
            role: "CPE 6EME/5EME", 
            missions: ["À compléter. "] },
          { id: "cfe-col-2",
            firstName: "Sylvain",
            lastName: "LAQUIEVRE",
            photoUrl:"",
            role: "CPE 4EME/3EME",
            missions: ["À compléter. "] },
          { id: "cfe-col-3", 
            firstName: "Lise",
            lastName: "HAMEL",
            photoUrl:scolaImageUrl("organigram/HAMEL+LISE.jpg"),
            role: "CPE transverse - Responsables des surveillants", 
            missions: ["À compléter. "] },
          { id: "vs-col-assist", 
            firstName: "Constance",
            lastName: "VIERA DA ROSA",
            photoUrl:scolaImageUrl("organigram/VIEIRA+DA+ROSA+Constance.jpg"),
            role: "Assistant·e éducation / équipe vie scolaire", 
            missions: ["À compléter. "] },
        ],
      },
    ],
  },
  {
    id: "pole-lycee",
    label: SCHOOL.lycee.label,
    blocks: [
      {
        id: "vs-lycee",
        title: "Vie scolaire lycée",
        people: [
          {
            id: "cfe-lycee",
            firstName: "Isabelle",
            lastName: "CONSTANT",
            photoUrl:scolaImageUrl("organigram/I.+CONSTANT.jpg"),
            role: "CPE lycée",
            missions: ["Suivi disciplinaire et éducatif. ", "Accompagnement des élèves et des familles. "],
          },
          {
            id: "vs-lycee-2",
            firstName: "Karim",
            lastName: "YAICI",
            photoUrl:scolaImageUrl("organigram/YAICI+Karim.jpg"),
            role: "Membre équipe vie scolaire",
            missions: ["À compléter. "],
          }
        ],
      },
    ],
  },
];

export const ORGANIGRAM_PASTORAL: OrganigramBlock = {
  id: "pastorale",
  title: "Pastorale",
  description: "Accompagnement spirituel et projet pastoral du groupe scolaire. ",
  people: [
    { id: "past-1", 
      firstName: "Céline",
      lastName: "DUBOC",
      photoUrl:scolaImageUrl("organigram/DUBOC+CELINE.jpg"),
      role: "Référent·e pastorale", missions: ["À compléter. "] },
    { id: "past-2",
      firstName: "Charlotte",
      lastName: "MASSET",
      photoUrl:"",
      role: "Membre équipe pastorale", missions: ["À compléter. "] },
    { id: "past-3", 
      firstName: "Soeur",
      lastName: "PATRICIA",
      photoUrl:"",
      role: "Aumônier / intervenant·e (si applicable)", missions: ["À compléter. "] },
  ],
};

export const ORGANIGRAM_OGEC: OrganigramBlock = {
  id: "ogec",
  title: "OGEC",
  description:
    "Organisme de gestion de l'enseignement catholique : employeur du personnel administratif, de comptabilité et des équipes de service rattachées à l'OGEC. Pour les enseignants, le cadre employeur peut différer selon le statut — à préciser en interne si besoin.",
  people: [
    {
      id: "ogec-1",
      firstName: "Christian",
      lastName: "DELSINNE",
      photoUrl:"",
      role: "OGEC — contact / direction (à compléter)",
      missions: [
        "Ressources humaines et administration du personnel relevant de l'OGEC. ",
        "Contrats, gestion administrative et lien avec la direction d'établissement. ",
        "À compléter : nom et coordonnées. ",
      ],
    },
    {
      id: "ogec-2",
      firstName: "Mr",
      lastName: "HAREL",
      photoUrl:"",
      role: "OGEC — contact complémentaire",
      missions: ["À compléter selon votre organisation. "],
    },
  ],
};

export const ORGANIGRAM_TUTELLE: OrganigramBlock = {
  id: "tutelle",
  title: "Tutelle de la congrégation",
  description:
    "La tutelle relève de la congrégation : ce n'est ni l'OGEC ni un employeur au même titre. Elle porte la mission et la vigilance sur le projet spirituel et la gouvernance propre à la famille religieuse.",
  people: [
    {
      id: "tutelle-1",
      firstName: "Soeur",
      lastName: "ANGELE",
      photoUrl:"",
      role: "Tutelle — référent·e congrégationnel·le",
      missions: [
        "Lien avec la tutelle religieuse et les instances de la congrégation. ",
        "À compléter : nom et coordonnées. ",
      ],
    },
    {
      id: "tutelle-2",
      firstName: "Jean-Sébastien",
      lastName: "DOUERE",
      photoUrl:"",
      role: "Tutelle — référent·e congrégationnel·le",
      missions: [
        "Lien avec la tutelle religieuse et les instances de la congrégation. ",
        "À compléter : nom et coordonnées. ",
      ],
    },
  ],
};
