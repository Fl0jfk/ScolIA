import { hasRole, normRole } from "@/app/lib/intranet-role-utils";

export type PhotocopiesEtablissement = "École" | "Collège" | "Lycée";

export type PhotocopiesRecordLike = {
  etablissement: PhotocopiesEtablissement | string;
  createdBy: { userId: string };
};

export function getPhotocopiesRoleFlags(roles: string[]) {
  const n = roles.map(normRole);
  return {
    isDirectionEcole: n.some((r) => r.includes("direction") && r.includes("ecole")),
    isDirectionCollege: n.some((r) => r.includes("direction") && r.includes("college")),
    isDirectionLycee: n.some((r) => r.includes("direction") && r.includes("lycee")),
    isAdministratif: n.some((r) => r.includes("administratif")) || hasRole(roles, "administratif"),
    isProfesseur: n.some((r) => r.includes("professeur")),
    isEducation: n.some((r) => r.includes("education") || r === "cpe"),
  };
}

export function canCreatePhotocopiesDemand(roles: string[]) {
  const f = getPhotocopiesRoleFlags(roles);
  return f.isProfesseur || f.isAdministratif || f.isEducation;
}

export function canManagePhotocopiesDemand(rec: PhotocopiesRecordLike, roles: string[]) {
  const f = getPhotocopiesRoleFlags(roles);
  if (rec.etablissement === "École") return f.isDirectionEcole;
  if (rec.etablissement === "Collège") return f.isDirectionCollege;
  if (rec.etablissement === "Lycée") return f.isDirectionLycee;
  return false;
}

export function canViewPhotocopiesDemand(rec: PhotocopiesRecordLike, userId: string, roles: string[]) {
  if (rec.createdBy.userId === userId) return true;
  return canManagePhotocopiesDemand(rec, roles);
}
