import { NextResponse } from "next/server";
import { requireMobileStaffAccess } from "@/app/lib/mobile-auth";
import {
  closeAppel,
  getAppelWithLignes,
  getOrCreateAppel,
  listElevesForClasse,
  listElevesForGroupeAppel,
  saveAppelLignes,
  type VsAppelLigneInput,
} from "@/app/lib/vs-absences-db";
import {
  jourSemaineFromIsoDate,
  listEdtCreneauxForJour,
} from "@/app/lib/vs-calendrier-db";
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
      let classe = String(body.classe || "").trim();
      if (!dateAppel) {
        return NextResponse.json({ error: "dateAppel requis." }, { status: 400 });
      }

      let heureDebut = body.heureDebut || null;
      let heureFin = body.heureFin || null;
      let matiereLibelle = body.matiereLibelle || null;
      const creneauId = body.creneauId || null;
      let groupeId: string | null = null;

      if (creneauId) {
        const jour = jourSemaineFromIsoDate(dateAppel);
        const creneaux = await listEdtCreneauxForJour(etabId, jour);
        const creneau = creneaux.find((c) => c.id === creneauId);
        if (creneau) {
          heureDebut = heureDebut || creneau.heureDebut;
          heureFin = heureFin || creneau.heureFin;
          matiereLibelle =
            matiereLibelle || creneau.matiereLibelle || creneau.enseignantNom || null;
          groupeId = creneau.groupeId ?? null;
          if (!classe) {
            classe = creneau.groupeCode || creneau.classe || "";
          }
        }
      }

      if (!classe && !groupeId) {
        return NextResponse.json(
          { error: "dateAppel et classe (ou créneau groupe) requis." },
          { status: 400 },
        );
      }
      if (!classe && groupeId) classe = "groupe";

      const appel = await getOrCreateAppel(etabId, {
        dateAppel,
        classe,
        creneauId,
        heureDebut,
        heureFin,
        matiereLibelle,
        enseignantUserId: gate.ctx.authUserId,
        enseignantNom: gate.ctx.name,
      });
      const eleves = groupeId
        ? await listElevesForGroupeAppel(etabId, groupeId)
        : await listElevesForClasse(etabId, classe);
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
      const result = await saveAppelLignes(etabId, appelId, body.lignes);
      const data = await getAppelWithLignes(etabId, appelId);
      return NextResponse.json({ channel: "mobile", ...result, ...data });
    }

    if (action === "close") {
      const appelId = String(body.appelId || "").trim();
      if (!appelId) {
        return NextResponse.json({ error: "appelId requis." }, { status: 400 });
      }
      const appel = await closeAppel(etabId, appelId);
      if (!appel) return NextResponse.json({ error: "Appel introuvable." }, { status: 404 });
      return NextResponse.json({ channel: "mobile", appel });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur appel." },
      { status: 400 },
    );
  }
}
