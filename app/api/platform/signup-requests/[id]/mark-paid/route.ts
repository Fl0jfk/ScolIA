import { NextResponse } from "next/server";
import { requirePlatformMaster } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { emailPaymentCompleted } from "@/app/lib/platform-signup-email";
import { loadSignupRequest, saveSignupRequest } from "@/app/lib/platform-signup-request";
import { emailSignupPaymentInvoice } from "@/app/lib/tenant-billing-email";

type Ctx = { params: Promise<{ id: string }> };

/** Mode dégradé : marquer comme payé manuellement (tests / premier client). */
export async function POST(req: Request, ctx: Ctx) {
  const gate = await requirePlatformMaster();
  if (!gate.ok) return gate.response;

  const { id } = await ctx.params;
  const request = await loadSignupRequest(id);
  if (!request) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

  if (
    request.status !== "microsoft_approved" &&
    request.status !== "pending_payment" &&
    request.status !== "payment_completed"
  ) {
    return NextResponse.json(
      { error: "Le dossier doit être validé Microsoft avant paiement." },
      { status: 400 },
    );
  }

  let body: { billingMode?: "monthly" | "annual_upfront" };
  try {
    body = await req.json();
  } catch {
    body = {};
  }

  const user = await safeCurrentUser();
  const updated = await saveSignupRequest(
    {
      ...request,
      status: "payment_completed",
      billingMode: body.billingMode || request.billingMode || "monthly",
      easytransac: {
        ...request.easytransac,
        lastPaymentStatus: "manual",
        lastPaymentAt: new Date().toISOString(),
      },
    },
    { action: "payment_manual", by: user?.id || undefined },
  );

  void emailPaymentCompleted(updated);
  void emailSignupPaymentInvoice(updated).catch((e) =>
    console.error("[mark-paid] signup invoice", e),
  );
  return NextResponse.json({ ok: true, request: updated });
}
