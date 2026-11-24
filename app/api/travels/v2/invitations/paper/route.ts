import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getTenantAppUrl } from "@/app/lib/tenant-context";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { isWorkflowV2 } from "@/app/lib/travels-v2";
import { canWrite, resolveAccess } from "@/app/lib/travels-v2-access";
import { recordPaperResponse } from "@/app/lib/travels-v2-lot3-store";
import { getInvitation } from "@/app/lib/travels-v2-registration-store";
import { DOC_MAX_BYTES, MEAL_TYPE_VALUES, PHONE_RE } from "@/app/lib/travels-v2-registration-types";

const metaSchema = z
  .object({
    tripId: z.string().min(1),
    invitationId: z.string().min(1),
    decision: z.enum(["oui", "non"]),
    signerName: z.string().trim().min(2).max(100),
    phone: z.string().trim().regex(PHONE_RE).optional(),
    meals: z
      .record(z.string().min(1).max(40), z.object({ wants: z.enum(["oui", "non"]), type: z.enum(MEAL_TYPE_VALUES).optional() }))
      .optional(),
  })
  .strict();

/**
 * Coupon papier : l'organisateur (ou la direction / l'administratif) enregistre la réponse d'une famille
 * (multipart : `meta` JSON + `scan`). Compte comme une signature, avec la mention « papier » ; le scan fait office d'engagement.
 */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const declared = Number(req.headers.get("content-length") || 0);
    if (declared > DOC_MAX_BYTES + 512 * 1024) return NextResponse.json({ error: "Scan trop volumineux (8 Mo maximum)." }, { status: 413 });
    const form = await req.formData().catch(() => null);
    const scan = form?.get("scan");
    let metaRaw: unknown = null;
    try {
      metaRaw = JSON.parse(String(form?.get("meta") || "null"));
    } catch {
      metaRaw = null;
    }
    const meta = metaSchema.safeParse(metaRaw);
    if (!meta.success || !(scan instanceof File)) {
      return NextResponse.json({ error: "Requête invalide : le scan du coupon est obligatoire." }, { status: 400 });
    }
    if (scan.size > DOC_MAX_BYTES) return NextResponse.json({ error: "Scan trop volumineux (8 Mo maximum)." }, { status: 413 });
    const trip = await getTravelTrip(meta.data.tripId);
    if (!trip || !isWorkflowV2(trip) || !trip.data?.familyPage?.openedAt) {
      return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });
    }
    const access = await resolveAccess(trip);
    if (!access || !canWrite(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    if (trip.status !== "INSCRIPTIONS_OUVERTES") {
      return NextResponse.json({ error: "Les inscriptions ne sont plus ouvertes pour ce voyage." }, { status: 409 });
    }
    const inv = await getInvitation(trip.id, meta.data.invitationId);
    if (!inv) return NextResponse.json({ error: "Introuvable." }, { status: 404 });
    const res = await recordPaperResponse({
      trip,
      inv,
      baseUrl: await getTenantAppUrl(),
      input: {
        decision: meta.data.decision,
        signerName: meta.data.signerName,
        phone: meta.data.phone,
        meals: meta.data.meals,
        scan: { bytes: Buffer.from(await scan.arrayBuffer()), fileName: scan.name || "coupon" },
        by: access.userName,
      },
    });
    return res.ok ? NextResponse.json({ ok: true }) : NextResponse.json({ error: res.error }, { status: res.status });
  } catch (e) {
    console.error("[travels/v2/invitations/paper]", e);
    return NextResponse.json({ error: "Enregistrement impossible." }, { status: 500 });
  }
}
