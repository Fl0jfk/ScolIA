import type { BrainToolCtx, BrainToolDefinition } from "@/app/lib/brain-ai/types";
import { assertToolPermissions } from "@/app/lib/brain-ai/permissions";
import { handleCreateAbsence } from "@/app/lib/brain-ai/tools/handlers/absences";
import { handleCreateHseDemand, handleListHseDemands } from "@/app/lib/brain-ai/tools/handlers/hse";
import { handleGetInternatStatus, handleAssignInternatRoom, handleOpenInternatAppel } from "@/app/lib/brain-ai/tools/handlers/internat";
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
import { handleResendStageSignatures } from "@/app/lib/brain-ai/tools/handlers/stages-signatures";
import {
  handleCreateTrip,
  handleGetTripStatus,
  handleListTripsBrief,
  handlePreviewVoyageImpacts,
} from "@/app/lib/brain-ai/tools/handlers/travels";
import {
  handleGetWeekSheetRange,
  handleGetWeekSheetToday,
} from "@/app/lib/brain-ai/tools/handlers/week-sheet";
import {
  handleListDestinations,
  handleResolveAndOpen,
} from "@/app/lib/brain-ai/tools/handlers/navigation";
import {
  handleOpenEleveDossier,
  handleSearchEleves,
  handleUpdateEleveRegime,
} from "@/app/lib/brain-ai/tools/handlers/eleves";
import {
  handleCancelAccueilAbsence,
  handleCreateAccueilAbsence,
} from "@/app/lib/brain-ai/tools/handlers/accueil-absences";
import { handleListElevesFiltered } from "@/app/lib/brain-ai/tools/handlers/eleves-filtered";
import { handleUpdateEleveGrilleRepas } from "@/app/lib/brain-ai/tools/handlers/eleve-grille-repas";
import { handleCreateElevePreinscrit } from "@/app/lib/brain-ai/tools/handlers/eleve-create";
import { handleOpenTrip } from "@/app/lib/brain-ai/tools/handlers/open-trip";
import { handleDecideRhAbsence } from "@/app/lib/brain-ai/tools/handlers/rh-absences";
import { handleGetMyPendingActions } from "@/app/lib/brain-ai/personal-signals";
import {
  handleGetEdt,
  handleGetEleve,
  handleGetGrilleRepas,
  handleGetPresenceJour,
  handleGetTenantContext,
  handleGetVoyage,
} from "@/app/lib/brain-ai/tools/handlers/core-read";
import {
  handleCloseAppel,
  handleOpenAppel,
  handleSaveAppelLignes,
} from "@/app/lib/brain-ai/tools/handlers/vs-appels";

