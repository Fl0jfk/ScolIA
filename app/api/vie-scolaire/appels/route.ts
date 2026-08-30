import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  closeAppel,
  getAppelWithLignes,
  getOrCreateAppel,
  listAppelsForDate,
  listAppelsManquants,
  listElevesForClasse,
  listElevesForGroupeAppel,
  saveAppelLignes,
  listAccueilCoveringForEleves,
  type VsAppelLigneInput,
} from "@/app/lib/vs-absences-db";
import {
  jourSemaineFromIsoDate,
  listEdtCreneauxForJour,
} from "@/app/lib/vs-calendrier-db";
import { resolvePhotoUrlsForEleves } from "@/app/lib/eleve-photos";
import { requireAppUser } from "@/app/lib/app-session";

async function withPhotoUrls<
  T extends { id?: string; eleveId?: string; nom: string; prenom: string; ine?: string | null; photoKey?: string | null },
>(rows: T[]): Promise<Array<T & { photoUrl: string | null }>> {
  const forResolve = rows.map((r) => ({
    id: r.id || r.eleveId || "",
    nom: r.nom,
    prenom: r.prenom,
    ine: r.ine,
    photoKey: r.photoKey,
  }));
  const urls = await resolvePhotoUrlsForEleves(forResolve.filter((r) => r.id));
  return rows.map((r) => {
    const id = r.id || r.eleveId || "";
    return { ...r, photoUrl: urls[id] ?? null };
  });
}

export async function GET(req: Request) {
  const gate = await requireModule("vs-appels");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const appelId = url.searchParams.get("appelId")?.trim();
  const classe = url.searchParams.get("classe")?.trim();
  const date = url.searchParams.get("date")?.trim();

  if (appelId) {
    const data = await getAppelWithLignes(etabId, appelId);
    if (!data) return NextResponse.json({ error: "Appel introuvable." }, { status: 404 });
    const lignes = await withPhotoUrls(
      data.lignes.map((l) => ({
        ...l,
        id: l.eleveId,
      })),
    );
    const prevenu = await listAccueilCoveringForEleves(
      etabId,
      lignes.map((l) => l.eleveId),
      { date: data.appel.dateAppel, heureDebut: data.appel.heureDebut, heureFin: data.appel.heureFin },
    );
    return NextResponse.json({
      ...data,
      lignes: lignes.map((l) => ({
        ...l,
        prevenuAccueil: prevenu.has(l.eleveId),
      })),
    });
  }

  if (date) {
    const jour = jourSemaineFromIsoDate(date);
    const [creneaux, appels, manquants] = await Promise.all([
      listEdtCreneauxForJour(etabId, jour, { classe }),
      listAppelsForDate(etabId, date),
      listAppelsManquants(etabId, { dateAppel: date }),
    ]);
    return NextResponse.json({
      date,
      jourSemaine: jour,
      creneaux,
      appels,
      manquants,
    });
  }

  if (classe) {
    const eleves = await listElevesForClasse(etabId, classe);
    const withPhotos = await withPhotoUrls(eleves);
    return NextResponse.json({ classe, eleves: withPhotos });
  }

  return NextResponse.json({ error: "appelId, date ou classe requis." }, { status: 400 });
}

export async function POST(req: Request) {
  const gate = await requireModule("vs-appels");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
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
        return NextResponse.json({ error: "dateAppel et classe (ou créneau groupe) requis." }, { status: 400 });
      }
      if (!classe && groupeId) {
        classe = "groupe";
      }

      const enseignantNom = appUser.ok
        ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
          appUser.user.name ||
          null
        : null;
      const appel = await getOrCreateAppel(etabId, {
        dateAppel,
        classe,
        creneauId,
        heureDebut,
        heureFin,
        matiereLibelle,
        enseignantUserId: appUser.ok ? appUser.user.id : null,
        enseignantNom,
      });
      const eleves = groupeId
        ? await listElevesForGroupeAppel(etabId, groupeId)
        : await listElevesForClasse(etabId, classe);
      const elevesWithPhotos = await withPhotoUrls(eleves);
      const existing = await getAppelWithLignes(etabId, appel.id);
      const prevenu = await listAccueilCoveringForEleves(
        etabId,
        elevesWithPhotos.map((e) => e.id),
        { date: dateAppel, heureDebut, heureFin },
      );
      const lignes = (existing?.lignes ?? []).map((l) => ({
        ...l,
        prevenuAccueil: prevenu.has(l.eleveId),
      }));
      return NextResponse.json({
        appel,
        eleves: elevesWithPhotos.map((e) => ({ ...e, prevenuAccueil: prevenu.has(e.id) })),
        lignes,
      });
    }

    if (action === "save") {
      const appelId = String(body.appelId || "").trim();
      if (!appelId || !Array.isArray(body.lignes)) {
        return NextResponse.json({ error: "appelId et lignes requis." }, { status: 400 });
      }
      const result = await saveAppelLignes(etabId, appelId, body.lignes);
      const data = await getAppelWithLignes(etabId, appelId);
      return NextResponse.json({ ...result, ...data });
    }

    if (action === "close") {
      const appelId = String(body.appelId || "").trim();
      if (!appelId) return NextResponse.json({ error: "appelId requis." }, { status: 400 });
      const appel = await closeAppel(etabId, appelId);
      if (!appel) return NextResponse.json({ error: "Appel introuvable." }, { status: 404 });
      return NextResponse.json({ appel });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur appel." },
      { status: 400 },
    );
  }
}
