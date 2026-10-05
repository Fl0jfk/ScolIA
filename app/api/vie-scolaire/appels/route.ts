import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  getAppelWithLignes,
  listAppelsForDate,
  listAppelsManquants,
  listElevesForClasse,
  listElevesForGroupeAppel,
  listAccueilCoveringForEleves,
  type VsAppelLigneInput,
} from "@/app/lib/vs-absences-db";
import {
  closeAppelAction,
  listCreneauxAppelForActor,
  openAppelForCreneau,
  saveAppelLignesAction,
} from "@/app/lib/vs-appels-actions";
import { assertProfesseurCanAccessAppel } from "@/app/lib/vs-appels-access";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { jourSemaineFromIsoDate } from "@/app/lib/vs-calendrier-db";
import { resolvePhotoUrlsForEleves } from "@/app/lib/eleve-photos";
import { requireAppUser } from "@/app/lib/app-session";
import { getPresenceJour } from "@/app/lib/occupancy";
import { badgesByEleveId, type AppelOccupancyBadge } from "@/app/lib/vs-appel-occupancy";

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

async function occupancyBadgesForEleves(
  etablissementId: string,
  date: string,
  eleveIds: string[],
): Promise<Map<string, AppelOccupancyBadge>> {
  if (eleveIds.length === 0) return new Map();
  try {
    const result = await getPresenceJour({
      etablissementId,
      date,
      eleveIds,
    });
    return badgesByEleveId(result.facts);
  } catch (err) {
    console.error("[vs-appels] occupancy", err);
    return new Map();
  }
}

function appelActorFromGate(
  gate: { ctx: { userId: string; user: { id: string; firstName?: string | null; lastName?: string | null; name?: string | null; roles: string[] } } },
  appUser: { ok: boolean; user?: { id: string; firstName?: string | null; lastName?: string | null; name?: string | null } },
) {
  const displayName = appUser.ok
    ? [appUser.user!.firstName, appUser.user!.lastName].filter(Boolean).join(" ") ||
      appUser.user!.name ||
      "Enseignant"
    : "Enseignant";
  return {
    userId: appUser.ok ? appUser.user!.id : gate.ctx.userId,
    displayName,
    roles: gate.ctx.user.roles,
    isOrgAdmin: isOrgAdminFromAppUser(gate.ctx.user),
  };
}

export async function GET(req: Request) {
  const gate = await requireModule("vs-appels");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const actor = appelActorFromGate(gate, appUser);

  const url = new URL(req.url);
  const appelId = url.searchParams.get("appelId")?.trim();
  const classe = url.searchParams.get("classe")?.trim();
  const date = url.searchParams.get("date")?.trim();

  if (appelId) {
    try {
      await assertProfesseurCanAccessAppel(etabId, appelId, actor);
    } catch (e) {
      return NextResponse.json(
        { error: e instanceof Error ? e.message : "Accès refusé." },
        { status: 403 },
      );
    }
    const data = await getAppelWithLignes(etabId, appelId);
    if (!data) return NextResponse.json({ error: "Appel introuvable." }, { status: 404 });
    const lignes = await withPhotoUrls(
      data.lignes.map((l) => ({
        ...l,
        id: l.eleveId,
      })),
    );
    const dateKey = String(data.appel.dateAppel).slice(0, 10);
    const [prevenu, occupancyMap] = await Promise.all([
      listAccueilCoveringForEleves(
        etabId,
        lignes.map((l) => l.eleveId),
        { date: data.appel.dateAppel, heureDebut: data.appel.heureDebut, heureFin: data.appel.heureFin },
      ),
      occupancyBadgesForEleves(
        etabId,
        dateKey,
        lignes.map((l) => l.eleveId),
      ),
    ]);
    return NextResponse.json({
      ...data,
      lignes: lignes.map((l) => ({
        ...l,
        prevenuAccueil: prevenu.has(l.eleveId),
        occupancy: occupancyMap.get(l.eleveId) ?? null,
      })),
    });
  }

  if (date) {
    const jour = jourSemaineFromIsoDate(date);
    const [creneaux, appels, manquants] = await Promise.all([
      listCreneauxAppelForActor(etabId, date, actor, { classe }),
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
  const actor = appelActorFromGate(gate, appUser);
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
      const { appel, groupeId, classe } = opened;
      const heureDebut = appel.heureDebut;
      const heureFin = appel.heureFin;
      const eleves = groupeId
        ? await listElevesForGroupeAppel(etabId, groupeId)
        : await listElevesForClasse(etabId, classe);
      const elevesWithPhotos = await withPhotoUrls(eleves);
      const existing = await getAppelWithLignes(etabId, appel.id);
      const [prevenu, occupancyMap] = await Promise.all([
        listAccueilCoveringForEleves(
          etabId,
          elevesWithPhotos.map((e) => e.id),
          { date: dateAppel, heureDebut, heureFin },
        ),
        occupancyBadgesForEleves(
          etabId,
          dateAppel,
          elevesWithPhotos.map((e) => e.id),
        ),
      ]);
      const lignes = (existing?.lignes ?? []).map((l) => ({
        ...l,
        prevenuAccueil: prevenu.has(l.eleveId),
        occupancy: occupancyMap.get(l.eleveId) ?? null,
      }));
      return NextResponse.json({
        appel,
        eleves: elevesWithPhotos.map((e) => ({
          ...e,
          prevenuAccueil: prevenu.has(e.id),
          occupancy: occupancyMap.get(e.id) ?? null,
        })),
        lignes,
      });
    }

    if (action === "save") {
      const appelId = String(body.appelId || "").trim();
      if (!appelId || !Array.isArray(body.lignes)) {
        return NextResponse.json({ error: "appelId et lignes requis." }, { status: 400 });
      }
      const data = await saveAppelLignesAction(etabId, appelId, body.lignes, actor);
      return NextResponse.json(data);
    }

    if (action === "close") {
      const appelId = String(body.appelId || "").trim();
      if (!appelId) return NextResponse.json({ error: "appelId requis." }, { status: 400 });
      const closed = await closeAppelAction(etabId, appelId, actor);
      return NextResponse.json(closed);
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur appel." },
      { status: 400 },
    );
  }
}
