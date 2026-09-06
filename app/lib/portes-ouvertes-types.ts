import { DEFAULT_CLASSES_BY_POLE } from "@/app/lib/school-classes-catalog";

/** Cycle scolaire pour une visite portes ouvertes. */
export type PortesOuvertesCycle = "ecole" | "college" | "lycee";

export type PortesOuvertesRegistrationSource = "public" | "accueil";

export type PortesOuvertesRegistration = {
  id: string;
  slotId: string;
  /** Snapshot créneau (conservé même si le créneau est retiré de la config). */
  slotLabel?: string;
  slotStartAt?: string;
  slotEndAt?: string;
  firstName: string;
  lastName: string;
  email: string;
  phone?: string;
  /** Texte libre legacy (inscription publique). */
  childrenInfo?: string;
  /** Prénom de l’enfant concerné par la visite. */
  childFirstName?: string;
  /** Nom de l’enfant concerné par la visite. */
  childLastName?: string;
  /** Cycle demandé (école / collège / lycée). */
  cycle?: PortesOuvertesCycle;
  /** Classe / niveau souhaité pour la visite. */
  classeSouhaitee?: string;
  consent: boolean;
  source?: PortesOuvertesRegistrationSource;
  recordedBy?: {
    userId: string;
    name: string;
  };
  lastModifiedBy?: {
    userId: string;
    name: string;
  };
  createdAt: string;
  updatedAt?: string;
};

export const PORTES_OUVERTES_CYCLE_LABELS: Record<PortesOuvertesCycle, string> = {
  ecole: "École",
  college: "Collège",
  lycee: "Lycée",
};

export const PORTES_OUVERTES_CYCLES: PortesOuvertesCycle[] = ["ecole", "college", "lycee"];

/** Classes proposées par cycle (catalogue établissement). */
export function classesForPortesOuvertesCycle(cycle: PortesOuvertesCycle): string[] {
  if (cycle === "ecole") return [...(DEFAULT_CLASSES_BY_POLE.ÉCOLE || [])];
  if (cycle === "college") return [...(DEFAULT_CLASSES_BY_POLE.COLLÈGE || [])];
  return [...(DEFAULT_CLASSES_BY_POLE.LYCÉE || [])];
}

export function isPortesOuvertesCycle(v: unknown): v is PortesOuvertesCycle {
  return v === "ecole" || v === "college" || v === "lycee";
}

/**
 * Cycles proposés selon les établissements actifs (kind ecole/college/lycee).
 * Si aucun kind standard n’est trouvé (ex. uniquement « custom »), les 3 cycles restent proposés.
 */
export function cyclesFromActiveEstablishments(
  establishments: ReadonlyArray<{ kind?: string | null; active?: boolean | null }>,
): PortesOuvertesCycle[] {
  const active = establishments.filter((e) => e.active !== false);
  const found = new Set<PortesOuvertesCycle>();
  for (const e of active) {
    if (e.kind === "ecole" || e.kind === "college" || e.kind === "lycee") {
      found.add(e.kind);
    }
  }
  const ordered = PORTES_OUVERTES_CYCLES.filter((c) => found.has(c));
  return ordered.length > 0 ? ordered : [...PORTES_OUVERTES_CYCLES];
}

/** Libellé visite (cycle + enfant + classe) pour mails / listes. */
export function portesOuvertesVisitLine(params: {
  cycle?: PortesOuvertesCycle;
  childFirstName?: string;
  childLastName?: string;
  classeSouhaitee?: string;
  childrenInfo?: string;
}): string {
  const cycleLabel = params.cycle ? PORTES_OUVERTES_CYCLE_LABELS[params.cycle] : "";
  const childName = [params.childFirstName, params.childLastName].filter(Boolean).join(" ").trim();
  const classe = (params.classeSouhaitee || "").trim();
  const structured = [cycleLabel, childName, classe].filter(Boolean).join(" — ");
  if (structured) return structured;
  return (params.childrenInfo || "").trim();
}

/** Créneau encore à venir (modifiable). */
export function isPortesOuvertesRegistrationUpcoming(
  reg: PortesOuvertesRegistration,
  nowMs = Date.now(),
): boolean {
  const endIso = reg.slotEndAt || reg.slotStartAt;
  if (!endIso) return false;
  const t = Date.parse(endIso);
  return Number.isFinite(t) && t > nowMs;
}
