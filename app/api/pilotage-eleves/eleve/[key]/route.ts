import { NextRequest, NextResponse } from "next/server";
import { getDirectoryUserRoles } from "@/app/lib/directory-members";
import { requireAuth } from "@/app/lib/intranet-auth";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import {
  canAccessPilotageModule,
  resolvePilotageSecteursForRoles,
} from "@/app/lib/pilotage-eleves-access";
import { appendPilotageAudit, findEleveRow, loadPilotageDossier } from "@/app/lib/pilotage-eleves";
import { resolveEleveFolderName } from "@/app/lib/eleves-config";
import { canonicalClasseLabel, computeDropSignal, computeNiveauAverages, buildDeterministicFlashPoints, focusDossierOnRecentYears, sortBulletinsChrono } from "@/app/lib/pilotage-eleves-logic";

type Ctx = { params: Promise<{ key: string }> };

export async function GET(_req: NextRequest, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const roles = await getDirectoryUserRoles(gate.ctx.userId);
  if (!canAccessPilotageModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { key } = await ctx.params;
  const allowed = await resolvePilotageSecteursForRoles(roles, gate.ctx.userId);
  const row = await findEleveRow(key, allowed);
  if (!row) return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });

  const dossier = await loadPilotageDossier(row.secteur, row.key);
  void appendPilotageAudit({
    userId: gate.ctx.userId,
    key: row.key,
    secteur: row.secteur,
    classe: row.classe,
  }).catch((e) => console.error("[pilotage] audit:", e));

  const bulletinsAll = dossier ? sortBulletinsChrono(dossier.bulletins) : [];
  const allNiveaux = dossier
    ? dossier.moyennesParNiveau ?? computeNiveauAverages(bulletinsAll)
    : [];
  const focus = dossier
    ? focusDossierOnRecentYears({
        classe: row.classe || dossier.classe,
        flags: dossier.flags,
        bulletins: bulletinsAll,
        moyennesParNiveau: allNiveaux,
        extraSignalIds: dossier.synthese?.signals?.map((s) => s.id),
      })
    : null;
  const points =
    dossier?.synthese?.points?.length
      ? dossier.synthese.points.slice(0, 3)
      : dossier && focus
        ? buildDeterministicFlashPoints({
            flags: dossier.flags,
            drop: focus.drop,
            bulletins: focus.bulletins,
            moyennesParNiveau: focus.niveaux,
            classe: row.classe,
          })
        : [];

  return NextResponse.json({
    eleve: {
      key: row.key,
      nom: row.nom,
      prenom: row.prenom,
      classe: canonicalClasseLabel(row.classe),
      folderName: resolveEleveFolderName(row),
      secteur: row.secteur as Secteur,
      ine: row.ine || undefined,
    },
    dossier: dossier
      ? {
          ...dossier,
          classe: row.classe || dossier.classe,
          bulletins: focus?.bulletins ?? bulletinsAll,
          moyennesParNiveau: focus?.niveaux ?? allNiveaux,
          drop: focus?.drop ?? dossier.drop ?? computeDropSignal(bulletinsAll),
          synthese: {
            text: dossier.synthese?.text,
            points,
            signals: focus?.signals ?? dossier.synthese?.signals ?? [],
            mood: focus?.mood ?? dossier.synthese?.mood,
            updatedAt: dossier.synthese?.updatedAt ?? "",
            sources: dossier.synthese?.sources ?? [],
          },
        }
      : null,
  });
}
