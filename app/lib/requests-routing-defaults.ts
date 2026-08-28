import type { Establishment, RequestsRoutingConfig, RoutingAssignment, RoutingTask } from "@/app/lib/app-config-schemas";
import { getActiveEstablishments } from "@/app/lib/app-config-establishments";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

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
  rh_personnel: "rh",
};

/** File ticketing dédiée aux demandes ouvertes depuis le module RH. */
export const RH_REQUEST_ROUTE_ID = "rh_personnel";
export const RH_REQUEST_SUBJECT_PREFIX = "[Demande RH]";

export function isManualOnlyDirectionRoute(id: string): boolean {
  return id.startsWith("direction_");
}

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
    active: false,
  },
  {
    id: "admin_college",
    label: "Administratif — collège",
    hint: "Demandes administratives collège",
    keywords: ["collège", "6e", "5e", "4e", "3e"],
    active: false,
  },
  {
    id: "admin_lycee",
    label: "Administratif — lycée",
    hint: "Demandes administratives lycée",
    keywords: ["lycée", "2nde", "1ère", "terminale"],
    active: false,
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
  {
    id: "rh_personnel",
    label: "RH / Personnel",
    hint: "Demandes RH ouvertes depuis le module RH (contrat, dossier, attestation…)",
    keywords: ["rh", "ressources humaines", "personnel", "attestation", "contrat de travail"],
    active: true,
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
      { id: "rh", label: "RH / Personnel", category: "RH" },
    ],
    tasks: DEFAULT_TASKS,
    assignments: [] as RoutingAssignment[],
    directionQueues: [],
    parentPortal: { enabled: false },
    tagCatalog: [],
    personnelTags: [],
  };
}

function buildAssignmentsFromStaffRows(
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

const BUILTIN_ADMIN: Record<"ecole" | "college" | "lycee", string> = {
  ecole: "admin_ecole",
  college: "admin_college",
  lycee: "admin_lycee",
};
const BUILTIN_DIRECTION: Record<"ecole" | "college" | "lycee", string> = {
  ecole: "direction_ecole",
  college: "direction_college",
  lycee: "direction_lycee",
};

function directorEmailForDirectionTask(
  taskId: string,
  establishments: Establishment[],
): string {
  for (const kind of ["ecole", "college", "lycee"] as const) {
    if (taskId === BUILTIN_DIRECTION[kind]) {
      const est = getActiveEstablishments(establishments).find((e) => inferEstablishmentKind(e) === kind);
      const email = est?.directorEmail?.trim().toLowerCase();
      if (email) return email;
    }
  }
  const custom = /^direction_(.+)$/.exec(taskId);
  if (custom?.[1] && !["ecole", "college", "lycee"].includes(custom[1])) {
    const est = getActiveEstablishments(establishments).find((e) => e.id === custom[1]);
    const email = est?.directorEmail?.trim().toLowerCase();
    if (email) return email;
  }
  return "";
}

/** Alimente directionQueues depuis les files direction_* + e-mails directeurs. */
export function syncDirectionQueuesFromTasks(
  routing: RequestsRoutingConfig,
  establishments: Establishment[] = [],
): RequestsRoutingConfig {
  const prevById = new Map(routing.directionQueues.map((q) => [q.id, q]));
  const directionTasks = routing.tasks.filter((t) => isManualOnlyDirectionRoute(t.id));

  const queues = directionTasks.map((task) => {
    const prev = prevById.get(task.id);
    let email = prev?.email?.trim().toLowerCase() || "";
    if (!email) email = directorEmailForDirectionTask(task.id, establishments);
    if (!email) {
      const asg = routing.assignments.find((a) => a.active && a.taskId === task.id);
      email = asg?.email?.trim().toLowerCase() || "";
    }
    return {
      id: task.id,
      label: task.label,
      email,
      active: task.active && Boolean(email),
    };
  });

  return {
    ...routing,
    directionQueues: queues.filter((q) => q.active || prevById.has(q.id)),
  };
}

/** Active / génère les files admin_* et direction_* selon les sites, sans supprimer les files custom. */
export function syncRequestsRoutingWithEstablishments(
  routing: RequestsRoutingConfig,
  establishments: Establishment[],
): RequestsRoutingConfig {
  const active = getActiveEstablishments(establishments);
  const kinds = new Set(active.map((e) => inferEstablishmentKind(e)));
  const defaults = defaultRequestsRouting();
  let tasks = routing.tasks.map((t) => {
    for (const kind of ["ecole", "college", "lycee"] as const) {
      if (t.id === BUILTIN_ADMIN[kind]) return { ...t, active: kinds.has(kind) };
      if (t.id === BUILTIN_DIRECTION[kind]) return { ...t, active: kinds.has(kind) ? t.active : false };
    }
    return t;
  });

  for (const kind of ["ecole", "college", "lycee"] as const) {
    if (!kinds.has(kind)) continue;
    if (!tasks.some((t) => t.id === BUILTIN_ADMIN[kind])) {
      const d = defaults.tasks.find((t) => t.id === BUILTIN_ADMIN[kind]);
      if (d) tasks.push({ ...d, active: true });
    }
    if (!tasks.some((t) => t.id === BUILTIN_DIRECTION[kind])) {
      const d = defaults.tasks.find((t) => t.id === BUILTIN_DIRECTION[kind]);
      if (d) tasks.push({ ...d });
    }
  }

  for (const est of active) {
    const kind = inferEstablishmentKind(est);
    if (kind !== "custom") continue;
    const adminId = `admin_${est.id}`;
    const dirId = `direction_${est.id}`;
    if (!tasks.some((t) => t.id === adminId)) {
      tasks.push({
        id: adminId,
        label: `Administratif — ${est.label}`,
        hint: `Demandes administratives ${est.label}`,
        keywords: [est.label],
        active: true,
      });
    } else {
      tasks = tasks.map((t) => (t.id === adminId ? { ...t, active: true } : t));
    }
    if (!tasks.some((t) => t.id === dirId)) {
      tasks.push({
        id: dirId,
        label: `Direction — ${est.label}`,
        hint: `Transfert manuel vers la direction ${est.label}`,
        keywords: [`direction ${est.label}`],
        active: false,
      });
    }
  }

  return syncDirectionQueuesFromTasks({ ...routing, tasks }, establishments);
}
