import { NextResponse } from "next/server";

import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canBrowseStageConventions,
  canViewReferentConventions,
  resolveStageViewerRole,
} from "@/app/lib/stage-access";
import { getElevePhotoUrl } from "@/app/lib/eleve-photos";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import {
  getStageWatchersConfig,
  listWatcherAssignmentsForUser,
} from "@/app/lib/stage-watchers-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

/**
 * Proxy photo pour le hub Stages — même auth que /api/stages.
 * Le hub renvoie ces URLs sans pré-signer N objets S3 (gain ~400 ms).
 */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  const viewer = resolveStageViewerRole(roles);
  const watchersCfg = await getStageWatchersConfig(currentStageSchoolYear());
  const watcherAssignments = listWatcherAssignmentsForUser(watchersCfg, gate.ctx.userId);
  if (
    !viewer &&
    watcherAssignments.length === 0 &&
    !canBrowseStageConventions(roles) &&
    !canViewReferentConventions(roles)
  ) {
    return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
  }

  const eleveId = new URL(req.url).searchParams.get("eleveId")?.trim() || "";
  if (!eleveId) {
    return NextResponse.json({ error: "eleveId requis." }, { status: 400 });
  }

  const eleves = await loadElevesRegistry().catch(() => []);
  const eleve = eleves.find((e) => e.id === eleveId);
  if (!eleve) {
    return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  }

  const url = await getElevePhotoUrl({
    nom: eleve.nom,
    prenom: eleve.prenom,
    ine: eleve.ine || "",
    photoKey: eleve.photoKey ?? undefined,
    folderName: eleve.folderName || "",
  });
  if (!url) {
    return NextResponse.json({ error: "Photo introuvable." }, { status: 404 });
  }
  return NextResponse.redirect(url);
}
