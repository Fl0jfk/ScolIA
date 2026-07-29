import type { BrainToolDefinition } from "@/app/lib/brain-ai/types";
import { handleCreateAbsence } from "@/app/lib/brain-ai/tools/handlers/absences";
import { handleCreateHseDemand, handleListHseDemands } from "@/app/lib/brain-ai/tools/handlers/hse";
import { handleGetInternatStatus } from "@/app/lib/brain-ai/tools/handlers/internat";
import { handleOcrModuleStatus } from "@/app/lib/brain-ai/tools/handlers/ocr";
import {
  handleCreatePhotocopie,
  handleListPhotocopies,
} from "@/app/lib/brain-ai/tools/handlers/photocopies";
import {
  handleCheckAvailability,
  handleCreateReservation,
  handleListRooms,
} from "@/app/lib/brain-ai/tools/handlers/rooms";
import { handleCreateRequest } from "@/app/lib/brain-ai/tools/handlers/requests";
import { handleGetStagesOverview } from "@/app/lib/brain-ai/tools/handlers/stages";
import {
  handleCreateTrip,
  handleGetTripStatus,
  handleListTripsBrief,
} from "@/app/lib/brain-ai/tools/handlers/travels";
import {
  handleGetWeekSheetRange,
  handleGetWeekSheetToday,
} from "@/app/lib/brain-ai/tools/handlers/week-sheet";

