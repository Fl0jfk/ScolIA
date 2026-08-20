import { NextResponse } from "next/server";
import {
  getEasytransacTransactionStatus,
  isEasytransacPaymentSuccess,
} from "@/app/lib/billing/easytransac";
import { emailPaymentCompleted } from "@/app/lib/platform-signup-email";
import {
  emailSignupPaymentFailed,
  emailSignupPaymentInvoice,
} from "@/app/lib/tenant-billing-email";
import {
  loadSignupRequest,
  loadSignupRequestByToken,
  saveSignupRequest,
} from "@/app/lib/platform-signup-request";

/** Retour navigateur après paiement Easytransac (GET). */
export async function GET(req: Request) {
  const { searchParams } = new URL(req.url);
  const signupId = searchParams.get("signupId")?.trim();
  const token = searchParams.get("token")?.trim();
  const tid = searchParams.get("tid")?.trim();

  if (!signupId || !token) {
    return NextResponse.redirect(new URL("/souscrire?error=return", req.url));
  }

  const request =
    (await loadSignupRequest(signupId)) || (await loadSignupRequestByToken(token));
  if (!request || request.accessToken !== token) {
    return NextResponse.redirect(new URL("/souscrire?error=invalid", req.url));
  }

  let paid = false;
  try {
    const status = await getEasytransacTransactionStatus({
      tid: tid || request.easytransac?.paymentPageRequestId,
      orderId: request.billingMode
        ? `scola-${request.id}-${request.billingMode}`
        : undefined,
    });
    paid = isEasytransacPaymentSuccess(status.status);
  } catch (e) {
    console.error("[easytransac/return] status check", e);
  }

  if (paid && request.status !== "payment_completed" && request.status !== "active") {
    const updated = await saveSignupRequest(
      {
        ...request,
        status: "payment_completed",
        easytransac: {
          ...request.easytransac,
          lastPaymentStatus: "captured",
          lastPaymentAt: new Date().toISOString(),
        },
      },
      { action: "payment_completed", detail: tid || undefined },
    );
    void emailPaymentCompleted(updated);
    void emailSignupPaymentInvoice(updated, { tid: tid || undefined }).catch((e) =>
      console.error("[easytransac/return] signup invoice", e),
    );
  } else if (!paid && request.status === "pending_payment") {
    void emailSignupPaymentFailed(request);
  }

  return NextResponse.redirect(
    new URL(`/souscrire/statut?token=${encodeURIComponent(token)}&paid=${paid ? "1" : "0"}`, req.url),
  );
}