const BRAIN_TOOLS: BrainToolDefinition[] = [
  {
    name: "get_my_pending_actions",
    description:
      "File personnelle de l’utilisateur : signatures stages, absences à valider, photocopies, demandes… d’après ses signaux intranet. Utiliser pour « qu’est-ce que j’ai à faire », « mes signatures », « à traiter ».",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    pathPrefix: "/dashboard",
    moduleId: "dashboard-week-sheet",
    requiresAuth: true,
    mutates: false,
    handler: handleGetMyPendingActions,
  },
  {
    name: "resolve_and_open",
    description:
      "Ouvre une page de l’intranet (modale ou navigation). Utiliser dès que l’utilisateur veut aller sur un module / une page / un écran. Passer query (texte libre) ou destinationId ou href.",
    parameters: {
      type: "object",
      properties: {
        query: { type: "string", description: "Ex. sorties scolaires, photocopies, absences" },
        destinationId: { type: "string" },
        href: { type: "string", description: "Chemin interne /…" },
        label: { type: "string" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/dashboard",
    moduleId: "dashboard-week-sheet",
    requiresAuth: true,
    mutates: false,
    handler: handleResolveAndOpen,
  },
  {
    name: "list_destinations",
    description: "Liste les pages / modules accessibles (optionnellement filtrés par query).",
    parameters: {
      type: "object",
      properties: {
        query: { type: "string" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/dashboard",
    moduleId: "dashboard-week-sheet",
    requiresAuth: true,
    mutates: false,
    handler: handleListDestinations,
  },
  {
    name: "get_tenant_context",
    description:
      "Contexte établissement courant (slug, sites, présence d’un site internat). À appeler avant de parler d’internat ou multi-sites.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    pathPrefix: "/api/dashboard/signals",
    moduleId: "dashboard-week-sheet",
    requiresAuth: true,
    mutates: false,
    handler: handleGetTenantContext,
  },
  {
    name: "search_eleves",
    description: "Recherche des élèves par nom/prénom (dossiers). Retourne id, classe, régime.",
    parameters: {
      type: "object",
      properties: {
        query: { type: "string", description: "Nom et/ou prénom" },
      },
      required: ["query"],
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: false,
    handler: handleSearchEleves,
  },
  {
    name: "open_eleve_dossier",
    description:
      "Ouvre le dossier élève (ou les documents d’inscription) en modale, ou directement un document d’accompagnement (PAP/PAI/PPS/GEVASCO) en aperçu PDF. Passer eleveId ou query (nom). documentKind=pap|pai|pps|gevasco pour ouvrir la pièce. subView=inscription pour les docs d’inscription.",
    parameters: {
      type: "object",
      properties: {
        eleveId: { type: "string" },
        query: { type: "string" },
        subView: { type: "string", enum: ["dossier", "inscription"] },
        documentKind: {
          type: "string",
          description: "pap | pai | pps | gevasco — ouvre le PDF directement",
        },
      },
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: false,
    handler: handleOpenEleveDossier,
  },
  {
    name: "get_eleve",
    description:
      "Fiche élève filtrée par droits dossier (identité / scolarité / famille). Pas de création. Prof hors classe → refus.",
    parameters: {
      type: "object",
      properties: {
        eleveId: { type: "string" },
        query: { type: "string" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: false,
    handler: handleGetEleve,
  },
  {
    name: "get_presence_jour",
    description:
      "Où est l’élève / la classe aujourd’hui (occupancy) : en_sortie, en_stage, a_infirmerie, absent_vs, hors_etablissement, en_cours… " +
      "en_sortie / a_infirmerie ≠ absence bulletin. Params : eleveId | query | classe, date optionnelle.",
    parameters: {
      type: "object",
      properties: {
        eleveId: { type: "string" },
        query: { type: "string" },
        classe: { type: "string" },
        date: { type: "string", description: "YYYY-MM-DD (défaut aujourd’hui Paris)" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: false,
    handler: handleGetPresenceJour,
  },
  {
    name: "update_eleve_regime",
    description:
      "Change le régime d’un élève (interne / demi-pensionnaire / externe). Demande toujours confirmation UI. Passer eleveId ou query + regime.",
    parameters: {
      type: "object",
      properties: {
        eleveId: { type: "string" },
        query: { type: "string" },
        regime: {
          type: "string",
          description: "interne | demi_pension | externe (ou libellé FR)",
        },
      },
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: true,
    handler: handleUpdateEleveRegime,
  },
  {
    name: "list_eleves_filtered",
    description:
      "Liste les élèves filtrés par classe/pôle et/ou accompagnement (PAP, PAI, PPS, GEVASCO). Ex. « tous les PAP du collège », « PAP de 6ème B ». « 6ème A » / « sixième A » = classe 6A (pas 6E). Affiche un catalogue groupé par classe avec aperçu PDF cliquable pour chaque document — ne pas inventer de liens.",
    parameters: {
      type: "object",
      properties: {
        classe: {
          type: "string",
          description:
            "Classe (ex. 6ème A, 6A) ou pôle entier (Collège, Lycée, École). Garder la lettre de division si précisée.",
        },
        accompagnement: {
          type: "string",
          description: "pap | pai | pps | gevasco | any",
        },
      },
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: false,
    handler: handleListElevesFiltered,
  },
  {
    name: "create_accueil_absence",
    description:
      "Déclare une absence ou un retard élève à l’accueil (aujourd’hui, multi-jours, ou horaires). Wizard + confirmation. Ex. « Paul est absent 2 jours ».",
    parameters: {
      type: "object",
      properties: {
        query: { type: "string", description: "Nom élève" },
        subjectId: { type: "string" },
        eleveNature: { type: "string", enum: ["absence", "retard"] },
        mode: { type: "string", enum: ["today", "multi_day", "hours"] },
        startDate: { type: "string" },
        endDate: { type: "string" },
        days: { type: "number", description: "Nombre de jours (raccourci multi_day)" },
        startTime: { type: "string" },
        endTime: { type: "string" },
        motif: { type: "string" },
        canal: { type: "string", enum: ["telephone", "physique", "mail"] },
      },
      additionalProperties: false,
    },
    pathPrefix: "/vie-scolaire/absences",
    moduleId: "accueil-absences",
    requiresAuth: true,
    mutates: true,
    handler: handleCreateAccueilAbsence,
  },
  {
    name: "cancel_accueil_absence",
    description:
      "Annule une absence/retard élève déclarée à l’accueil (board du jour). Passer query (nom) ou absenceId.",
    parameters: {
      type: "object",
      properties: {
        query: { type: "string" },
        absenceId: { type: "string" },
        date: { type: "string", description: "YYYY-MM-DD (défaut aujourd’hui)" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/vie-scolaire/absences",
    moduleId: "accueil-absences",
    requiresAuth: true,
    mutates: true,
    handler: handleCancelAccueilAbsence,
  },
  {
    name: "update_eleve_grille_repas",
    description:
      "Modifie la grille repas d’un élève (nb de midi / jours). Ex. « 3 repas par semaine », preset 3|4|5|0. Confirmation UI.",
    parameters: {
      type: "object",
      properties: {
        eleveId: { type: "string" },
        query: { type: "string" },
        repasParSemaine: { type: "number" },
        preset: { type: "string" },
        days: { type: "string", description: "lun,mar,jeu" },
        soir: { type: "boolean" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: true,
    handler: handleUpdateEleveGrilleRepas,
  },
  {
    name: "open_trip",
    description:
      "Ouvre un séjour / sortie scolaire EXISTANT (voir, afficher, accéder). Si plusieurs matchent, propose un choix. Passer tripId ou query. IMPORTANT : pour « ouvre / montre / va sur » une sortie → TOUJOURS cet outil, JAMAIS create_trip.",
    parameters: {
      type: "object",
      properties: {
        tripId: { type: "string" },
        query: { type: "string", description: "Titre ou destination (vide = liste récente)" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/travels",
    moduleId: "travels",
    requiresAuth: true,
    mutates: false,
    handler: handleOpenTrip,
  },
  {
    name: "get_edt",
    description:
      "Créneaux EDT ScolIA (`edt_creneau`) pour une date / classe. Étiquette source — grille non STS auto.",
    parameters: {
      type: "object",
      properties: {
        date: { type: "string" },
        classe: { type: "string" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/vie-scolaire/calendrier",
    moduleId: "vs-calendrier",
    requiresAuth: true,
    mutates: false,
    handler: handleGetEdt,
  },
  {
    name: "get_voyage",
    description:
      "Dossier voyage + liste participants live (eleve_id). Orthogonal à get_trip_status (workflow). Accueil : résumé sans fiche.",
    parameters: {
      type: "object",
      properties: {
        tripId: { type: "string" },
      },
      required: ["tripId"],
      additionalProperties: false,
    },
    pathPrefix: "/travels",
    moduleId: "travels",
    requiresAuth: true,
    mutates: false,
    handler: handleGetVoyage,
  },
  {
    name: "get_grille_repas",
    description:
      "Droit repas élève (grille scolarité). Ops cantine = unavailable. Pas d’allergies inventées.",
    parameters: {
      type: "object",
      properties: {
        eleveId: { type: "string" },
      },
      required: ["eleveId"],
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: false,
    handler: handleGetGrilleRepas,
  },
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
    description:
      "Analyse complète d’un séjour scolaire (JSON + étape workflow + manques + conseils). " +
      "À appeler dès qu’on parle d’un dossier travels : qui doit agir, devis reçus, blocage compta, SIMPLE vs COMPLEX. " +
      "Paramètres : tripId (préféré) ou query (titre/destination).",
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
      "Crée une NOUVELLE sortie scolaire (wizard). Uniquement si l’utilisateur dit créer / nouvelle / démarrer une sortie. INTERDIT si « ouvre », « montre », « va sur », « affiche » une sortie → utiliser open_trip.",
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
      required: [],
      additionalProperties: false,
    },
    pathPrefix: "/travels",
    moduleId: "travels",
    requiresAuth: true,
    mutates: true,
    handler: handleCreateTrip,
  },
  {
    name: "preview_voyage_impacts",
    description:
      "Calcule les impacts A/B/C/D d’une sortie scolaire (créneaux EDT potentiellement vidés, " +
      "questions resto/internat/accompagnateurs). Orthogonal à get_trip_status (workflow). " +
      "Ne décide pas et n’écrit pas le planning. Paramètres : tripId ou query.",
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
    handler: handlePreviewVoyageImpacts,
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
    description:
      "Vérifie la disponibilité d'une salle pour une date et des créneaux horaires (heures entières, créneau H:30→H+1:30). La date DOIT être YYYY-MM-DD calculée depuis l'horloge Europe/Paris du system prompt (jamais une année inventée).",
    parameters: {
      type: "object",
      properties: {
        roomId: { type: "string" },
        date: {
          type: "string",
          description: "YYYY-MM-DD (ex. demain = date « Demain = » du system prompt)",
        },
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
      "Démarre / poursuit un wizard de réservation de salle. Appeler immédiatement (même sans args) : l'UI propose salle → date → créneaux libres → matière → classe, puis confirmation.",
    parameters: {
      type: "object",
      properties: {
        roomId: { type: "string" },
        date: {
          type: "string",
          description: "YYYY-MM-DD depuis l'horloge institutionnelle (pas d'année fantaisiste)",
        },
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
      required: [],
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
      "Démarre / poursuit un wizard de demande interne. Appeler immédiatement (même sans args) : sujet → description → confirmation.",
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
      required: [],
      additionalProperties: false,
    },
    requiresAuth: true,
    mutates: true,
    handler: handleCreateRequest,
  },
  {
    name: "create_absence",
    description:
      "Démarre / poursuit un wizard de demande d'autorisation d'absence (soi uniquement). Appeler immédiatement (même sans args) : date → durée → motif → établissement si prof → confirmation.",
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
        etablissement: { type: "string" },
      },
      required: [],
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
      "Liste les demandes de photocopies visibles (soi ou direction établissement). Filtre status optionnel.",
    parameters: {
      type: "object",
      properties: {
        status: { type: "string", enum: ["EN_ATTENTE", "ACCEPTEE", "REFUSEE"] },
        limit: { type: "number" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/photocopies",
    moduleId: "photocopies-couleur",
    requiresAuth: true,
    mutates: false,
    handler: handleListPhotocopies,
  },
  {
    name: "create_photocopie_demand",
    description:
      "Démarre / poursuit un wizard de photocopies. Appeler immédiatement (même sans args) : typeImpression (NOIR_BLANC|COULEUR) → établissement → motif → classes/matière → nombre → confirmation. N&B = file impressions directe ; couleur = validation direction. Si PDF joints via trombone, passer documents[] (max 5) ou documentKey/documentFileName/documentContentType.",
    parameters: {
      type: "object",
      properties: {
        typeImpression: {
          type: "string",
          enum: ["NOIR_BLANC", "COULEUR"],
          description: "NOIR_BLANC = direct impressions ; COULEUR = validation direction",
        },
        etablissement: { type: "string" },
        motif: { type: "string" },
        classesOuMatiere: { type: "string" },
        nombrePhotocopies: { type: "number" },
        documents: {
          type: "array",
          description: "PDF joints (max 5), chacun avec key / fileName / contentType",
          items: {
            type: "object",
            properties: {
              key: { type: "string" },
              fileName: { type: "string" },
              contentType: { type: "string" },
            },
            additionalProperties: false,
          },
        },
        documentKey: { type: "string", description: "Clé S3 d'un PDF joint (legacy / mono)" },
        documentFileName: { type: "string" },
        documentContentType: { type: "string" },
      },
      required: [],
      additionalProperties: false,
    },
    pathPrefix: "/photocopies",
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
      "Démarre / poursuit un wizard HSE (enseignants). Appeler immédiatement (même sans args) : établissement → résumé → heures (multiple 0,25) → classe → précisions optionnelles → confirmation.",
    parameters: {
      type: "object",
      properties: {
        etablissement: { type: "string" },
        resumeDemande: { type: "string" },
        nombreHeures: { type: "number" },
        classe: { type: "string" },
        details: { type: "string" },
      },
      required: [],
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
    name: "resend_stage_signatures",
    description:
      "Relance les e-mails de signature d’une convention de stage (file signatures_pending). Sans args : propose la liste. Passer query (nom élève) ou conventionId. openOnly=true pour ouvrir sans relancer.",
    parameters: {
      type: "object",
      properties: {
        conventionId: { type: "string" },
        query: { type: "string", description: "Nom élève / entreprise" },
        openOnly: { type: "boolean" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/stages",
    moduleId: "stages",
    requiresAuth: true,
    mutates: true,
    handler: handleResendStageSignatures,
  },
  {
    name: "decide_rh_absence",
    description:
      "File direction / validateur OGEC : lister les absences RH en attente, puis valider ou refuser (avec choix du traitement des heures si besoin). Appeler avec {} pour ouvrir la file.",
    parameters: {
      type: "object",
      properties: {
        absenceId: { type: "string" },
        query: { type: "string", description: "Nom de l’agent" },
        decision: { type: "string", enum: ["VALIDER", "REFUSER"] },
        hoursTreatment: {
          type: "string",
          description: "RATTRAPAGE | DEDUCTION_SALAIRE | RATTRAPAGE_INTERNE | DECLARATION_RECTORAT | DECLARATION_ONISE…",
        },
        managerNote: { type: "string" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/absences",
    moduleId: "absences",
    requiresAuth: true,
    mutates: true,
    handler: handleDecideRhAbsence,
  },
  {
    name: "create_eleve_preinscrit",
    description:
      "Crée un dossier élève préinscrit (direction / admin / administratif). Wizard : nom → prénom → e-mail parent → confirmation. Ouvre ensuite les docs d’inscription.",
    parameters: {
      type: "object",
      properties: {
        nom: { type: "string" },
        prenom: { type: "string" },
        parentEmail: { type: "string" },
        parentPhone: { type: "string" },
        parentFirstName: { type: "string" },
        parentLastName: { type: "string" },
        classe: { type: "string" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/eleves/dossiers",
    moduleId: "eleve-dossier",
    requiresAuth: true,
    mutates: true,
    handler: handleCreateElevePreinscrit,
  },
  {
    name: "open_appel",
    description:
      "Ouvre (ou récupère) la feuille d’appel d’un créneau EDT pour une date. Prof : ses créneaux uniquement. Confirmation UI obligatoire.",
    parameters: {
      type: "object",
      properties: {
        dateAppel: { type: "string", description: "YYYY-MM-DD" },
        creneauId: { type: "string" },
      },
      required: ["dateAppel", "creneauId"],
      additionalProperties: false,
    },
    pathPrefix: "/vie-scolaire/absences",
    moduleId: "vs-appels",
    requiresAuth: true,
    mutates: true,
    handler: handleOpenAppel,
  },
  {
    name: "save_appel_lignes",
    description:
      "Enregistre les lignes présent/absent/retard d’une feuille d’appel ouverte. Même handler que l’UI. Confirmation obligatoire.",
    parameters: {
      type: "object",
      properties: {
        appelId: { type: "string" },
        lignes: {
          type: "array",
          items: {
            type: "object",
            properties: {
              eleveId: { type: "string" },
              statut: { type: "string" },
              retardMinutes: { type: "number" },
              note: { type: "string" },
            },
            required: ["eleveId", "statut"],
          },
        },
      },
      required: ["appelId", "lignes"],
      additionalProperties: false,
    },
    pathPrefix: "/vie-scolaire/absences",
    moduleId: "vs-appels",
    requiresAuth: true,
    mutates: true,
    handler: handleSaveAppelLignes,
  },
  {
    name: "close_appel",
    description:
      "Clôture l’appel : statut clos, absences CPE (source appel), événement attendance.call_completed. Confirmation obligatoire.",
    parameters: {
      type: "object",
      properties: {
        appelId: { type: "string" },
      },
      required: ["appelId"],
      additionalProperties: false,
    },
    pathPrefix: "/vie-scolaire/absences",
    moduleId: "vs-appels",
    requiresAuth: true,
    mutates: true,
    handler: handleCloseAppel,
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
  {
    name: "open_internat_appel",
    description: "Ouvre l’écran d’appel du soir internat.",
    parameters: { type: "object", properties: {}, additionalProperties: false },
    pathPrefix: "/gestion-internat",
    moduleId: "internat",
    requiresAuth: true,
    mutates: false,
    handler: async (ctx) => handleOpenInternatAppel(ctx),
  },
  {
    name: "assign_internat_room",
    description:
      "Affecte un interne à une chambre (ou retire). Wizard : élève → chambre libre → confirmation.",
    parameters: {
      type: "object",
      properties: {
        studentId: { type: "string" },
        query: { type: "string", description: "Nom de l’interne" },
        roomId: { type: "string" },
        roomQuery: { type: "string", description: "Libellé chambre" },
      },
      additionalProperties: false,
    },
    pathPrefix: "/gestion-internat",
    moduleId: "internat",
    requiresAuth: true,
    mutates: true,
    handler: handleAssignInternatRoom,
  },
];

export function getBrainTool(name: string): BrainToolDefinition | undefined {
  return BRAIN_TOOLS.find((t) => t.name === name);
}

/**
 * Outils exposés au modèle : filtrés par auth + droits module intranet.
 * Un outil absent de la liste = l’utilisateur n’y a pas accès (le prompt l’indique).
 */
export function mistralToolsForUser(
  signedIn: boolean,
  ctx?: Pick<BrainToolCtx, "userId" | "roles" | "isOrgAdmin" | "audience" | "etablissementId">,
) {
  let tools = signedIn ? BRAIN_TOOLS : BRAIN_TOOLS.filter((t) => !t.requiresAuth);
  if (ctx && signedIn) {
    const gateCtx: BrainToolCtx = {
      userId: ctx.userId,
      roles: ctx.roles,
      isOrgAdmin: ctx.isOrgAdmin,
      audience: ctx.audience,
      etablissementId: ctx.etablissementId ?? null,
      confirmed: false,
    };
    tools = tools.filter((t) => assertToolPermissions(gateCtx, t).ok);
  }
  return tools.map((t) => ({
    type: "function" as const,
    function: {
      name: t.name,
      description: t.description,
      parameters: t.parameters,
    },
  }));
}
