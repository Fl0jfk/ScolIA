import { NextRequest, NextResponse } from "next/server";
import { getClerkUserRoles } from "@/app/lib/clerk-users";
import { requireAuth } from "@/app/lib/intranet-auth";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import {
  canAccessPilotageModule,
  resolvePilotageSecteursForRoles,
} from "@/app/lib/pilotage-eleves-access";
import { appendPilotageAudit, findEleveRow, loadPilotageDossier } from "@/app/lib/pilotage-eleves";
import { resolveEleveFolderName } from "@/app/lib/eleves-config";
import { computeDropSignal } from "@/app/lib/pilotage-eleves-logic";

type Ctx = { params: Promise<{ key: string }> };

export async function GET(_req: NextRequest, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const roles = await getClerkUserRoles(gate.ctx.userId);
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

  return NextResponse.json({
    eleve: {
      key: row.key,
      nom: row.nom,
      prenom: row.prenom,
      classe: row.classe ?? "",
      folderName: resolveEleveFolderName(row),
      secteur: row.secteur as Secteur,
      ine: row.ine || undefined,
    },
    dossier: dossier
      ? { ...dossier, drop: dossier.drop ?? computeDropSignal(dossier.bulletins) }
      : null,
  });
}
