import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  createCahierTexte,
  hideCahierTexteFromFamille,
  listCahierTexte,
  listDistinctClasses,
  updateCahierTexte,
} from "@/app/lib/cahier-texte-db";

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const url = new URL(req.url);
  const classe = url.searchParams.get("classe");
  const from = url.searchParams.get("from");
  const to = url.searchParams.get("to");
  const [entries, classes] = await Promise.all([
    listCahierTexte(etabId, { classe, from, to }),
    listDistinctClasses(etabId),
  ]);
  return NextResponse.json({ entries, classes });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }
  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    id?: string;
    dateSeance?: string;
    classe?: string;
    matiereLibelle?: string;
    contenu?: string;
    travail?: string;
    aRendreLe?: string | null;
    visibleFamille?: boolean;
  };

  const enseignantNom =
    user?.fullName ||
    [user?.firstName, user?.lastName].filter(Boolean).join(" ") ||
    user?.primaryEmailAddress?.emailAddress ||
    "Enseignant";

  try {
    if (body.action === "hide" && body.id) {
      const entry = await hideCahierTexteFromFamille(etabId, String(body.id));
      return NextResponse.json({ success: true, entry });
    }
    if (body.action === "update" && body.id) {
      const entry = await updateCahierTexte(etabId, String(body.id), {
        dateSeance: body.dateSeance,
        classe: body.classe,
        matiereLibelle: body.matiereLibelle,
        contenu: body.contenu,
        travail: body.travail,
        aRendreLe: body.aRendreLe,
        visibleFamille: body.visibleFamille,
      });
      return NextResponse.json({ success: true, entry });
    }

    const entry = await createCahierTexte(etabId, {
      dateSeance: String(body.dateSeance || ""),
      classe: String(body.classe || ""),
      matiereLibelle: body.matiereLibelle,
      contenu: body.contenu,
      travail: body.travail,
      aRendreLe: body.aRendreLe,
      visibleFamille: body.visibleFamille !== false,
      enseignantUserId: userId,
      enseignantNom,
    });
    return NextResponse.json({ success: true, entry });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 400 },
    );
  }
}
