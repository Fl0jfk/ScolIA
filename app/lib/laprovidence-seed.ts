/**
 * Données de seed pour le tenant La Providence (dev / migration uniquement).
 * Ne pas importer ce fichier au runtime applicatif.
 */
import { SCHOOL } from "@/app/lib/school";
import type {
  Establishment,
  ExternalQuickLinkConfig,
  IntegrationsConfig,
  NotificationsConfig,
  SiteIdentity,
  StaffDirectoryRow,
  TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";
import { PLATFORM_ASSISTANCE_EMAIL } from "@/app/lib/platform-assistance-email";

export const LAPROVIDENCE_STAFF_DIRECTORY: StaffDirectoryRow[] = [
  { email: "florian@h-me.fr", branchId: "corbeille", role: "leader" },
  { email: "jerome.laine@laprovidence-nicolasbarre.fr", branchId: "maintenance", role: "leader" },
  { email: "sarah@laprovidence-nicolasbarre.fr", branchId: "maintenance", role: "executor" },
  { email: "m.leblond@laprovidence-nicolasbarre.fr", branchId: "admin_ecole", role: "leader" },
  { email: "0762565a@ac-normandie.fr", branchId: "admin_college", role: "leader" },
  { email: "florian@h-me.fr", branchId: "admin_lycee", role: "leader" },
  { email: "florian@h-me.fr", branchId: "cpe_lycee", role: "leader" },
  { email: "sarah@laprovidence-nicolasbarre.fr", branchId: "cpe_3e4e", role: "leader" },
  { email: "sarah@laprovidence-nicolasbarre.fr", branchId: "cpe_5e6e", role: "leader" },
  { email: "sarah@laprovidence-nicolasbarre.fr", branchId: "vie_scolaire_infirmerie", role: "leader" },
  { email: "florian@h-me.fr", branchId: "vie_scolaire_infirmerie", role: "leader" },
  { email: "m.leblond@laprovidence-nicolasbarre.fr", branchId: "accueil", role: "leader" },
  { email: "florian.hacqueville-mathi@ac-normandie.fr", branchId: "comptabilite", role: "leader" },
  { email: "sarah@laprovidence-nicolasbarre.fr", branchId: "comptabilite", role: "executor" },
  { email: "m.leblond@laprovidence-nicolasbarre.fr", branchId: "comptabilite", role: "executor" },
  { email: SCHOOL.ecole.email, branchId: "direction_ecole", role: "leader" },
  { email: SCHOOL.college.email, branchId: "direction_college", role: "leader" },
  { email: SCHOOL.lycee.email, branchId: "direction_lycee", role: "leader" },
];

const LAPROVIDENCE_TRANSPORT_PROVIDERS = [
  { name: "Perrier", email: "stephanie.fouin@cars-perier.fr" },
  { name: "Reflexe", email: "contact@reflexe-voyages.com" },
  { name: "Grisel", email: "j.saint-denis@grisel-voyages.fr" },
  { name: "Hangard", email: "carole@hangard-autocars.com" },
];

export function laprovidenceSiteIdentity(): SiteIdentity {
  return {
    name: SCHOOL.name,
    shortName: SCHOOL.shortName,
    organizationKind: "groupe",
    onboardingCompleted: true,
    onboardingCompletedAt: new Date().toISOString(),
    address: {
      street: SCHOOL.address.street,
      city: SCHOOL.address.city,
      zip: SCHOOL.address.zip,
      full: SCHOOL.address.full,
      fullCompact: SCHOOL.address.fullCompact,
      mapsEmbed: SCHOOL.address.mapsEmbed,
      mapsItinerary: SCHOOL.address.mapsItinerary,
      latitude: SCHOOL.address.latitude,
      longitude: SCHOOL.address.longitude,
    },
    phone: { ...SCHOOL.phone },
    preinscriptionUrl: SCHOOL.preinscriptionUrl,
    reglementFinancier: SCHOOL.reglementFinancier,
    assistanceEmail: PLATFORM_ASSISTANCE_EMAIL,
    schoolHolidayZone: "B",
  };
}

export function laprovidenceEstablishments(): Establishment[] {
  return [
    {
      id: "ecole",
      label: SCHOOL.ecole.label,
      kind: "ecole",
      directorName: SCHOOL.ecole.directrice,
      directorEmail: SCHOOL.ecole.email,
      colorHex: "#F59E0B",
      grades: SCHOOL.ecole.grades,
      roleSlugs: ["direction_ecole", "direction école"],
      active: true,
    },
    {
      id: "college",
      label: SCHOOL.college.label,
      kind: "college",
      directorName: SCHOOL.college.directrice,
      directorEmail: SCHOOL.college.email,
      colorHex: "#0EA5E9",
      grades: SCHOOL.college.grades,
      roleSlugs: ["direction_college", "direction collège"],
      active: true,
    },
    {
      id: "lycee",
      label: SCHOOL.lycee.label,
      kind: "lycee",
      directorName: SCHOOL.lycee.directrice,
      directorEmail: SCHOOL.lycee.email,
      colorHex: "#F43F5E",
      grades: SCHOOL.lycee.grades,
      roleSlugs: ["direction_lycee", "direction_lycee"],
      active: true,
    },
  ];
}

export function laprovidenceNotifications(): NotificationsConfig {
  return {
    travelsCompta: ["valerie.vasseur@laprovidence-nicolasbarre.fr", "cecile.douaglin@laprovidence-nicolasbarre.fr"],
    travelsCuisine: ["chef.0056isi@newrest.eu"],
    travelsZeendoc: "comptabilite@laprovidence-nicolasbarre.fr",
    hseOps: "sarah.buno@ac-normandie.fr",
    photocopiesOps: "carine.perier@ac-normandie.fr",
    photocopiesOpsEmails: ["carine.perier@ac-normandie.fr"],
    absencesNotifyProfEcole: {
      label: SCHOOL.absences.notifyProfEcole.label,
      email: SCHOOL.absences.notifyProfEcole.email,
    },
    absencesNotifyProfCollege: {
      label: SCHOOL.absences.notifyProfCollegeLycee.label,
      email: SCHOOL.absences.notifyProfCollegeLycee.email,
    },
    absencesNotifyProfLycee: {
      label: SCHOOL.absences.notifyProfCollegeLycee.label,
      email: SCHOOL.absences.notifyProfCollegeLycee.email,
    },
    absencesNotifyProfCollegeLycee: {
      label: SCHOOL.absences.notifyProfCollegeLycee.label,
      email: SCHOOL.absences.notifyProfCollegeLycee.email,
    },
    absencesNotifyOgecCompta: [...SCHOOL.absences.notifyOgecCompta],
    internatRollCallRecipients: {
      appelContact: SCHOOL.lycee.email,
      directionLycee: SCHOOL.lycee.email,
      cpeLycee: "florian@h-me.fr",
      cpeCollege: SCHOOL.requestsRouting.cpeCollege,
    },
    internatEmergencyRecipients: [SCHOOL.lycee.email, "florian@h-me.fr", SCHOOL.requestsRouting.cpeCollege],
  };
}

export function laprovidenceTravelsModule(): TravelsModuleConfig {
  return {
    comptaEmails: ["valerie.vasseur@laprovidence-nicolasbarre.fr", "cecile.douaglin@laprovidence-nicolasbarre.fr"],
    transportProviders: LAPROVIDENCE_TRANSPORT_PROVIDERS.map((p) => ({ ...p })),
    showGroupeScolaireOption: true,
    pdfFooterText: "Groupe Scolaire La Providence Nicolas Barré  ·  Établissement catholique sous contrat avec l'État",
  };
}

export function laprovidenceIntegrations(): IntegrationsConfig {
  return {
    zeendoc: {
      enabled: true,
      buttonLabel: "Envoyer sur Zeendoc",
      destinationEmail: "comptabilite@laprovidence-nicolasbarre.fr",
    },
    microsoftOneDrive: { enabled: true },
  };
}

export function laprovidenceExternalLinks(): ExternalQuickLinkConfig[] {
  return [
    {
      id: "ecole-directe",
      name: "École Directe",
      img: "",
      link: "https://www.ecoledirecte.com/login?cameFrom=%2FAccueil",
      allowedRoles: [
        "direction_college",
        "administratif",
        "professeur",
        "direction_ecole",
        "direction_lycee",
        "maintenance",
        "comptabilite",
        "infirmerie",
        "surveillant",
      ],
    },
    {
      id: "zeendoc",
      name: "ZeenDoc",
      img: "",
      link: "https://armoires.zeendoc.com/_Login/Login.php",
      allowedRoles: ["administratif", "comptabilite", "direction_college", "direction_ecole", "direction_lycee"],
    },
    {
      id: "arena",
      name: "Arena Ac-Normandie",
      img: "",
      link: "https://arena.ac-normandie.fr/arena/",
      allowedRoles: ["administratif", "direction_college", "direction_ecole", "direction_lycee"],
    },
  ];
}
