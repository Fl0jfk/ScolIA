import { NextResponse } from "next/server";
import {
  findElevesByParentEmail,
  toParentLinkedChildren,
} from "@/app/lib/eleves-parent-emails";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import {
  deletePendingRequestPrefix,
  generatePendingRequestToken,
  savePendingRequestWithFiles,
} from "@/app/lib/request-pending-verify";
import {
  assertEligibleRequestAttachment,
  getPublicAppBaseUrl,
  notifyRequestPendingVerification,
  validateParentPortalInput,
} from "@/app/lib/requests";
import { getRequestsRoutingConfig } from "@/app/lib/requests-routing-config";
import { getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { clientIpFromRequest, createSlidingWindowRateLimiter } from "@/app/lib/memory-rate-limit";

export const runtime = "nodejs";

const MAX_PARENT_ATTACHMENTS = 2;

const parentPortalLimiter = createSlidingWindowRateLimiter({
  windowMs: 60 * 60 * 1000,
  max: 8,
});

/** Statut page parents (public). */
export async function GET() {
  try {
    const config = await getRequestsRoutingConfig();
    return NextResponse.json({
      enabled: config.parentPortal?.enabled === true,
      path: "/demande-parents",
    });
  } catch (e) {
    console.error("[parent-portal] GET", e);
    return NextResponse.json({ error: "Indisponible." }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const config = await getRequestsRoutingConfig();
    if (config.parentPortal?.enabled !== true) {
      return NextResponse.json(
        { error: "La page de demandes parents n’est pas ouverte pour cet établissement." },
        { status: 403 },
      );
    }

    const ip = clientIpFromRequest(req);
    if (!parentPortalLimiter.allow(ip)) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez plus tard." },
        { status: 429 },
      );
    }

    if (!(await getTenantSmtpConfig())) {
      return NextResponse.json(
        {
          error:
            "La confirmation par e-mail n’est pas configurée. Contactez l’établissement.",
        },
        { status: 503 },
      );
    }

    const base = await getPublicAppBaseUrl();
    if (!base) {
      return NextResponse.json(
        { error: "Configuration incomplète (URL publique)." },
        { status: 503 },
      );
    }

    const form = await req.formData();
    // Honeypot anti-bot
    const honeypot = String(form.get("website") || form.get("company") || "").trim();
    if (honeypot) {
      return NextResponse.json({
        success: true,
        needsEmailVerification: true,
        message:
          "Un e-mail de confirmation vient de vous être envoyé si l’adresse est valide.",
      });
    }

    const validated = validateParentPortalInput({
      fullName: String(form.get("fullName") || ""),
      email: String(form.get("email") || ""),
      phone: String(form.get("phone") || ""),
      description: String(form.get("description") || ""),
    });
    if (!validated.ok) {
      return NextResponse.json({ error: validated.error }, { status: 400 });
    }

    const rawFiles = form
      .getAll("files")
      .filter((x): x is File => x instanceof File && x.size > 0)
      .slice(0, MAX_PARENT_ATTACHMENTS);
    for (const f of rawFiles) {
      const check = assertEligibleRequestAttachment(f.name, f.type, f.size);
      if (!check.ok) return NextResponse.json({ error: check.error }, { status: 400 });
    }

    const eleves = await loadElevesRegistry();
    const matched = findElevesByParentEmail(eleves, validated.value.email);
    const parentContext = {
      source: "parent_portal" as const,
      matched: matched.length > 0,
      children: toParentLinkedChildren(matched),
    };

    const { firstName, lastName, email, phone, subject, description } = validated.value;
    let token: string | undefined;
    try {
      token = generatePendingRequestToken();
      const fileBufs = await Promise.all(
        rawFiles.map(async (f) => ({
          buffer: Buffer.from(await f.arrayBuffer()),
          fileName: f.name,
          contentType: f.type || "application/octet-stream",
        })),
      );
      await savePendingRequestWithFiles(
        token,
        { firstName, lastName, email, phone, subject, description, parentContext },
        fileBufs,
      );
      const confirmUrl = `${base}/api/requests/confirm?token=${encodeURIComponent(token)}`;
      await notifyRequestPendingVerification(email, firstName, confirmUrl);
    } catch (e) {
      console.error("[parent-portal] pending", e);
      if (token) {
        try {
          await deletePendingRequestPrefix(token);
        } catch {
          /* ignore */
        }
      }
      return NextResponse.json(
        { error: "Impossible d’envoyer l’e-mail de confirmation. Réessayez plus tard." },
        { status: 500 },
      );
    }

    return NextResponse.json({
      success: true,
      needsEmailVerification: true,
      message:
        "Un e-mail vient de vous être envoyé. Cliquez sur le lien pour valider votre demande — cela confirme que l’adresse est bien la vôtre.",
    });
  } catch (e) {
    console.error("[parent-portal] POST", e);
    return NextResponse.json({ error: "Erreur serveur." }, { status: 500 });
  }
}
