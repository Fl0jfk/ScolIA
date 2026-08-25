/**
 * Données nominatives La Providence (seed / migration uniquement).
 * Ne pas importer ce fichier au runtime applicatif.
 */
import { scolaImageUrl } from "@/app/lib/scola-image";
export const SCHOOL = {
  name: "Groupe Scolaire La Providence Nicolas Barré",
  shortName: "La Providence Nicolas Barré",
  address: {
    street: "6, rue de Neuvillette",
    city: "Le Mesnil-Esnard",
    zip: "76240",
    full: "6, rue de Neuvillette — 76240 Le Mesnil-Esnard",
    fullCompact: "6 rue de Neuvillette · 76240 Le Mesnil-Esnard",
    mapsEmbed:"https://www.google.com/maps/embed?pb=!1m18!1m12!1m3!1d2596.094939189387!2d1.1363342768705298!3d49.407110261934506!2m3!1f0!2f0!3f0!3m2!1i1024!2i768!4f13.1!3m3!1m2!1s0x47e0ded159510c6f%3A0x5f13891b9b6ad8fa!2s6%20Rue%20de%20Neuvillette%2C%2076240%20Le%20Mesnil-Esnard!5e0!3m2!1sfr!2sfr!4v1774430884088!5m2!1sfr!2sfr",
    mapsItinerary:"https://www.google.com/maps/dir/?api=1&destination=6+Rue+de+Neuvillette,+76240+Le+Mesnil-Esnard",
    latitude: 49.4071,
    longitude: 1.1363,
  },
  phone: {
    display: "02 32 86 50 90",
    tel: "tel:0232865090",
    hours: "Lun – Ven · 8h00 – 17h30",
  },
  preinscriptionUrl: "https://preinscriptions.ecoledirecte.com/fr/?RNE=0761713Z",
  reglementFinancier:scolaImageUrl("autres/R%C3%A8glement+financier+2026-2027.pdf"),
  ecole: {
    label: "École",
    directrice: "Mme Elise PLANTEC",
    email: "0762041f@ac-normandie.fr",
    emailHref: "mailto:0762041f@ac-normandie.fr",
    grades: "Maternelle & Élémentaire",
  },
  college: {
    label: "Collège",
    directrice: "Mme Anne-Sophie DUMOUCHEL",
    email: "0762565a@ac-normandie.fr",
    emailHref: "mailto:0762565a@ac-normandie.fr",
    grades: "6ème · 5ème · 4ème · 3ème",
    internPlaces: 25,
  },
  lycee: {
    label: "Lycée",
    directrice: "Mme Anne-Marie DONA",
    email: "0761713z@ac-normandie.fr",
    emailHref: "mailto:0761713z@ac-normandie.fr",
    grades: "2nde · 1ère · Terminale",
    internPlaces: 125,
  },
  /**
   * Routage absences (demande d'autorisation → validation direction → notification après validation).
   * Prof : mail création → direction de l'établissement ; validation → destinataires config (secrétariat cycle).
   * OGEC : mail création → direction ; validation → compta RH (+ responsables des surveillants si profil Surveillant).
   */
  absences: {
    /** Après validation d'une absence professeur — École */
    notifyProfEcole: {
      label: "Mme Pauline LEBLOND",
      email: "pauline.leblond@ac-normandie.fr",
    },
    /** Après validation d'une absence professeur — Collège ou Lycée */
    notifyProfCollegeLycee: {
      label: "Mme Sarah VILLIERS",
      email: "sarah.buno@ac-normandie.fr",
    },
    /** Après validation d'une absence personnel OGEC — comptabilité (2 destinataires) */
    notifyOgecCompta: [
      "anais.boutigny@laprovidence-nicolasbarre.fr",
      "valerie.vasseur@laprovidence-nicolasbarre.fr",
    ] as const,
  },
  /**
   * Emails pour le routage des demandes chatbot (routeId → boîtes réelles).
   * Ajustez selon votre organigramme : terrain vs chef, accueil photocopieur, CPE, etc.
   */
  requestsRouting: {
    /** Petits travaux, lampes, papier/toner, réappro salle */
    maintenanceTerrain: "sarah@laprovidence-nicolasbarre.fr",
    /**
     * File maintenance partagée : même demande visible par les deux jusqu'à prise en charge.
     * Ajustez les deux emails (ordre libre).
     */
    maintenancePool: [
      "sarah@laprovidence-nicolasbarre.fr",
      "florian@h-me.fr",
    ] as const,
    /** Copie systématique sur les routes maintenance.* (chef / coordinateur) */
    maintenanceChefCc: "florian@h-me.fr",
    /** Pannes photocopieur / lien prestataire — souvent accueil ou secrétariat */
    accueilPhotocopieur: "m.leblond@laprovidence-nicolasbarre.fr",
    /** Postes, comptes, messagerie, réseau */
    informatique: "florian@h-me.fr",
    /** CPE / vie scolaire collège (discipline, suivi élève) */
    cpeCollege: "sarah@laprovidence-nicolasbarre.fr",
    /** File de tri si la demande est ambiguë ou confiance IA basse */
    fileTri: "florian@h-me.fr",
  },
  /**
   * Grandes branches métier (module demandes) : libellés, catégorie et mots-clés de routage.
   * Les personnes (leaders / exécutants) sont dans `app/lib/staff-directory.ts`.
   */
  requestsBranches: {
    corbeille: {
      roleLabel: "Corbeille établissement",
      category: "Établissement",
      promptLine:
        "Demande floue, non classée ou renvoyée par un responsable : visible par toute l’équipe dans la corbeille jusqu’à prise en charge.",
      keywords: [] as const,
    },
    maintenance: {
      roleLabel: "Maintenance",
      category: "Établissement",
      promptLine:
        "Bâtiment, lampes, petits travaux, consommables (papier, toner), réseau, postes, comptes, matériel informatique — tout regroupé sous maintenance.",
      keywords: [
        "papier",
        "toner",
        "lampe",
        "ampoule",
        "fuite",
        "chauffage",
        "porte",
        "fenetre",
        "reparation",
        "photocopieur",
        "copieur",
        "bourrage",
        "mot de passe",
        "compte",
        "mail",
        "wifi",
        "ordinateur",
        "pc",
        "ent",
        "ecoledirecte",
        "imprimante",
        "connexion",
      ] as const,
    },
    admin_ecole: {
      roleLabel: "Administratif — école",
      category: "Scolarité",
      promptLine: "Secrétariat et administratif de l’école (maternelle / élémentaire), inscriptions côté école.",
      keywords: ["ecole", "maternelle", "elementaire", "primaire", "secretariat ecole", "inscription", "reinscription", "cp", "ce1", "ce2", "cm1", "cm2"] as const,
    },
    admin_college: {
      roleLabel: "Administratif — collège",
      category: "Scolarité",
      promptLine: "Secrétariat et administratif collège (bulletins, certificats, dossiers hors CPE pur).",
      keywords: ["bulletin", "certificat", "secretariat college", "6eme", "5eme", "4eme", "3eme", "college"] as const,
    },
    admin_lycee: {
      roleLabel: "Administratif — lycée",
      category: "Scolarité",
      promptLine: "Secrétariat et administratif lycée (2nde, 1re, Tale, baccalauréat).",
      keywords: ["lycee", "seconde", "premiere", "terminale", "bac", "baccalaureat", "secretariat lycee"] as const,
    },
    cpe_lycee: {
      roleLabel: "CPE — lycée",
      category: "Vie scolaire",
      promptLine: "CPE et vie scolaire au lycée.",
      keywords: ["cpe lycee", "vie scolaire lycee", "discipline lycee"] as const,
    },
    cpe_3e4e: {
      roleLabel: "CPE — 3e / 4e",
      category: "Vie scolaire",
      promptLine: "CPE collège — niveaux 3e et 4e.",
      keywords: ["cpe 3e", "cpe 4e", "4eme cpe", "3eme cpe"] as const,
    },
    cpe_5e6e: {
      roleLabel: "CPE — 5e / 6e",
      category: "Vie scolaire",
      promptLine: "CPE collège — niveaux 5e et 6e, discipline et suivi.",
      keywords: ["cpe", "discipline", "sanction", "retard", "5eme", "6eme", "vie scolaire college"] as const,
    },
    vie_scolaire_infirmerie: {
      roleLabel: "Vie scolaire & infirmerie",
      category: "Élèves",
      promptLine: "Absences, justificatifs, appels, santé / infirmerie, présence.",
      keywords: ["absence", "justificatif", "appel", "presence", "infirmerie", "sante", "medicament"] as const,
    },
    accueil: {
      roleLabel: "Accueil",
      category: "Établissement",
      promptLine: "Accueil, orientation des familles, photocopieur / prestataire si besoin.",
      keywords: ["accueil", "photocopieur accueil", "prestataire", "orientation"] as const,
    },
    comptabilite: {
      roleLabel: "Comptabilité",
      category: "Finances",
      promptLine: "Factures, paiements, tarifs, remboursements, règlement financier.",
      keywords: ["facture", "paiement", "reglement", "compta", "tarif", "remboursement", "budget", "frais"] as const,
    },
    direction_ecole: {
      roleLabel: "Direction — école",
      category: "Direction",
      promptLine: "Recours direction école, décisions directionnelles, plaintes graves côté école.",
      keywords: ["direction ecole", "directrice ecole", "recours direction", "plainte direction"] as const,
    },
    direction_college: {
      roleLabel: "Direction — collège",
      category: "Direction",
      promptLine: "Recours direction collège, décisions directionnelles, plaintes graves côté collège.",
      keywords: ["direction college", "directrice college", "recours direction", "plainte direction"] as const,
    },
    direction_lycee: {
      roleLabel: "Direction — lycée",
      category: "Direction",
      promptLine: "Recours direction lycée, décisions directionnelles, plaintes graves côté lycée.",
      keywords: ["direction lycee", "directrice lycee", "recours direction", "plainte direction"] as const,
    },
  },
} as const;