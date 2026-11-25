import { NextResponse } from "next/server";
import { z } from "zod";
import { canRead, canWrite, resolveAccess } from "@/app/lib/travels-v2-access";
import { requireAuth } from "@/app/lib/intranet-auth";
import { maskEmail } from "@/app/lib/mask-email";
import { getTenantAppUrl } from "@/app/lib/tenant-context";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { isWorkflowV2 } from "@/app/lib/travels-v2";
import { addRecipientForTrip } from "@/app/lib/travels-v2-registration-store";
import { SOURCE_LABELS, validateExtraRecipient } from "@/app/lib/travels-v2-recipients";
import { keyOfIne, listExtraRecipients, resolveTripRecipients } from "@/app/lib/travels-v2-recipients-store";

/**
 * Destinataires prévus des invitations (circuit v2) — API INTERNE (compte requis).
 * GET : élève par élève, adresses masquées + origine (fiche / foyer / ajoutée) ; mêmes rôles que la vue Inscriptions.
 * POST : ajoute une adresse pour CE séjour (organisateur, direction, administratif, admin) ; ne touche ni la fiche élève ni le foyer.
 */

const MAX_EXTRAS_PER_STUDENT = 6;

async function loadV2Trip(tripId: string) {
  const trip = await getTravelTrip(tripId);
  if (!trip || !isWorkflowV2(trip)) return null;
  return trip;
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
    const { byIne, students, withoutAddress } = await resolveTripRecipients(trip);
    return NextResponse.json({
      canWrite: canWrite(access),
      students: students
        .map((s) => ({
          ine: s.ine,
          name: s.name,
          classe: s.classe ?? null,
          recipients: (byIne.get(keyOfIne(s.ine)) || []).map((r) => ({
            email: maskEmail(r.email),
            source: r.source,
            sourceLabel: SOURCE_LABELS[r.source],
            name: r.name ?? null,
          })),
        }))
        .sort((a, b) => a.name.localeCompare(b.name, "fr", { sensitivity: "base" })),
      withoutAddress,
    });
  } catch (e) {
    console.error("[travels/v2/recipients] GET", e);
    return NextResponse.json({ error: "Chargement impossible." }, { status: 500 });
  }
}

const addSchema = z.object({
  tripId: z.string().min(1),
  ine: z.string().trim().min(1).max(40),
  email: z.string().max(254),
  name: z.string().max(200).optional(),
});

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const parsed = addSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return NextResponse.json({ error: "Requête invalide." }, { status: 400 });
  const input = parsed.data;
  const valid = validateExtraRecipient(input.email, input.name);
  if (!valid.ok) return NextResponse.json({ error: valid.error }, { status: 400 });
  try {
    const trip = await loadV2Trip(input.tripId);
    if (!trip) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });
    const access = await resolveAccess(trip);
    if (!access || !canWrite(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    if (trip.status === "ANNULE" || trip.status === "REJETE" || trip.status === "SEANCE_ANNULEE") {
      return NextResponse.json({ error: "Ce voyage n'est plus actif." }, { status: 409 });
    }
    const opened = Boolean(trip.data?.familyPage?.openedAt);
    if (opened && trip.status !== "INSCRIPTIONS_OUVERTES") {
      return NextResponse.json({ error: "Les inscriptions ne sont plus ouvertes pour ce voyage." }, { status: 409 });
    }
    const inList = (trip.data?.preListeEleves || []).some((p) => keyOfIne(p.ine || "") === keyOfIne(input.ine));
    if (!inList) return NextResponse.json({ error: "Cet élève n'est pas dans la pré-liste du voyage." }, { status: 400 });
    const mine = (await listExtraRecipients(trip.id)).filter((x) => keyOfIne(x.ine) === keyOfIne(input.ine));
    if (mine.length >= MAX_EXTRAS_PER_STUDENT) {
      return NextResponse.json({ error: `Au plus ${MAX_EXTRAS_PER_STUDENT} adresses ajoutées par élève.` }, { status: 409 });
    }
    const res = await addRecipientForTrip({
      trip,
      ine: input.ine,
      email: valid.email,
      name: valid.name,
      addedBy: access.userName,
      baseUrl: await getTenantAppUrl(),
    });
    if (res.busy) return NextResponse.json({ error: "Une opération est en cours sur ce voyage, réessayez dans un instant." }, { status: 409 });
    if (res.duplicate) return NextResponse.json({ error: "Cette adresse est déjà prévue pour cet élève." }, { status: 409 });
    return NextResponse.json({ ok: true, invitationCreated: res.created, sent: res.sent, opened });
  } catch (e) {
    console.error("[travels/v2/recipients] POST", e);
    return NextResponse.json({ error: "Ajout impossible." }, { status: 500 });
  }
}
