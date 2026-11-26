import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAuth } from "@/app/lib/intranet-auth";
import { requireAppUser } from "@/app/lib/app-session";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { resolveContactsAccess } from "@/app/lib/travels-parent-contacts";
import { buildParentContactsPdf } from "@/app/lib/travels-parent-contacts-pdf";
import { auditParentContacts, loadContactsForEleves, loadTripParticipants } from "@/app/lib/travels-parent-contacts-store";

/** Fiche contacts PDF (tous les participants) : mêmes droits que l'écran, journalisée, jamais stockée ni mise en cache. */
const NO_STORE = { "Cache-Control": "private, no-store, max-age=0" };
const bodySchema = z.object({ tripId: z.string().min(1).max(100) });

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const parsed = bodySchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ error: "Requête invalide." }, { status: 400, headers: NO_STORE });
  try {
    const me = await safeCurrentUser();
    const trip = await getTravelTrip(parsed.data.tripId);
    if (!me || !trip) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404, headers: NO_STORE });
    if (!resolveContactsAccess(trip, me).allowed) {
      return NextResponse.json({ error: "Accès refusé." }, { status: 403, headers: NO_STORE });
    }
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 404, headers: NO_STORE });

    const participants = await loadTripParticipants(trip, etabId);
    await auditParentContacts({ etabId, actorUserId: await auditActorId(), tripId: trip.id, action: "travel_parent_contacts_pdf", count: participants.length });
    const eleves = await loadContactsForEleves(etabId, participants);
    const d = trip.data;
    const dateLabel = trip.type === "COMPLEX" ? [d?.startDate, d?.endDate].filter(Boolean).join(" au ") : String(d?.date || "");
    const bytes = buildParentContactsPdf({
      tripTitle: String(d?.title || d?.destination || "Voyage"),
      destination: d?.destination ? String(d.destination) : undefined,
      dateLabel: dateLabel || undefined,
      editedAt: new Date(),
      editedBy: me.fullName || "Utilisateur",
      eleves,
    });
    return new NextResponse(Buffer.from(bytes), {
      status: 200,
      headers: {
        ...NO_STORE,
        "Content-Type": "application/pdf",
        "Content-Disposition": 'attachment; filename="contacts-voyage-confidentiel.pdf"',
      },
    });
  } catch (e) {
    console.error("[travels/parent-contacts/pdf]", e instanceof Error ? e.message : "erreur");
    return NextResponse.json({ error: "Génération impossible.", code: errorCode(e) }, { status: 500, headers: NO_STORE });
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
