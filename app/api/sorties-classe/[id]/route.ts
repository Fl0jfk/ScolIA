import { NextResponse } from "next/server";
import { requireActeurSorties } from "@/app/api/sorties-classe/_acteur";
import {
  appliquerDecisionDirection,
  appliquerReponseParent,
  peutDeciderDirection,
  peutGerer,
  peutVoir,
  sortiePartie,
  vueSortie,
  type SortieClasse,
} from "@/app/lib/sorties-classe";
import { lireSortie, modifierSortie } from "@/app/lib/sorties-classe-store";
import {
  apresAnnulation,
  apresDecisionDirection,
  apresReponseParent,
  relancerParents,
} from "@/app/lib/sorties-classe-mail";

const UNE_HEURE_MS = 60 * 60_000;

function erreur(error: string, status: number) {
  return NextResponse.json({ error }, { status });
}

async function suite(task: () => Promise<unknown>) {
  try {
    await task();
  } catch (e) {
    console.error("[sorties-classe] suites", e instanceof Error ? e.message : "erreur");
  }
}

export async function PATCH(req: Request, ctx: { params: Promise<{ id: string }> }) {
  const access = await requireActeurSorties();
  if (!access.ok) return access.response;
  const { acteur, nom } = access;
  const { id } = await ctx.params;
  const body = (await req.json().catch(() => ({}))) as Record<string, unknown>;
  const action = String(body.action || "");

  const sortie = await lireSortie(id);
  if (!sortie || !peutVoir(acteur, sortie)) return erreur("Sortie introuvable.", 404);

  const repondre = async (current: SortieClasse, extra: Record<string, unknown> = {}) =>
    NextResponse.json({ sortie: vueSortie((await lireSortie(id)) ?? current, acteur), ...extra });

  if (action === "direction") {
    const etablissement = String(body.etablissement || "");
    const decision = body.decision === "approve" ? "approved" : body.decision === "refuse" ? "refused" : null;
    const direction = sortie.directions.find((d) => d.etablissement === etablissement);
    if (!decision || !direction) return erreur("Décision incomplète.", 400);
    if (!peutDeciderDirection(acteur, direction)) return erreur("Réservé à la direction concernée.", 403);
    const out = await modifierSortie(id, (s) =>
      appliquerDecisionDirection(s, etablissement, {
        status: decision,
        par: nom,
        note: String(body.note || ""),
      }),
    );
    if (!out.ok) return erreur(out.error, out.status);
    await suite(() => apresDecisionDirection(out.sortie, etablissement));
    return repondre(out.sortie);
  }

  // Tout le reste est réservé au créateur de la sortie et à l'administrateur général.
  if (!peutGerer(acteur, sortie)) return erreur("Réservé à l'organisateur et à l'administrateur.", 403);

  if (action === "manuel") {
    const studentId = String(body.studentId || "");
    const decision = body.decision === "approve" ? "authorized" : body.decision === "refuse" ? "refused" : null;
    if (!decision) return erreur("Décision incomplète.", 400);
    const out = await modifierSortie(id, (s) =>
      appliquerReponseParent(s, studentId, {
        decision,
        source: "manuel",
        par: nom,
        note: String(body.note || ""),
      }),
    );
    if (!out.ok) return erreur(out.error, out.status);
    await suite(() => apresReponseParent(out.sortie, studentId));
    return repondre(out.sortie);
  }

  if (action === "relancer") {
    if (sortie.cancelledAt || sortiePartie(sortie)) {
      return erreur("Cette sortie n'accepte plus de réponses.", 409);
    }
    const recent = (at?: string) => Boolean(at) && Date.now() - new Date(at!).getTime() < UNE_HEURE_MS;
    const eleves = sortie.eleves.filter(
      (e) => e.parentStatus === "pending" && e.emails.length && !recent(e.remindedAt),
    );
    if (!eleves.length) {
      return erreur(
        "Aucun parent à relancer (réponses reçues, sans e-mail, ou relancés il y a moins d'une heure).",
        409,
      );
    }
    const envoyes = await relancerParents(sortie, eleves, "manuelle");
    return repondre(sortie, { relances: envoyes });
  }

  if (action === "annuler") {
    const out = await modifierSortie(id, (s) => {
      if (s.cancelledAt) return { ok: false, error: "Sortie déjà annulée." };
      if (sortiePartie(s)) return { ok: false, error: "L'heure de départ est passée : annulation impossible." };
      s.cancelledAt = new Date().toISOString();
      s.cancelledBy = nom;
      s.updatedAt = s.cancelledAt;
      return { ok: true };
    });
    if (!out.ok) return erreur(out.error, out.status);
    let avertissements = ["Des e-mails d'annulation ne sont peut-être pas partis : prévenez l'internat."];
    await suite(async () => {
      avertissements = await apresAnnulation(out.sortie);
    });
    return repondre(out.sortie, { avertissements });
  }

  return erreur("Action inconnue.", 400);
}