export const BRAIN_TOOLS: BrainToolDefinition[] = [
  {
    name: "get_week_sheet_today",
    description:
      "Lit la feuille de semaine (actualité live) pour aujourd'hui ou une date donnée. Utiliser pour « qu'est-ce qui se passe aujourd'hui / cette semaine ». ",
    parameters: {
      type: "object",
      properties: {
        date: { type: "string", description: "YYYY-MM-DD (défaut: aujourd'hui, fuseau Paris)" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/dashboard",
    moduleId: "dashboard-week-sheet",
    requiresAuth: true,
    mutates: false,
    handler: handleGetWeekSheetToday,
  },
  {
    name: "get_week_sheet_range",
    description: "Liste les événements de la feuille de semaine entre deux dates (max ~31 jours).",
    parameters: {
      type: "object",
      properties: {
        from: { type: "string", description: "YYYY-MM-DD" },
        to: { type: "string", description: "YYYY-MM-DD" },
      },
      required: ["from", "to"],
      additionalProperties: false,
    },
    pathPrefix: "/dashboard",
    moduleId: "dashboard-week-sheet",
    requiresAuth: true,
    mutates: false,
    handler: handleGetWeekSheetRange,
  },
  {
    name: "list_trips_brief",
    description: "Liste les séjours/voyages (titre, dates, classes, statut workflow).",
    parameters: {
      type: "object",
      properties: {
        limit: { type: "number", description: "Nombre max (défaut 12)" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/travels",
    moduleId: "travels",
    requiresAuth: true,
    mutates: false,
    handler: handleListTripsBrief,
  },
  {
    name: "get_trip_status",
    description: "Statut d'un séjour par id ou recherche texte (titre/destination).",
    parameters: {
      type: "object",
      properties: {
        tripId: { type: "string" },
        query: { type: "string", description: "Titre ou destination partielle" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/travels",
    moduleId: "travels",
    requiresAuth: true,
    mutates: false,
    handler: handleGetTripStatus,
  },
  {
    name: "create_trip",
    description:
      "Crée un brouillon de séjour (SIMPLE ou COMPLEX) après avoir collecté titre et date. Demander confirmation.",
    parameters: {
      type: "object",
      properties: {
        title: { type: "string" },
        destination: { type: "string" },
        date: { type: "string", description: "YYYY-MM-DD" },
        startDate: { type: "string" },
        endDate: { type: "string" },
        classes: { type: "string" },
        etablissement: { type: "string" },
        nbEleves: { type: "number" },
        type: { type: "string", enum: ["SIMPLE", "COMPLEX"] },
      },
      required: ["title"],
      additionalProperties: false,
    },
    pathPrefix: "/travels",
    moduleId: "travels",
    requiresAuth: true,
    mutates: true,
    handler: handleCreateTrip,
  },
  {
    name: "list_rooms",
    description: "Liste les salles réservables.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    pathPrefix: "/prof-room",
    moduleId: "prof-room",
    requiresAuth: true,
    mutates: false,
    handler: async (ctx) => handleListRooms(ctx),
  },
  {
    name: "check_availability",
    description: "Vérifie la disponibilité d'une salle pour une date et des créneaux horaires (heures entières, créneau H:30→H+1:30).",
    parameters: {
      type: "object",
      properties: {
        roomId: { type: "string" },
        date: { type: "string", description: "YYYY-MM-DD" },
        selectedHours: {
          type: "array",
          items: { type: "number" },
          description: "Heures de début (ex. [8,9] pour 8h30 et 9h30)",
        },
      },
      required: ["roomId", "date", "selectedHours"],
      additionalProperties: false,
    },
    pathPrefix: "/prof-room",
    moduleId: "prof-room",
    requiresAuth: true,
    mutates: false,
    handler: handleCheckAvailability,
  },
  {
    name: "create_reservation",
    description:
      "Réserve une salle. Collecter salle, date, créneaux, matière, classe, récurrence. Toujours confirmer avant.",
    parameters: {
      type: "object",
      properties: {
        roomId: { type: "string" },
        date: { type: "string" },
        selectedHours: { type: "array", items: { type: "number" } },
        subject: { type: "string" },
        className: { type: "string" },
        comment: { type: "string" },
        recurrence: { type: "string", enum: ["none", "weekly", "biweekly"] },
        untilDate: { type: "string" },
        firstName: { type: "string" },
        lastName: { type: "string" },
        email: { type: "string" },
      },
      required: ["roomId", "date", "selectedHours"],
      additionalProperties: false,
    },
    pathPrefix: "/prof-room",
    moduleId: "prof-room",
    requiresAuth: true,
    mutates: true,
    handler: handleCreateReservation,
  },
  {
    name: "create_request",
    description:
      "Crée une demande / ticket interne (sujet + description détaillée ≥ 15 caractères). Confirmer avant création.",
    parameters: {
      type: "object",
      properties: {
        subject: { type: "string" },
        description: { type: "string" },
        contact: {
          type: "object",
          properties: {
            firstName: { type: "string" },
            lastName: { type: "string" },
            email: { type: "string" },
            phone: { type: "string" },
          },
        },
      },
      required: ["description"],
      additionalProperties: false,
    },
    requiresAuth: true,
    mutates: true,
    handler: handleCreateRequest,
  },
  {
    name: "create_absence",
    description:
      "Déclare l'absence de l'utilisateur connecté uniquement (pas de lecture RH). Collecter date, motif, établissement si professeur. Confirmer avant.",
    parameters: {
      type: "object",
      properties: {
        date: { type: "string", description: "YYYY-MM-DD" },
        startDate: { type: "string" },
        endDate: { type: "string" },
        periodType: { type: "string", enum: ["single_day", "multi_day"] },
        startTime: { type: "string" },
        endTime: { type: "string" },
        reason: { type: "string" },
        details: { type: "string" },
        scope: { type: "string", enum: ["professeur", "ogec"] },
        etablissement: { type: "string", enum: ["École", "Collège", "Lycée"] },
      },
      required: ["reason"],
      additionalProperties: false,
    },
    pathPrefix: "/absences",
    moduleId: "absences",
    requiresAuth: true,
    mutates: true,
    handler: handleCreateAbsence,
  },
  {
    name: "ocr_module_status",
    description:
      "Préflight du module Ajout de documents IA (OCR) : rôles, listes élèves, config OneDrive. Retourne des CTA.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    pathPrefix: "/agentIAOCR",
    moduleId: "agent-ia-ocr",
    requiresAuth: true,
    mutates: false,
    handler: async (ctx) => handleOcrModuleStatus(ctx),
  },
  {
    name: "list_photocopies",
    description:
      "Liste les demandes de photocopies couleur visibles (soi ou direction établissement). Filtre status optionnel.",
    parameters: {
      type: "object",
      properties: {
        status: { type: "string", enum: ["EN_ATTENTE", "ACCEPTEE", "REFUSEE"] },
        limit: { type: "number" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/photocopies-couleur",
    moduleId: "photocopies-couleur",
    requiresAuth: true,
    mutates: false,
    handler: handleListPhotocopies,
  },
  {
    name: "create_photocopie_demand",
    description:
      "Crée une demande de photocopies couleur (sans PDF — CTA pour joindre ensuite). Confirmer avant. Champs: etablissement, motif, classesOuMatiere, nombrePhotocopies.",
    parameters: {
      type: "object",
      properties: {
        etablissement: { type: "string", enum: ["École", "Collège", "Lycée"] },
        motif: { type: "string" },
        classesOuMatiere: { type: "string" },
        nombrePhotocopies: { type: "number" },
      },
      required: ["etablissement", "motif", "classesOuMatiere", "nombrePhotocopies"],
      additionalProperties: false,
    },
    pathPrefix: "/photocopies-couleur",
    moduleId: "photocopies-couleur",
    requiresAuth: true,
    mutates: true,
    handler: handleCreatePhotocopie,
  },
  {
    name: "list_hse_demands",
    description:
      "Liste les demandes HSE visibles (soi pour un prof, ou direction de l'établissement). Pas de données RH.",
    parameters: {
      type: "object",
      properties: {
        status: { type: "string", enum: ["EN_ATTENTE", "ACCEPTEE", "REFUSEE", "ANNULEE"] },
        limit: { type: "number" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/demandes-hse",
    moduleId: "demandes-hse",
    requiresAuth: true,
    mutates: false,
    handler: handleListHseDemands,
  },
  {
    name: "create_hse_demand",
    description:
      "Crée une demande HSE (enseignants uniquement). Collecter etablissement, resumeDemande, nombreHeures (multiple de 0,25), classe. Confirmer avant.",
    parameters: {
      type: "object",
      properties: {
        etablissement: { type: "string", enum: ["École", "Collège", "Lycée"] },
        resumeDemande: { type: "string" },
        nombreHeures: { type: "number" },
        classe: { type: "string" },
        details: { type: "string" },
      },
      required: ["etablissement", "resumeDemande", "nombreHeures", "classe"],
      additionalProperties: false,
    },
    pathPrefix: "/demandes-hse",
    moduleId: "demandes-hse",
    requiresAuth: true,
    mutates: true,
    handler: handleCreateHseDemand,
  },
  {
    name: "get_stages_overview",
    description:
      "Vue d'ensemble stages : compteurs (offres, conventions, file admin, signatures), signatures en attente pour l'utilisateur, conventions récentes.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    pathPrefix: "/stages",
    moduleId: "stages",
    requiresAuth: true,
    mutates: false,
    handler: async (ctx) => handleGetStagesOverview(ctx),
  },
  {
    name: "get_internat_status",
    description:
      "Statut live internat : effectifs, occupation, appel du soir, incidents 30j (agrégats, pas de dossiers nominatifs sensibles).",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    pathPrefix: "/gestion-internat",
    moduleId: "internat",
    requiresAuth: true,
    mutates: false,
    handler: async (ctx) => handleGetInternatStatus(ctx),
  },
];

export function getBrainTool(name: string): BrainToolDefinition | undefined {
  return BRAIN_TOOLS.find((t) => t.name === name);
}

export function mistralToolsForUser(signedIn: boolean) {
  const tools = signedIn ? BRAIN_TOOLS : BRAIN_TOOLS.filter((t) => !t.requiresAuth);
  return tools.map((t) => ({
    type: "function" as const,
    function: {
      name: t.name,
      description: t.description,
      parameters: t.parameters,
    },
  }));
}
