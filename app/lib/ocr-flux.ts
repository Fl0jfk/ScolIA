import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";

export const OCR_FLUX_IDS = [
  "eleves_ecole",
  "eleves_college",
  "eleves_lycee",
  "enseignants_ecole",
  "enseignants_college",
  "enseignants_lycee",
  "personnel_ogec",
] as const;

export type OcrFluxId = (typeof OCR_FLUX_IDS)[number];
export type OcrFluxKind = "eleves" | "enseignants" | "personnel";

/** Ancien id unique `enseignants` (bref) → réparti sur les 3 lignes. */
export const UNIFIED_ENSEIGNANTS_FLUX_ID = "enseignants" as const;

export type OcrFluxAssignment = {
  id: OcrFluxId;
  externalUserId?: string;
  match?: string;
  displayName?: string;
  basePath?: string;
};

export type OcrResolvedFlux = {
  id: OcrFluxId;
  kind: OcrFluxKind;
  secteur: Secteur | null;
  basePath: string;
  label: string;
  externalUserId?: string;
  match?: string;
  displayName?: string;
};

export type OcrUserCapabilities = {
  fluxes: OcrResolvedFlux[];
  /** Premier flux élèves — compat stages / ancien profil unique. */
  primaryEleves: OneDriveUserProfile | null;
};

/** Chemin OneDrive unique pour tous les enseignants (école / collège / lycée). */
export const ENSEIGNANTS_SHARED_BASE_PATH = "Dossier enseignants";

export const OCR_FLUX_META: Record<
  OcrFluxId,
  { kind: OcrFluxKind; secteur: Secteur | null; label: string; defaultBasePath: string }
> = {
  eleves_ecole: {
    kind: "eleves",
    secteur: "ecole",
    label: "Élèves école",
    defaultBasePath: "Dossier élèves/École",
  },
  eleves_college: {
    kind: "eleves",
    secteur: "college",
    label: "Élèves collège",
    defaultBasePath: "Dossier élèves/Collège",
  },
  eleves_lycee: {
    kind: "eleves",
    secteur: "lycee",
    label: "Élèves lycée",
    defaultBasePath: "Dossier élèves/Lycée",
  },
  enseignants_ecole: {
    kind: "enseignants",
    secteur: "ecole",
    label: "Enseignants école",
    defaultBasePath: ENSEIGNANTS_SHARED_BASE_PATH,
  },
  enseignants_college: {
    kind: "enseignants",
    secteur: "college",
    label: "Enseignants collège",
    defaultBasePath: ENSEIGNANTS_SHARED_BASE_PATH,
  },
  enseignants_lycee: {
    kind: "enseignants",
    secteur: "lycee",
    label: "Enseignants lycée",
    defaultBasePath: ENSEIGNANTS_SHARED_BASE_PATH,
  },
  personnel_ogec: {
    kind: "personnel",
    secteur: null,
    label: "Personnel OGEC",
    defaultBasePath: "Dossier personnel",
  },
};

export function isOcrFluxId(value: unknown): value is OcrFluxId {
  return typeof value === "string" && (OCR_FLUX_IDS as readonly string[]).includes(value);
}

export function elevesFluxIdForSecteur(secteur: Secteur): OcrFluxId {
  if (secteur === "ecole") return "eleves_ecole";
  if (secteur === "college") return "eleves_college";
  return "eleves_lycee";
}

export function enseignantsFluxIdForSecteur(secteur: Secteur): OcrFluxId {
  if (secteur === "ecole") return "enseignants_ecole";
  if (secteur === "college") return "enseignants_college";
  return "enseignants_lycee";
}

