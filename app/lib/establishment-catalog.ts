import type { Establishment, EstablishmentKind } from "@/app/lib/app-config-schemas";
import {
  getActiveEstablishments,
  shouldShowGroupeScolaire,
} from "@/app/lib/app-config-establishments";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { normRole } from "@/app/lib/intranet-role-utils";

export const GROUPE_SCOLAIRE_ID = "groupe_scolaire";
export const GROUPE_SCOLAIRE_LABEL = "Groupe Scolaire";

export const DIRECTION_ROLE_BY_KIND: Record<EstablishmentKind, string> = {
  ecole: "direction_ecole",
  college: "direction_college",
  lycee: "direction_lycee",
  custom: "direction",
};

const KIND_ALIASES: Record<EstablishmentKind, string[]> = {
  ecole: ["ecole", "école", "primaire", "elementaire", "élémentaire", "maternelle"],
  college: ["college", "collège"],
  lycee: ["lycee", "lycée"],
  custom: [],
};

export function directionRoleForKind(kind: EstablishmentKind | string | undefined): string {
  if (kind === "ecole" || kind === "college" || kind === "lycee" || kind === "custom") {
    return DIRECTION_ROLE_BY_KIND[kind];
  }
  return DIRECTION_ROLE_BY_KIND[inferEstablishmentKind({ kind })];
}

export function clerkRoleSlugsForEstablishment(est: {
  kind?: string;
  id?: string;
  label?: string;
}): string[] {
  return [directionRoleForKind(inferEstablishmentKind(est))];
}

function fold(s: string): string {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");
}

function kindFromAlias(raw: string): EstablishmentKind | null {
  const folded = fold(raw);
  for (const kind of ["ecole", "college", "lycee"] as EstablishmentKind[]) {
    if (KIND_ALIASES[kind].some((a) => fold(a) === folded || folded === kind)) return kind;
  }
  return null;
}

/** Résout un établissement par id, libellé exact, libellé normalisé, puis alias de kind (École ↔ ecole). */
export function matchEstablishment(
  establishments: Establishment[],
  ref: string | null | undefined,
  opts?: { includeInactive?: boolean },
): Establishment | null {
  const raw = String(ref || "").trim();
  if (!raw || raw === GROUPE_SCOLAIRE_LABEL || raw === GROUPE_SCOLAIRE_ID) return null;
  const list = opts?.includeInactive ? establishments : getActiveEstablishments(establishments);
  const byId = list.find((e) => e.id === raw);
  if (byId) return byId;
  const byLabel = list.find((e) => e.label === raw);
  if (byLabel) return byLabel;
  const folded = fold(raw);
  const byFolded = list.find((e) => fold(e.label) === folded || fold(e.id) === folded);
  if (byFolded) return byFolded;
  const kind = kindFromAlias(raw) || inferEstablishmentKind({ label: raw, id: raw });
  const byKind = list.filter((e) => inferEstablishmentKind(e) === kind);
  if (byKind.length === 1) return byKind[0]!;
  return byKind.find((e) => e.id === kind) || null;
}

export type EstablishmentSelectOption = {
  id: string;
  label: string;
  kind: EstablishmentKind;
  isGroupe?: boolean;
};

export function establishmentSelectOptions(
  establishments: Establishment[],
  opts?: { includeGroupe?: boolean; kinds?: EstablishmentKind[] },
): EstablishmentSelectOption[] {
  let active = getActiveEstablishments(establishments);
  if (opts?.kinds?.length) {
    const allowed = new Set(opts.kinds);
    active = active.filter((e) => allowed.has(inferEstablishmentKind(e)));
  }
  const options: EstablishmentSelectOption[] = active.map((e) => ({
    id: e.id,
    label: e.label,
    kind: inferEstablishmentKind(e),
  }));
  const wantGroupe = opts?.includeGroupe !== false && shouldShowGroupeScolaire(establishments);
  if (wantGroupe && !opts?.kinds) {
    options.push({
      id: GROUPE_SCOLAIRE_ID,
      label: GROUPE_SCOLAIRE_LABEL,
      kind: "custom",
      isGroupe: true,
    });
  }
  return options;
}

