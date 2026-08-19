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

export type OcrFluxAssignment = {
  id: OcrFluxId;
  clerkUserId?: string;
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
  clerkUserId?: string;
  match?: string;
  displayName?: string;
};

export type OcrUserCapabilities = {
  fluxes: OcrResolvedFlux[];
  /** Premier flux élèves — compat stages / ancien profil unique. */
  primaryEleves: OneDriveUserProfile | null;
};

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
    defaultBasePath: "Dossier enseignants/École",
  },
  enseignants_college: {
    kind: "enseignants",
    secteur: "college",
    label: "Enseignants collège",
    defaultBasePath: "Dossier enseignants/Collège",
  },
  enseignants_lycee: {
    kind: "enseignants",
    secteur: "lycee",
    label: "Enseignants lycée",
    defaultBasePath: "Dossier enseignants/Lycée",
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

export function mergeOcrFluxGrid(raw: OcrFluxAssignment[] | undefined | null): OcrFluxAssignment[] {
  const byId = new Map<OcrFluxId, OcrFluxAssignment>();
  for (const row of raw ?? []) {
    if (!isOcrFluxId(row.id)) continue;
    byId.set(row.id, {
      id: row.id,
      clerkUserId: row.clerkUserId?.trim() || undefined,
      match: row.match?.trim() || undefined,
      displayName: row.displayName?.trim() || undefined,
      basePath: row.basePath?.trim() || undefined,
    });
  }
  return OCR_FLUX_IDS.map((id) => byId.get(id) ?? { id });
}

type LegacyUserSecteur = {
  clerkUserId?: string;
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
  const hasAnyAssignee = grid.some((row) => row.clerkUserId || row.match);
  if (!hasAnyAssignee && input.userSecteurs?.length) {
    for (const row of input.userSecteurs) {
      const secteur = String(row.secteur ?? "").trim().toLowerCase();
      if (secteur !== "ecole" && secteur !== "college" && secteur !== "lycee") continue;
      const id = elevesFluxIdForSecteur(secteur);
      const current = grid.find((g) => g.id === id);
      if (!current || current.clerkUserId || current.match) continue;
      current.clerkUserId = row.clerkUserId?.trim() || undefined;
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
    clerkUserId: row.clerkUserId,
    match: row.match,
    displayName: row.displayName,
  };
}

export function fluxesAssignedToUser(
  grid: OcrFluxAssignment[],
  user: { id?: string | null; lastName?: string | null; emails?: string[] },
): OcrResolvedFlux[] {
  const clerkId = user.id?.trim();
  const identifiers = [
    ...(user.emails ?? []).map(normalizeMatch),
    user.lastName ? normalizeMatch(user.lastName) : "",
  ].filter(Boolean);

  return grid
    .filter((row) => {
      if (clerkId && row.clerkUserId?.trim() === clerkId) return true;
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

export function hasPersonnelFlux(caps: OcrUserCapabilities | null): boolean {
  return Boolean(caps?.fluxes.some((f) => f.kind === "personnel"));
}

/** True si le compte range aussi des enseignants ou du personnel — hors chemin élèves pur. */
export function ocrHasExtraFluxes(caps: OcrUserCapabilities | null): boolean {
  return Boolean(caps?.fluxes.some((f) => f.kind !== "eleves"));
}

export function findFluxBasePath(
  caps: OcrUserCapabilities | null,
  kind: OcrFluxKind,
  secteur?: Secteur | null,
): string | null {
  if (!caps) return null;
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
