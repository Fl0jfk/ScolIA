import { NextResponse } from "next/server";
import { requireMobileStaffAccess } from "@/app/lib/mobile-auth";
import { getAppelWithLignes, listElevesForClasse, type VsAppelLigneInput } from "@/app/lib/vs-absences-db";
import {
  closeAppelAction,
  openAppelForCreneau,
  saveAppelLignesAction,
} from "@/app/lib/vs-appels-actions";
import { resolvePhotoUrlsForEleves } from "@/app/lib/eleve-photos";

/**
 * Appel mobile-first — même moteur que /api/vie-scolaire/appels,
 * exposé sous /api/mobile pour l’app staff-lite (pas l’intranet web).
 */
export async function GET(req: Request) {
  const gate = await requireMobileStaffAccess();
  if (!gate.ok) return gate.response;
  const etabId = gate.ctx.etablissementId;
  const url = new URL(req.url);
  const appelId = url.searchParams.get("appelId")?.trim();
  const classe = url.searchParams.get("classe")?.trim();

  if (appelId) {
    const data = await getAppelWithLignes(etabId, appelId);
    if (!data) return NextResponse.json({ error: "Appel introuvable." }, { status: 404 });
    const forResolve = data.lignes.map((l) => ({
      id: l.eleveId,
      nom: l.nom,
      prenom: l.prenom,
      ine: l.ine,
      photoKey: l.photoKey,
    }));
    const urls = await resolvePhotoUrlsForEleves(forResolve);
    return NextResponse.json({
      channel: "mobile",
      ...data,
      lignes: data.lignes.map((l) => ({ ...l, photoUrl: urls[l.eleveId] ?? null })),
    });
  }

  if (classe) {
    const eleves = await listElevesForClasse(etabId, classe);
    const urls = await resolvePhotoUrlsForEleves(eleves);
    return NextResponse.json({
      channel: "mobile",
      classe,
      eleves: eleves.map((e) => ({ ...e, photoUrl: urls[e.id] ?? null })),
    });
  }

  return NextResponse.json({ error: "appelId ou classe requis." }, { status: 400 });
}

export async function POST(req: Request) {
  const gate = await requireMobileStaffAccess();
  if (!gate.ok) return gate.response;
  const etabId = gate.ctx.etablissementId;
  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    appelId?: string;
    dateAppel?: string;
    classe?: string;
    creneauId?: string;
    heureDebut?: string;
    heureFin?: string;
    matiereLibelle?: string;
    lignes?: VsAppelLigneInput[];
  };
  const action = body.action || "create";

  try {
    if (action === "create") {
      const dateAppel = String(body.dateAppel || "").trim();
      if (!dateAppel) {
        return NextResponse.json({ error: "dateAppel requis." }, { status: 400 });
      }

      const actor = {
        userId: gate.ctx.authUserId,
        displayName: gate.ctx.name,
        roles: gate.ctx.roles,
        isOrgAdmin: false,
      };
      const opened = await openAppelForCreneau(
        etabId,
        {
          dateAppel,
          creneauId: body.creneauId || null,
          classe: body.classe,
          heureDebut: body.heureDebut,
          heureFin: body.heureFin,
          matiereLibelle: body.matiereLibelle,
        },
        actor,
      );
      const { appel, groupeId: gid, classe: cls } = opened;
      const eleves = gid
        ? await (await import("@/app/lib/vs-absences-db")).listElevesForGroupeAppel(etabId, gid)
        : await listElevesForClasse(etabId, cls);
      const urls = await resolvePhotoUrlsForEleves(eleves);
      const existing = await getAppelWithLignes(etabId, appel.id);
      return NextResponse.json({
        channel: "mobile",
        appel,
        eleves: eleves.map((e) => ({ ...e, photoUrl: urls[e.id] ?? null })),
        lignes: existing?.lignes ?? [],
      });
    }

    if (action === "save" || action === "save_lignes") {
      const appelId = String(body.appelId || "").trim();
      if (!appelId || !Array.isArray(body.lignes)) {
        return NextResponse.json({ error: "appelId et lignes requis." }, { status: 400 });
      }
      const actor = {
        userId: gate.ctx.authUserId,
        displayName: gate.ctx.name,
        roles: gate.ctx.roles,
        isOrgAdmin: false,
      };
      const data = await saveAppelLignesAction(etabId, appelId, body.lignes, actor);
      return NextResponse.json({ channel: "mobile", ...data });
    }

    if (action === "close") {
      const appelId = String(body.appelId || "").trim();
      if (!appelId) {
        return NextResponse.json({ error: "appelId requis." }, { status: 400 });
      }
      const actor = {
        userId: gate.ctx.authUserId,
        displayName: gate.ctx.name,
        roles: gate.ctx.roles,
        isOrgAdmin: false,
      };
      const closed = await closeAppelAction(etabId, appelId, actor);
      return NextResponse.json({ channel: "mobile", ...closed });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur appel." },
      { status: 400 },
    );
  }
}