export function establishmentChoiceOptions(
  establishments: Establishment[],
  opts?: { includeGroupe?: boolean; kinds?: EstablishmentKind[] },
): Array<{ value: string; label: string }> {
  return establishmentSelectOptions(establishments, { includeGroupe: false, ...opts }).map((o) => ({
    value: o.label,
    label: o.label,
  }));
}

export function presentEstablishmentKinds(establishments: Establishment[]): EstablishmentKind[] {
  const set = new Set<EstablishmentKind>();
  for (const e of getActiveEstablishments(establishments)) {
    set.add(inferEstablishmentKind(e));
  }
  const order: EstablishmentKind[] = ["ecole", "college", "lycee", "custom"];
  return order.filter((k) => set.has(k));
}

/** Internat : collège / lycée / autre — pas l’école primaire. */
export function internatEligibleEstablishments(establishments: Establishment[]): Establishment[] {
  return getActiveEstablishments(establishments).filter((e) => {
    const kind = inferEstablishmentKind(e);
    return kind === "college" || kind === "lycee" || kind === "custom";
  });
}

export function isGroupeScolaireRef(ref: string | null | undefined): boolean {
  const raw = String(ref || "").trim();
  return raw === GROUPE_SCOLAIRE_LABEL || raw === GROUPE_SCOLAIRE_ID;
}

export function userRolesMatchSlug(roles: string[], slug: string): boolean {
  const n = roles.map(normRole);
  const s = normRole(slug);
  if (!s) return false;
  return n.some((r) => r === s || r.includes(s) || s.includes(r));
}

export function isAnyDirectionRole(roles: string[]): boolean {
  return INTRANET_DIRECTION_SLUGS.some((slug) => userRolesMatchSlug(roles, slug));
}

function rolesFromUser(user: {
  publicMetadata?: Record<string, unknown> | null;
} | null | undefined): string[] {
  if (!user?.publicMetadata) return [];
  const raw = user.publicMetadata.role;
  return Array.isArray(raw) ? raw.map(String) : raw ? [String(raw)] : [];
}

export function userCanActAsDirectionFor(
  user: { id?: string | null; publicMetadata?: Record<string, unknown> | null } | null | undefined,
  establishments: Establishment[],
  etabRef: string | null | undefined,
  rolesOverride?: string[],
): boolean {
  const est = matchEstablishment(establishments, etabRef);
  if (!est) return false;
  if (est.directorClerkUserId && user?.id && est.directorClerkUserId === user.id) return true;
  const roles = rolesOverride ?? rolesFromUser(user);
  const slugs =
    est.clerkRoleSlugs && est.clerkRoleSlugs.length > 0
      ? est.clerkRoleSlugs
      : clerkRoleSlugsForEstablishment(est);
  return slugs.some((s) => userRolesMatchSlug(roles, s));
}

export function userIsAnyDirection(
  user: { id?: string | null; publicMetadata?: Record<string, unknown> | null } | null | undefined,
  establishments: Establishment[] = [],
  rolesOverride?: string[],
): boolean {
  const roles = rolesOverride ?? rolesFromUser(user);
  if (isAnyDirectionRole(roles)) return true;
  if (user?.id) {
    return getActiveEstablishments(establishments).some((e) => e.directorClerkUserId === user.id);
  }
  return false;
}

export function rolesCanManageEstablishmentLabel(
  roles: string[],
  establishments: Establishment[],
  etabRef: string | null | undefined,
  userId?: string | null,
): boolean {
  return userCanActAsDirectionFor(
    { id: userId, publicMetadata: { role: roles } },
    establishments,
    etabRef,
    roles,
  );
}

export function directionRolesMatchEstablishmentRef(
  roles: string[],
  etabRef: string | null | undefined,
  establishments: Establishment[] = [],
  userId?: string | null,
): boolean {
  if (establishments.length > 0) {
    return rolesCanManageEstablishmentLabel(roles, establishments, etabRef, userId);
  }
  if (!etabRef || isGroupeScolaireRef(etabRef)) return false;
  return userRolesMatchSlug(
    roles,
    directionRoleForKind(inferEstablishmentKind({ label: etabRef })),
  );
}
