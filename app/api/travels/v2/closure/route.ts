import { after, NextResponse } from "next/server";
import { z } from "zod";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getTenant, runWithTenant } from "@/app/lib/tenant-context";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { isWorkflowV2 } from "@/app/lib/travels-v2";
import { canRead, canSeeAmount, canWrite, resolveAccess } from "@/app/lib/travels-v2-access";
import {
  buildClosureView,
  closeRegistrations,
  comptaZipDownloadUrl,
  completeClosureSideEffects,
  recordWithdrawal,
  saveAdultMealsAndSend,
} from "@/app/lib/travels-v2-lot4-store";

/**
 * Vue « Clôture » du hub voyage (circuit v2) — API INTERNE (compte requis).
 * Lecture : organisateur, direction, administratif, admin général, compta.
 * Actions : organisateur, direction, administratif, admin général. Le ZIP des engagements : compta, direction, admin.
 */

async function loadV2Trip(tripId: string) {
  const trip = await getTravelTrip(tripId);
  return trip && isWorkflowV2(trip) ? trip : null;
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const tripId = new URL(req.url).searchParams.get("tripId")?.trim() || "";
  if (!tripId) return NextResponse.json({ error: "Voyage manquant." }, { status: 400 });
  try {
    const trip = await loadV2Trip(tripId);
    if (!trip) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });
    const access = await resolveAccess(trip);
    if (!access || !canRead(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    if (!trip.data?.familyPage?.openedAt) return NextResponse.json({ error: "Page des familles non ouverte." }, { status: 409 });
    const view = await buildClosureView(trip);
    return NextResponse.json({
      ...view,
      canWrite: canWrite(access),
      canZip: canSeeAmount(access),
      isDirection: access.isDirection || access.isAdmin,
    });
  } catch (e) {
    console.error("[travels/v2/closure] GET", e);
    return NextResponse.json({ error: "Chargement impossible." }, { status: 500 });
  }
}

const actionSchema = z.discriminatedUnion("action", [
  // Clôture confirmée par un humain (jamais sans `confirm: true`).
  z.object({ action: z.literal("close"), tripId: z.string().min(1), confirm: z.literal(true) }),
  z.object({
    action: z.literal("adult_meals"),
    tripId: z.string().min(1),
    adults: z.record(z.string().min(1).max(20), z.number().int().min(0).max(200)),
    send: z.boolean(),
  }),
  z.object({
    action: z.literal("withdraw"),
    tripId: z.string().min(1),
    ine: z.string().min(1).max(40),
    note: z.string().trim().max(300).optional(),
    confirm: z.literal(true),
  }),
  z.object({ action: z.literal("zip_link"), tripId: z.string().min(1) }),
]);

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const parsed = actionSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ error: "Requête invalide." }, { status: 400 });
  const input = parsed.data;
  try {
    const trip = await loadV2Trip(input.tripId);
    if (!trip) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });
    const access = await resolveAccess(trip);
    if (!access || !canRead(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });

    if (input.action === "zip_link") {
      // Engagements signés (noms, signatures) : compta, direction, admin seulement.
      if (!canSeeAmount(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
      if (!trip.data?.v2Closure) return NextResponse.json({ error: "Les inscriptions ne sont pas encore clôturées." }, { status: 409 });
      const res = await comptaZipDownloadUrl(trip);
      return res.ok ? NextResponse.json({ url: res.url, count: res.count }) : NextResponse.json({ error: res.error }, { status: 404 });
    }

    if (!canWrite(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    const me = await safeCurrentUser();
    const byEmail = me?.primaryEmailAddress?.emailAddress || me?.emailAddresses?.[0]?.emailAddress;

    if (input.action === "close") {
      const res = await closeRegistrations({ tripId: input.tripId, mode: "manuel", by: access.userName, byUserId: me?.id });
      if (!res.ok) return NextResponse.json({ error: res.error }, { status: res.status });
      // Liste transporteur, ZIP et e-mail compta : après la réponse, dans le contexte de l'établissement.
      const tenant = await getTenant();
      after(async () => {
        try {
          await runWithTenant(tenant, () => completeClosureSideEffects(input.tripId));
        } catch (e) {
          console.error("[travels/v2/closure] effets de clôture", e instanceof Error ? e.message : e);
        }
      });
      return NextResponse.json({ ok: true, already: res.already === true });
    }

    if (input.action === "adult_meals") {
      const res = await saveAdultMealsAndSend({ tripId: input.tripId, adults: input.adults, send: input.send, by: access.userName, byEmail });
      return res.ok ? NextResponse.json({ ok: true, sent: res.sent }) : NextResponse.json({ error: res.error }, { status: res.status });
    }

    if (input.action === "withdraw") {
      const res = await recordWithdrawal({ tripId: input.tripId, ine: input.ine, by: access.userName, note: input.note });
      return res.ok
        ? NextResponse.json({ ok: true, cuisineNotified: res.cuisineNotified, comptaNotified: res.comptaNotified })
        : NextResponse.json({ error: res.error }, { status: res.status });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    console.error("[travels/v2/closure] POST", e);
    return NextResponse.json({ error: "Action impossible." }, { status: 500 });
  }
}
