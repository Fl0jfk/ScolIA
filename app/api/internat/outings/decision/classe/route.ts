/**
 * Réponse publique (parent ou direction) par lien personnel — sous le préfixe public déjà déclaré
 * `/api/internat/outings/decision`. Le GET ne modifie jamais rien (les antivirus de messagerie ouvrent les liens).
 */
import { NextResponse } from "next/server";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import {
  appliquerDecisionDirection,
  appliquerReponseParent,
  sortiePartie,
  statutEleve,
  type SortieClasse,
} from "@/app/lib/sorties-classe";
import { modifierSortie, trouverParJeton } from "@/app/lib/sorties-classe-store";
import { apresDecisionDirection, apresReponseParent } from "@/app/lib/sorties-classe-mail";

const limiter = createMemoryRateLimiter({ windowMs: 10 * 60 * 1000, max: 30, name: "sortie-classe" });
const HEADERS = { "Cache-Control": "no-store", "Referrer-Policy": "no-referrer" };

function json(body: unknown, status = 200) {
  return NextResponse.json(body, { status, headers: HEADERS });
}

type Trouve = NonNullable<Awaited<ReturnType<typeof trouverParJeton>>>;

function vuePublique(found: Trouve) {
  const s: SortieClasse = found.sortie;
  const base = {
    lieu: s.lieu,
    motif: s.motif,
    date: s.date,
    heureDepart: s.heureDepart,
    heureRetour: s.heureRetour,
    accompagnateurAller: s.accompagnateurAller,
    accompagnateurRetour: s.accompagnateurRetour,
    organisateur: s.createdBy.name,
    annulee: Boolean(s.cancelledAt),
    partie: sortiePartie(s),
  };
  if (found.role === "parent") {
    return {
      ...base,
      role: "parent" as const,
      eleve: `${found.eleve.prenom} ${found.eleve.nom}`.trim(),
      classe: found.eleve.classe,
      reponse: found.eleve.parentStatus,
      reponduLe: found.eleve.respondedAt,
      statut: statutEleve(s, found.eleve),
    };
  }
  return {
    ...base,
    role: "direction" as const,
    etablissement: found.direction.etablissement,
    reponse: found.direction.status,
    reponduLe: found.direction.decidedAt,
    eleves: s.eleves
      .filter((e) => e.etablissement === found.direction.etablissement)
      .map((e) => ({ nom: `${e.prenom} ${e.nom}`.trim(), classe: e.classe, statut: statutEleve(s, e) })),
  };
}

export async function GET(req: Request) {
  const token = String(new URL(req.url).searchParams.get("token") || "").trim();
  const found = await trouverParJeton(token);
  if (!found) return json({ error: "Lien invalide." }, 404);
  return json({ vue: vuePublique(found) });
}

export async function POST(req: Request) {
  if (!(await limiter.allow(clientIpFromRequest(req)))) {
    return json({ error: "Trop de tentatives. Réessayez dans quelques minutes." }, 429);
  }
  const body = (await req.json().catch(() => ({}))) as Record<string, unknown>;
  const token = String(body.token || "").trim();
  const decision = body.decision === "approve" ? "approve" : body.decision === "refuse" ? "refuse" : null;
  const nom = String(body.nom || "").trim().slice(0, 120);
  const note = String(body.note || "").trim();
  const found = await trouverParJeton(token);
  if (!found || !decision) return json({ error: "Lien invalide." }, 404);
  if (!nom) return json({ error: "Indiquez vos nom et prénom." }, 400);

  const studentId = found.role === "parent" ? found.eleve.studentId : "";
  const etablissement = found.role === "direction" ? found.direction.etablissement : "";
  const out = await modifierSortie(found.sortie.id, (s) =>
    found.role === "parent"
      ? appliquerReponseParent(s, studentId, {
          decision: decision === "approve" ? "authorized" : "refused",
          source: "lien",
          par: nom,
          note,
          ip: clientIpFromRequest(req),
        })
      : appliquerDecisionDirection(s, etablissement, {
          status: decision === "approve" ? "approved" : "refused",
          par: `${nom} (lien e-mail)`,
          note,
        }),
  );
  if (!out.ok) return json({ error: out.error }, out.status);

  try {
    if (found.role === "parent") await apresReponseParent(out.sortie, studentId);
    else await apresDecisionDirection(out.sortie, etablissement);
  } catch (e) {
    console.error("[sorties-classe] suites", e instanceof Error ? e.message : "erreur");
  }
  const refreshed = await trouverParJeton(token);
  return json({ vue: vuePublique(refreshed ?? found) });
}
