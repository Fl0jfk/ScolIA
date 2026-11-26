import { NextResponse } from "next/server";
import { requireActeurSorties } from "@/app/api/sorties-classe/_acteur";
import { peutCreer, peutVoir, validerSaisie, vueSortie } from "@/app/lib/sorties-classe";
import { creerSortie, lireSortie, listerSorties } from "@/app/lib/sorties-classe-store";
import { envoyerInvitations } from "@/app/lib/sorties-classe-mail";

export async function GET() {
  const access = await requireActeurSorties();
  if (!access.ok) return access.response;
  const sorties = (await listerSorties())
    .filter((s) => peutVoir(access.acteur, s))
    .map((s) => vueSortie(s, access.acteur));
  return NextResponse.json({ sorties, peutCreer: peutCreer(access.acteur) });
}

export async function POST(req: Request) {
  const access = await requireActeurSorties();
  if (!access.ok) return access.response;
  if (!peutCreer(access.acteur)) {
    return NextResponse.json(
      { error: "Seuls les professeurs et l'administrateur général peuvent créer une sortie." },
      { status: 403 },
    );
  }
  const body = (await req.json().catch(() => ({}))) as Record<string, unknown>;
  const checked = validerSaisie(body);
  if (!checked.ok) return NextResponse.json({ error: checked.error }, { status: 400 });

  const created = await creerSortie(checked.saisie, {
    userId: access.acteur.userId,
    name: access.nom,
    email: access.acteur.email,
  });
  if (!created.ok) return NextResponse.json({ error: created.error }, { status: 400 });

  // La sortie est enregistrée : un incident d'envoi ne doit pas faire croire le contraire (doublon).
  let avertissements: string[];
  try {
    avertissements = await envoyerInvitations(created.sortie);
  } catch (e) {
    console.error("[sorties-classe] invitations", e instanceof Error ? e.message : "erreur");
    avertissements = ["Sortie enregistrée, mais l'envoi des e-mails a échoué. Utilisez « Relancer les parents »."];
  }
  const sortie = (await lireSortie(created.sortie.id)) ?? created.sortie;
  return NextResponse.json({ sortie: vueSortie(sortie, access.acteur), avertissements });
}
