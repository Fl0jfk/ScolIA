import { NextRequest, NextResponse } from "next/server";
import { getDirectoryUserRoles } from "@/app/lib/directory-members";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveEleveFolderName } from "@/app/lib/eleves-config";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { oneDrivePathForEleve } from "@/app/lib/onedrive-eleves";
import { resolveOcrCapabilitiesForUserServer } from "@/app/lib/onedrive-user-profiles.server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { elevesSecteursFromCapabilities } from "@/app/lib/ocr-flux";
import {
  canIndexPilotage,
  resolvePilotageSecteursForRoles,
} from "@/app/lib/pilotage-eleves-access";
import { findEleveRow, listClassRoster, listElevesForSecteurs } from "@/app/lib/pilotage-eleves";
import { refreshPilotageEleveDossier } from "@/app/lib/pilotage-eleves-analyze";

export const maxDuration = 120;

export async function POST(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const roles = await getDirectoryUserRoles(gate.ctx.userId);
  if (!canIndexPilotage(roles)) {
    return NextResponse.json({ error: "Seul le secrétariat peut indexer OneDrive." }, { status: 403 });
  }

  const body = await req.json();
  const accessToken = String(body.accessToken ?? "").trim();
  if (!accessToken) {
    return NextResponse.json({ error: "Connexion Microsoft requise." }, { status: 400 });
  }

  const allowed = await resolvePilotageSecteursForRoles(roles, gate.ctx.userId);
  const user = await safeCurrentUser();
  const caps = user ? await resolveOcrCapabilitiesForUserServer(user) : null;
  const ocrSecteurs = elevesSecteursFromCapabilities(caps);
  const basePathFor = (secteur: Secteur): string => {
    const flux = caps?.fluxes.find((f) => f.kind === "eleves" && f.secteur === secteur);
    return flux?.basePath || (caps?.primaryEleves?.basePath ?? "");
  };

  const key = String(body.key ?? "").trim();
  const classe = String(body.classe ?? "").trim();
  const secteurParam = String(body.secteur ?? "").trim() as Secteur | "";
  const syncAll = Boolean(body.syncAll);

  if (syncAll && secteurParam) {
    if (!allowed.includes(secteurParam)) {
      return NextResponse.json({ error: "Secteur non autorisé." }, { status: 403 });
    }
    if (ocrSecteurs.length && !ocrSecteurs.includes(secteurParam)) {
      return NextResponse.json({ error: "Ce compte OCR n’alimente pas cet établissement." }, { status: 403 });
    }
    const basePath = basePathFor(secteurParam);
    if (!basePath) {
      return NextResponse.json({ error: "Profil OneDrive manquant." }, { status: 400 });
    }
    const all = await listElevesForSecteurs([secteurParam]);
    const offset = Math.max(0, Number(body.offset) || 0);
    const limit = Math.min(8, Math.max(1, Number(body.limit) || 6));
    const slice = all.slice(offset, offset + limit);
    let ok = 0;
    let fail = 0;
    for (const e of slice) {
      const folderPath = oneDrivePathForEleve(basePath, resolveEleveFolderName(e));
      const result = await refreshPilotageEleveDossier({
        accessToken,
        folderPath,
        folderName: resolveEleveFolderName(e),
        secteur: secteurParam,
      });
      if (result.ok) ok += 1;
      else fail += 1;
    }
    const nextOffset = offset + slice.length;
    return NextResponse.json({
      ok: true,
      indexed: ok,
      failed: fail,
      offset,
      nextOffset,
      total: all.length,
      done: nextOffset >= all.length,
    });
  }

  if (key) {
    const row = await findEleveRow(key, allowed);
    if (!row) return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
    const basePath = basePathFor(row.secteur);
    const folderPath =
      String(body.folderPath ?? "").trim() ||
      (basePath ? oneDrivePathForEleve(basePath, resolveEleveFolderName(row)) : "");
    if (!folderPath) {
      return NextResponse.json(
        { error: "Chemin OneDrive inconnu — vérifiez le flux OCR de cet établissement." },
        { status: 400 },
      );
    }
    const result = await refreshPilotageEleveDossier({
      accessToken,
      folderPath,
      folderName: resolveEleveFolderName(row),
      secteur: row.secteur,
    });
    return NextResponse.json(result);
  }

  if (classe && secteurParam) {
    if (!allowed.includes(secteurParam)) {
      return NextResponse.json({ error: "Secteur non autorisé." }, { status: 403 });
    }
    if (ocrSecteurs.length && !ocrSecteurs.includes(secteurParam)) {
      return NextResponse.json({ error: "Ce compte OCR n’alimente pas cet établissement." }, { status: 403 });
    }
    const roster = await listClassRoster(secteurParam, classe, allowed);
    const basePath = basePathFor(secteurParam);
    if (!basePath) {
      return NextResponse.json({ error: "Profil OneDrive manquant." }, { status: 400 });
    }
    let ok = 0;
    let fail = 0;
    for (const e of roster.slice(0, 12)) {
      const folderPath = oneDrivePathForEleve(basePath, e.folderName);
      const result = await refreshPilotageEleveDossier({
        accessToken,
        folderPath,
        folderName: e.folderName,
        secteur: secteurParam,
      });
      if (result.ok) ok += 1;
      else fail += 1;
    }
    return NextResponse.json({ ok: true, indexed: ok, failed: fail });
  }

  return NextResponse.json({ error: "key ou classe+secteur requis." }, { status: 400 });
}