function normalizeMatch(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

export function emptyOcrFluxGrid(): OcrFluxAssignment[] {
  return OCR_FLUX_IDS.map((id) => ({ id }));
}

type LooseFluxRow = {
  id?: string;
  externalUserId?: string;
  match?: string;
  displayName?: string;
  basePath?: string;
};

/**
 * Normalise la grille : 7 flux, chemins enseignants partagés par défaut.
 * Si une ancienne config avait `id: "enseignants"`, on réplique assignee + chemin
 * sur les 3 lignes école / collège / lycée.
 */
export function mergeOcrFluxGrid(raw: OcrFluxAssignment[] | LooseFluxRow[] | undefined | null): OcrFluxAssignment[] {
  const byId = new Map<OcrFluxId, OcrFluxAssignment>();
  let unified: Pick<OcrFluxAssignment, "externalUserId" | "match" | "displayName" | "basePath"> | null =
    null;

  for (const item of raw ?? []) {
    const id = String(item.id ?? "").trim();
    if (id === UNIFIED_ENSEIGNANTS_FLUX_ID) {
      unified = {
        externalUserId: item.externalUserId?.trim() || undefined,
        match: item.match?.trim() || undefined,
        displayName: item.displayName?.trim() || undefined,
        basePath: item.basePath?.trim() || ENSEIGNANTS_SHARED_BASE_PATH,
      };
      continue;
    }
    if (!isOcrFluxId(id)) continue;
    byId.set(id, {
      id,
      externalUserId: item.externalUserId?.trim() || undefined,
      match: item.match?.trim() || undefined,
      displayName: item.displayName?.trim() || undefined,
      basePath: item.basePath?.trim() || undefined,
    });
  }

  if (unified) {
    for (const id of ["enseignants_ecole", "enseignants_college", "enseignants_lycee"] as const) {
      const current = byId.get(id) ?? { id };
      if (!current.externalUserId && !current.match) {
        current.externalUserId = unified.externalUserId;
        current.match = unified.match;
        current.displayName = unified.displayName;
      }
      if (!current.basePath) current.basePath = unified.basePath || ENSEIGNANTS_SHARED_BASE_PATH;
      byId.set(id, current);
    }
  }

  // Chemins vides ou anciens …/École|/Collège|/Lycée → racine commune
  for (const id of ["enseignants_ecole", "enseignants_college", "enseignants_lycee"] as const) {
    const row = byId.get(id) ?? { id };
    const path = row.basePath?.trim();
    if (!path) {
      row.basePath = ENSEIGNANTS_SHARED_BASE_PATH;
    } else {
      const collapsed = path.replace(/\/(École|Ecole|Collège|College|Lycée|Lycee)\s*$/i, "").trim();
      if (collapsed && collapsed !== path) row.basePath = collapsed;
    }
    byId.set(id, row);
  }

  return OCR_FLUX_IDS.map((id) => byId.get(id) ?? { id });
}

type LegacyUserSecteur = {
  externalUserId?: string;
  match?: string;
  displayName?: string;
  secteur?: string;
};

type LegacyBases = Partial<Record<Secteur, { basePath?: string; label?: string }>>;

/** Ancien mapping 1 personne = 1 cycle élèves → grille ocrFlux. */
export function migrateLegacyUserSecteursToOcrFlux(input: {
  ocrFlux?: OcrFluxAssignment[] | null;
  userSecteurs?: LegacyUserSecteur[] | null;
  basesBySecteur?: LegacyBases | null;
  personnelBasePath?: string | null;
}): OcrFluxAssignment[] {
  const grid = mergeOcrFluxGrid(input.ocrFlux);
  const hasAnyAssignee = grid.some((row) => row.externalUserId || row.match);
  if (!hasAnyAssignee && input.userSecteurs?.length) {
    for (const row of input.userSecteurs) {
      const secteur = String(row.secteur ?? "").trim().toLowerCase();
      if (secteur !== "ecole" && secteur !== "college" && secteur !== "lycee") continue;
      const id = elevesFluxIdForSecteur(secteur);
      const current = grid.find((g) => g.id === id);
      if (!current || current.externalUserId || current.match) continue;
      current.externalUserId = row.externalUserId?.trim() || undefined;
      current.match = row.match?.trim() || undefined;
      current.displayName = row.displayName?.trim() || undefined;
    }
  }
  for (const secteur of ["ecole", "college", "lycee"] as const) {
    const override = input.basesBySecteur?.[secteur]?.basePath?.trim();
    if (!override) continue;
    const id = elevesFluxIdForSecteur(secteur);
    const current = grid.find((g) => g.id === id);
    if (current && !current.basePath) current.basePath = override;
  }
  const personnel = grid.find((g) => g.id === "personnel_ogec");
  const rhBase = input.personnelBasePath?.trim();
  if (personnel && !personnel.basePath && rhBase) personnel.basePath = rhBase;
  return grid;
}

export function resolvedBasePath(row: OcrFluxAssignment): string {
  return row.basePath?.trim() || OCR_FLUX_META[row.id].defaultBasePath;
}

export function resolveOcrFluxRow(row: OcrFluxAssignment): OcrResolvedFlux {
  const meta = OCR_FLUX_META[row.id];
  return {
    id: row.id,
    kind: meta.kind,
    secteur: meta.secteur,
    basePath: resolvedBasePath(row),
    label: meta.label,
    externalUserId: row.externalUserId,
    match: row.match,
    displayName: row.displayName,
  };
}

export function fluxesAssignedToUser(
  grid: OcrFluxAssignment[],
  user: { id?: string | null; lastName?: string | null; emails?: string[] },
): OcrResolvedFlux[] {
  const directoryUserId = user.id?.trim();
  const identifiers = [
    ...(user.emails ?? []).map(normalizeMatch),
    user.lastName ? normalizeMatch(user.lastName) : "",
  ].filter(Boolean);

  return grid
    .filter((row) => {
      if (directoryUserId && row.externalUserId?.trim() === directoryUserId) return true;
      const target = normalizeMatch(row.match ?? "");
      if (!target) return false;
      return identifiers.some((id) => id === target || id.includes(target) || target.includes(id));
    })
    .map(resolveOcrFluxRow);
}

export function capabilitiesFromFluxes(fluxes: OcrResolvedFlux[]): OcrUserCapabilities {
  const eleves = fluxes.filter((f) => f.kind === "eleves" && f.secteur);
  const first = eleves[0];
  return {
    fluxes,
    primaryEleves: first
      ? {
          key: first.secteur!,
          secteur: first.secteur!,
          basePath: first.basePath,
          label: first.label,
        }
      : null,
  };
}

export function elevesSecteursFromCapabilities(caps: OcrUserCapabilities | null): Secteur[] {
  if (!caps) return [];
  return caps.fluxes
    .filter((f) => f.kind === "eleves" && f.secteur)
    .map((f) => f.secteur as Secteur);
}

export function enseignantsSecteursFromCapabilities(caps: OcrUserCapabilities | null): Secteur[] {
  if (!caps) return [];
  return caps.fluxes
    .filter((f) => f.kind === "enseignants" && f.secteur)
    .map((f) => f.secteur as Secteur);
}

export function hasEnseignantsFlux(caps: OcrUserCapabilities | null): boolean {
  return enseignantsSecteursFromCapabilities(caps).length > 0;
}

export function hasPersonnelFlux(caps: OcrUserCapabilities | null): boolean {
  return Boolean(caps?.fluxes.some((f) => f.kind === "personnel"));
}

/** True si le compte range aussi des enseignants ou du personnel — hors chemin élèves pur. */
export function ocrHasExtraFluxes(caps: OcrUserCapabilities | null): boolean {
  return Boolean(caps?.fluxes.some((f) => f.kind !== "eleves"));
}

/**
 * Chemin enseignants : même racine pour tous les cycles (fusion OneDrive).
 * Si plusieurs flux enseignants, on prend le premier chemin non vide.
 */
export function findFluxBasePath(
  caps: OcrUserCapabilities | null,
  kind: OcrFluxKind,
  secteur?: Secteur | null,
): string | null {
  if (!caps) return null;
  if (kind === "enseignants") {
    const ens = caps.fluxes.filter((f) => f.kind === "enseignants");
    if (ens.length === 0) return null;
    if (secteur) {
      const exact = ens.find((f) => f.secteur === secteur);
      if (exact?.basePath) return exact.basePath;
    }
    return ens[0]?.basePath ?? ENSEIGNANTS_SHARED_BASE_PATH;
  }
  const hit = caps.fluxes.find((f) => {
    if (f.kind !== kind) return false;
    if (kind === "personnel") return true;
    return !secteur || f.secteur === secteur;
  });
  return hit?.basePath ?? null;
}

export function profileFromCapabilities(caps: OcrUserCapabilities | null): OneDriveUserProfile | null {
  return caps?.primaryEleves ?? null;
}

export function capabilitiesFromLegacyProfile(
  profile: OneDriveUserProfile | null,
): OcrUserCapabilities {
  if (!profile) return { fluxes: [], primaryEleves: null };
  const id = elevesFluxIdForSecteur(profile.secteur);
  return {
    fluxes: [
      {
        id,
        kind: "eleves",
        secteur: profile.secteur,
        basePath: profile.basePath,
        label: profile.label,
      },
    ],
    primaryEleves: profile,
  };
}
