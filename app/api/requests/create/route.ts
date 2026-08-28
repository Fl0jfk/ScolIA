import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";

import { requireAuth } from "@/app/lib/intranet-auth";
import {
  RequestRecord,
  getPublicAppBaseUrl,
  notifyRequestCreated,
  notifyRequestPendingVerification,
  resolveForcedRequestRouting,
  resolveRequestRouting,
  saveRequestFile,
  saveRequestsIndex,
  getRequestsIndex,
  validateRequestInput,
  uploadBuffersAsRequestAttachments,
  assertEligibleRequestAttachment,
  MAX_REQUEST_ATTACHMENTS_PER_UPLOAD,
} from "@/app/lib/requests";
import { deletePendingRequestPrefix, generatePendingRequestToken, savePendingRequestWithFiles } from "@/app/lib/request-pending-verify";
import { RH_REQUEST_ROUTE_ID } from "@/app/lib/requests-routing-defaults";
import { getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { clientIpFromRequest, createSlidingWindowRateLimiter } from "@/app/lib/memory-rate-limit";

export const runtime = "nodejs";

const anonymousCreateLimiter = createSlidingWindowRateLimiter({
  windowMs: 60 * 60 * 1000,
  max: 8,
});

/** Routes que le client peut forcer (formulaire module RH). */
const ALLOWED_FORCE_ROUTE_IDS = new Set([RH_REQUEST_ROUTE_ID]);

function parseCreatePayload(
  contentType: string,
  bodyJson: unknown,
  form: FormData | null,
): {
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  subject: string;
  description: string;
  userId: string | null;
  forceRouteId: string | null;
  files: File[];
  honeypot: string;
} {
  if (form) {
    const g = (k: string) => {
      const v = form.get(k);
      return typeof v === "string" ? v : "";
    };
    const rawFiles = form.getAll("files").filter((x): x is File => x instanceof File && x.size > 0);
    const forceRouteId = g("forceRouteId").trim() || null;
    return {
      firstName: g("firstName"),
      lastName: g("lastName"),
      email: g("email"),
      phone: g("phone"),
      subject: g("subject"),
      description: g("description"),
      userId: null,
      forceRouteId,
      files: rawFiles.slice(0, MAX_REQUEST_ATTACHMENTS_PER_UPLOAD),
      honeypot: String(form.get("website") || form.get("company") || "").trim(),
    };
  }
  const b = bodyJson as Record<string, unknown>;
  const forceRouteId =
    typeof b?.forceRouteId === "string" && b.forceRouteId.trim() ? b.forceRouteId.trim() : null;
  return {
    firstName: String(b?.firstName ?? ""),
    lastName: String(b?.lastName ?? ""),
    email: String(b?.email ?? ""),
    phone: String(b?.phone ?? ""),
    subject: String(b?.subject ?? ""),
    description: String(b?.description ?? ""),
    userId: typeof b?.userId === "string" ? b.userId : null,
    forceRouteId,
    files: [],
    honeypot: String(b?.website || b?.company || "").trim(),
  };
}

export async function POST(req: Request) {
  try {
    const session = await resolveSession();
    const userId = session?.userId;
    const contentType = req.headers.get("content-type") || "";
    let payload: ReturnType<typeof parseCreatePayload>;
    if (contentType.includes("multipart/form-data")) {
      const form = await req.formData();
      payload = parseCreatePayload(contentType, null, form);
    } else {
      const body = await req.json();
      payload = parseCreatePayload(contentType, body, null);
    }
    payload.userId = userId ?? null;
    if (!userId) {
      if (!(await anonymousCreateLimiter.allow(clientIpFromRequest(req)))) {
        return NextResponse.json(
          { error: "Trop de tentatives. Réessayez plus tard." },
          { status: 429 },
        );
      }
      if (payload.honeypot) {
        return NextResponse.json({
          success: true,
          needsEmailVerification: true,
          message:
            "Un e-mail de confirmation vient de vous être envoyé si l’adresse est valide.",
        });
      }
    }
    const validated = validateRequestInput({
      firstName: payload.firstName,
      lastName: payload.lastName,
      email: payload.email,
      phone: payload.phone,
      subject: payload.subject,
      description: payload.description,
      userId: payload.userId || null,
    });
    if (!validated.ok) return NextResponse.json({ error: validated.error }, { status: 400 });
    if (payload.files.length > MAX_REQUEST_ATTACHMENTS_PER_UPLOAD) {
      return NextResponse.json(
        { error: `Maximum ${MAX_REQUEST_ATTACHMENTS_PER_UPLOAD} fichiers par envoi.` },
        { status: 400 },
      );
    }
    for (const f of payload.files) {
      const check = assertEligibleRequestAttachment(f.name, f.type, f.size);
      if (!check.ok) return NextResponse.json({ error: check.error }, { status: 400 });
    }
    if (!userId) {
      if (!(await getTenantSmtpConfig())) {
        return NextResponse.json(
          {
            error:
              "Pour les personnes non connectées, l’envoi d’une demande nécessite la confirmation par e-mail. Merci de vous connecter avec votre compte ou contactez l’établissement.",
          },
          { status: 503 },
        );
      }
      const base = await getPublicAppBaseUrl();
      if (!base) {
        return NextResponse.json(
          {
            error:
              "Configuration incomplète (NEXT_PUBLIC_APP_URL). Les demandes anonymes ne peuvent pas être confirmées par e-mail pour le moment.",
          },
          { status: 503 },
        );
      }
      const { firstName, lastName, email, phone, subject, description } = validated.value;
      let token: string | undefined;
      try {
        token = generatePendingRequestToken();
        const fileBufs = await Promise.all(
          payload.files.map(async (f) => ({
            buffer: Buffer.from(await f.arrayBuffer()),
            fileName: f.name,
            contentType: f.type || "application/octet-stream",
          })),
        );
        await savePendingRequestWithFiles(
          token,
          { firstName, lastName, email, phone, subject, description },
          fileBufs,
        );
        const confirmUrl = `${base}/api/requests/confirm?token=${encodeURIComponent(token)}`;
        await notifyRequestPendingVerification(email, firstName, confirmUrl);
      } catch (e) {
        console.error("Pending request / email:", e);
        if (token) {
          try {
            await deletePendingRequestPrefix(token);
          } catch {
            /* ignore */
          }
        }
        return NextResponse.json(
          {
            error:
              "Impossible d’envoyer l’e-mail de confirmation. Vérifiez l’adresse ou réessayez plus tard.",
          },
          { status: 500 },
        );
      }
      return NextResponse.json({
        success: true,
        needsEmailVerification: true,
        message:
          "Un e-mail vient de vous être envoyé à l’adresse indiquée. Cliquez sur le lien qu’il contient pour valider votre demande et confirmer que l’e-mail est bien le vôtre.",
      });
    }
    const orgGate = await requireAuth();
    if (!orgGate.ok) return orgGate.response;

    const now = new Date().toISOString();
    const id = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const { firstName, lastName, email, phone, subject, description } = validated.value;

    let routing = await resolveRequestRouting(subject, description);
    const forceRouteId = payload.forceRouteId;
    if (forceRouteId) {
      if (!ALLOWED_FORCE_ROUTE_IDS.has(forceRouteId)) {
        return NextResponse.json({ error: "Orientation de demande non autorisée." }, { status: 400 });
      }
      const forced = await resolveForcedRequestRouting(forceRouteId);
      if (!forced) {
        return NextResponse.json({ error: "File RH introuvable." }, { status: 400 });
      }
      routing = forced;
    }

    let attachments: RequestRecord["attachments"];
    if (payload.files.length > 0) {
      const bufs = await Promise.all(
        payload.files.map(async (f) => ({
          buffer: Buffer.from(await f.arrayBuffer()),
          fileName: f.name,
          contentType: f.type || "application/octet-stream",
        })),
      );
      attachments = await uploadBuffersAsRequestAttachments(id, bufs);
    }
    const record: RequestRecord = {
      id,
      createdAt: now,
      updatedAt: now,
      status: "NOUVELLE",
      category: routing.category,
      subject,
      description,
      requester: {
        firstName,
        lastName,
        fullName: `${firstName} ${lastName}`,
        email,
        phone,
        userId: validated.value.userId || null,
      },
      assignedTo: routing.assignedTo,
      routing: {
        source: routing.source,
        confidence: routing.confidence,
        reason: routing.reason,
        ...(routing.suggestedRouteId ? { suggestedRouteId: routing.suggestedRouteId } : {}),
        ...(routing.directionHint ? { directionHint: routing.directionHint } : {}),
        ...(routing.routingMeta?.assignmentId ? { assignmentId: routing.routingMeta.assignmentId } : {}),
        ...(routing.routingMeta?.taskId ? { taskId: routing.routingMeta.taskId } : {}),
        ...(routing.routingMeta?.unitId ? { unitId: routing.routingMeta.unitId } : {}),
        ...(routing.routingMeta?.servicePile ? { servicePile: routing.routingMeta.servicePile } : {}),
      },
      ...(attachments?.length ? { attachments } : {}),
      comments: [],
      history: [
        {
          at: now,
          by: `${firstName} ${lastName}`,
          action: "CREATION",
          note: `Demande créée — route ${routing.assignedTo.routeId ?? routing.assignedTo.unit} (${routing.assignedTo.roleLabel}).${attachments?.length ? ` ${attachments.length} pièce(s) jointe(s).` : ""}`,
        },
      ],
    };
    await saveRequestFile(record);
    const index = await getRequestsIndex();
    index.push(record);
    await saveRequestsIndex(index);
    try {
      await notifyRequestCreated(record);
    } catch (mailError) {
      console.error("Request create notification error:", mailError);
    }
    return NextResponse.json({
      success: true,
      id: record.id,
      status: record.status,
      assignedTo: record.assignedTo,
      category: record.category,
      attachmentCount: attachments?.length ?? 0,
    });
  } catch (error) {
    console.error("Request create error:", error);
    const msg = error instanceof Error ? error.message : "Erreur création demande";
    return NextResponse.json({ error: msg }, { status: 500 });
  }
}
