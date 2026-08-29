import { hasRole } from "@/app/lib/intranet-role-utils";
import type { PersonnelProfile, PersonnelRecord, PersonnelRhSpace } from "@/app/lib/personnel-types";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import type { MetaRhDocument } from "@/app/lib/rh/types";

/** Phase affichée dans l’onglet Tableau de bord RH (espace personnel). */
export type RhEspacePhase = "onboarding" | "pending_validation" | "active" | "no_dossier";

export function defaultPersonnelRhSpace(): PersonnelRhSpace {
  return { status: "onboarding" };
}

export function resolveRhEspacePhaseFromMeta(meta: MetaRhDocument | null): RhEspacePhase {
  if (!meta) return "no_dossier";
  if (meta.accountStatus === "active") return "active";
  const ob = meta.onboarding?.status;
  if (ob === "termine") return "active";
  if (ob === "soumis" || ob === "validation_rh" || meta.accountStatus === "pending") {
    return "pending_validation";
  }
  return "onboarding";
}

export function resolveRhEspacePhaseFromPersonnel(record: PersonnelRecord | null): RhEspacePhase {
  if (!record) return "no_dossier";
  const s = record.rhSpace?.status;
  if (!s) return "active";
  if (s === "active") return "active";
  if (s === "pending_validation") return "pending_validation";
  return "onboarding";
}

export function resolveRhEspacePhase(input: {
  roles: string[];
  meta: MetaRhDocument | null;
  postgresRecord: PersonnelRecord | null;
}): RhEspacePhase {
  if (canManagePersonnel(input.roles) || hasRole(input.roles, "admin")) {
    return "active";
  }
  if (input.meta) return resolveRhEspacePhaseFromMeta(input.meta);
  if (input.postgresRecord) return resolveRhEspacePhaseFromPersonnel(input.postgresRecord);
  return "onboarding";
}

export function isRhIdentityComplete(input: {
  profile?: PersonnelProfile;
  metaIdentity?: MetaRhDocument["identity"];
}): boolean {
  const id = input.metaIdentity;
  if (id) {
    return Boolean(
      id.birthDate?.trim() &&
        id.birthPlace?.trim() &&
        id.address?.line1?.trim() &&
        id.address?.postalCode?.trim() &&
        id.address?.city?.trim() &&
        (id.phoneMobile?.trim() || id.phone?.trim()),
    );
  }
  const p = input.profile;
  return Boolean(
    p?.birthDate?.trim() &&
      p?.birthPlace?.trim() &&
      p?.addressLine1?.trim() &&
      p?.postalCode?.trim() &&
      p?.city?.trim() &&
      (p?.phoneMobile?.trim() || p?.phone?.trim()),
  );
}

export function canValidateRhEspace(roles: string[]): boolean {
  return canManagePersonnel(roles);
}
