import { after, NextResponse } from "next/server";
import { z } from "zod";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { getTenantAppUrl, getTenant, runWithTenant } from "@/app/lib/tenant-context";
import {
  buildStaffFeedView,
  moderateFeedPost,
  notifyAfterPublish,
  openFeedAndSendLinks,
  publishFeedText,
  resolveFeedStaffContext,
  sendFeedWelcomeLinks,
  setDownloadAllowed,
} from "@/app/lib/travels-feed-store";

/**
 * « Fil du voyage » côté profs : organisateur, accompagnateurs de CE voyage, direction (modération), admin général.
 * Jamais de cache. Aucune donnée dans l'URL hormis l'identifiant du voyage.
 */
const NO_STORE = { "Cache-Control": "private, no-store, max-age=0" };
const json = (body: unknown, status = 200) => NextResponse.json(body, { status, headers: NO_STORE });
const publishLimiter = createMemoryRateLimiter({ name: "voyfeed-publish", windowMs: 60 * 60 * 1000, max: 120 });

const bodySchema = z.discriminatedUnion("action", [
  z.object({ action: z.literal("open") }),
  z.object({ action: z.literal("access") }),
  z.object({
    action: z.literal("publish"),
    kind: z.string().max(40),
    body: z.string().max(2000).optional(),
    delayMinutes: z.number().int().optional(),
  }),
  z.object({ action: z.literal("moderate"), postId: z.string().max(64), op: z.enum(["hide", "unhide", "delete"]) }),
  z.object({ action: z.literal("download"), allow: z.boolean() }),
]);

type Params = { params: Promise<{ tripId: string }> };

export async function GET(_req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { tripId } = await params;
  try {
    const me = await safeCurrentUser();
    const ctx = await resolveFeedStaffContext(tripId, me);
    if (!ctx) return json({ error: "Accès refusé." }, 403);
    return json(await buildStaffFeedView(ctx));
  } catch (e) {
    console.error("[travels/feed] lecture", e instanceof Error ? e.message : "erreur");
    return json({ error: "Chargement impossible." }, 500);
  }
}

export async function POST(req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { tripId } = await params;
  const parsed = bodySchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return json({ error: "Requête invalide." }, 400);
  const input = parsed.data;
  try {
    const me = await safeCurrentUser();
    const ctx = await resolveFeedStaffContext(tripId, me);
    if (!ctx) return json({ error: "Accès refusé." }, 403);

    // Résolution du contexte réussie = au moins un droit (publier ou modérer) sur CE voyage.
    if (input.action === "access") return json({ allowed: true });
    if (input.action === "open") {
      const res = await openFeedAndSendLinks(ctx);
      if (!res.ok) return json({ error: res.error }, res.status);
      if (!res.busy && res.toSend > 0) {
        // Les e-mails de bienvenue partent APRÈS la réponse (contexte établissement conservé), la page répond tout de suite.
        const tenant = await getTenant();
        const baseUrl = await getTenantAppUrl();
        after(() =>
          runWithTenant(tenant, () => sendFeedWelcomeLinks(ctx.etabId, ctx.trip, baseUrl)).catch((e) =>
            console.error("[travels/feed] liens de bienvenue", e instanceof Error ? e.message : "erreur"),
          ),
        );
      }
      return json({ ...res, sending: !res.busy && res.toSend > 0 });
    }
    if (input.action === "moderate") {
      const res = await moderateFeedPost(ctx, input.postId, input.op);
      return res.ok ? json({ ok: true }) : json({ error: res.error }, res.status);
    }
    if (input.action === "download") {
      const res = await setDownloadAllowed(ctx, input.allow);
      return res.ok ? json({ ok: true }) : json({ error: res.error }, res.status);
    }
    // publish
    if (!(await publishLimiter.allow(`${me?.id ?? clientIpFromRequest(req)}:${tripId}`))) {
      return json({ error: "Trop de publications. Réessayez plus tard." }, 429);
    }
    const res = await publishFeedText(ctx, { kind: input.kind, body: input.body, delayMinutes: input.delayMinutes });
    if (!res.ok) return json({ error: res.error }, res.status);
    const tenant = await getTenant();
    const baseUrl = await getTenantAppUrl();
    // E-mail regroupé aux familles, après la réponse (au plus un par famille et par heure ; le reste est repris par le job).
    after(() =>
      runWithTenant(tenant, () => notifyAfterPublish(ctx.etabId, ctx.trip.id, baseUrl)).catch((e) =>
        console.error("[travels/feed] notification", e instanceof Error ? e.message : "erreur"),
      ),
    );
    return json({ ok: true, postId: res.postId });
  } catch (e) {
    console.error("[travels/feed] action", e instanceof Error ? e.message : "erreur");
    return json({ error: "Action impossible." }, 500);
  }
}
