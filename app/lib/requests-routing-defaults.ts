import type { RequestsRoutingConfig, RoutingAssignment, RoutingTask } from "@/app/lib/app-config-schemas";

const BRANCH_TO_SERVICE: Record<string, string> = {
  corbeille: "etablissement",
  maintenance: "maintenance",
  admin_ecole: "administratif",
  admin_college: "administratif",
  admin_lycee: "administratif",
  cpe_lycee: "vie_scolaire",
  cpe_3e4e: "vie_scolaire",
  cpe_5e6e: "vie_scolaire",
  vie_scolaire_infirmerie: "vie_scolaire",
  accueil: "accueil",
  comptabilite: "comptabilite",
  direction_ecole: "direction",
  direction_college: "direction",
  direction_lycee: "direction",
};

function uid(prefix: string) {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
}

const DEFAULT_TASKS: RoutingTask[] = [
  {
    id: "corbeille",
    label: "Corbeille établissement",
    hint: "Demandes générales ou non classées",
    keywords: [],
    active: true,
  },
  {
    id: "maintenance",
    label: "Maintenance",
    hint: "Problèmes techniques, réparations, bâtiment",
    keywords: ["maintenance", "réparation", "panne", "fuite", "électricité"],
    active: true,
  },
  {
    id: "admin_ecole",
    label: "Administratif — école",
    hint: "Demandes administratives école",
    keywords: ["école", "maternelle", "élémentaire"],
    active: true,
  },
  {
    id: "admin_college",
    label: "Administratif — collège",
    hint: "Demandes administratives collège",
    keywords: ["collège", "6e", "5e", "4e", "3e"],
    active: true,
  },
  {
    id: "admin_lycee",
    label: "Administratif — lycée",
    hint: "Demandes administratives lycée",
    keywords: ["lycée", "2nde", "1ère", "terminale"],
    active: true,
  },
  {
    id: "comptabilite",
    label: "Comptabilité",
    hint: "Factures, paiements, budgets",
    keywords: ["compta", "facture", "paiement", "budget"],
    active: true,
  },
  {
    id: "direction_ecole",
    label: "Direction — école",
    hint: "Transfert manuel vers la direction école",
    keywords: ["direction école"],
    active: false,
  },
  {
    id: "direction_college",
    label: "Direction — collège",
    hint: "Transfert manuel vers la direction collège",
    keywords: ["direction collège"],
    active: false,
  },
  {
    id: "direction_lycee",
    label: "Direction — lycée",
    hint: "Transfert manuel vers la direction lycée",
    keywords: ["direction lycée"],
    active: false,
  },
];

export function defaultRequestsRouting(): RequestsRoutingConfig {
  return {
    version: 1,
    services: [
      { id: "etablissement", label: "Établissement (corbeille)", category: "Établissement" },
      { id: "maintenance", label: "Maintenance", category: "Établissement" },
      { id: "administratif", label: "Administratif", category: "Scolarité" },
      { id: "vie_scolaire", label: "Vie scolaire", category: "Vie scolaire" },
      { id: "accueil", label: "Accueil", category: "Établissement" },
      { id: "comptabilite", label: "Comptabilité", category: "Finances" },
      { id: "direction", label: "Direction", category: "Direction", manualOnly: true },
    ],
    tasks: DEFAULT_TASKS,
    assignments: [] as RoutingAssignment[],
    directionQueues: [],
    parentPortal: { enabled: false },
  };
}

export function buildAssignmentsFromStaffRows(
  rows: { email: string; branchId: string; role: string }[],
): RoutingAssignment[] {
  return rows
    .filter((r) => r.branchId !== "corbeille")
    .map((r) => ({
      id: uid("asg"),
      taskId: r.branchId,
      email: r.email,
      personName: r.email.split("@")[0] || r.email,
      serviceId: BRANCH_TO_SERVICE[r.branchId] || "administratif",
      active: true,
    }));
}

export { BRANCH_TO_SERVICE };
