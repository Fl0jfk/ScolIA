import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAuth } from "@/app/lib/intranet-auth";
import { requireAppUser } from "@/app/lib/app-session";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { findParticipant, resolveContactsAccess } from "@/app/lib/travels-parent-contacts";
import { auditParentContacts, loadContactsForEleves, loadTripParticipants } from "@/app/lib/travels-parent-contacts-store";

/**
 * Contacts des parents d'un élève PARTICIPANT d'un voyage (fonction d'urgence).
 * Droits : organisateur, accompagnateurs inscrits sur ce voyage, admin général. Rien d'autre.
 * POST uniquement : aucune donnée dans l'URL. Jamais de cache.
 */
const NO_STORE = { "Cache-Control": "private, no-store, max-age=0" };
const json = (body: unknown, status = 200) => NextResponse.json(body, { status, headers: NO_STORE });

const bodySchema = z.discriminatedUnion("action", [
  z.object({ action: z.literal("access"), tripId: z.string().min(1).max(100) }),
  z.object({ action: z.literal("participants"), tripId: z.string().min(1).max(100) }),
  z.object({ action: z.literal("eleve"), tripId: z.string().min(1).max(100), eleveId: z.string().uuid() }),
]);

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const parsed = bodySchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return json({ error: "Requête invalide." }, 400);
  const input = parsed.data;
  try {
    const me = await safeCurrentUser();
    const trip = await getTravelTrip(input.tripId); // déjà cloisonné par établissement
    if (!me || !trip) return json({ error: "Voyage introuvable." }, 404);
    const access = resolveContactsAccess(trip, me);
    if (input.action === "access") return json({ allowed: access.allowed });
    if (!access.allowed) return json({ error: "Accès refusé." }, 403);
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return json({ error: "Établissement introuvable." }, 404);

    const participants = await loadTripParticipants(trip, etabId);
    if (input.action === "participants") {
      return json({ participants: participants.map((p) => ({ eleveId: p.eleveId, nom: p.nom, prenom: p.prenom, classe: p.classe })) });
    }
    const target = findParticipant(participants, input.eleveId);
    if (!target) return json({ error: "Cet élève ne participe pas à ce voyage." }, 404);
    // Journal AVANT la réponse : sans trace, pas de données.
    await auditParentContacts({ etabId, actorUserId: await auditActorId(), tripId: trip.id, eleveId: target.eleveId, action: "travel_parent_contacts_view" });
    const [contacts] = await loadContactsForEleves(etabId, [target]);
    return json({ contacts });
  } catch (e) {
    console.error("[travels/parent-contacts]", e instanceof Error ? e.message : "erreur");
    return json({ error: "Chargement impossible.", code: errorCode(e) }, 500);
  }
}

/** Id du compte (table `user`) pour le journal : PAS le businessUserId historique (clé étrangère). */
async function auditActorId(): Promise<string | null> {
  const g = await requireAppUser();
  return g.ok ? g.user.id : null;
}

/** Code technique sans donnée personnelle, pour diagnostiquer un 500 depuis le navigateur. */
function errorCode(e: unknown): string {
  const c = (e as { code?: unknown; cause?: { code?: unknown } } | null)?.code ?? (e as { cause?: { code?: unknown } } | null)?.cause?.code;
  return typeof c === "string" ? c.slice(0, 20) : "unknown";
}
