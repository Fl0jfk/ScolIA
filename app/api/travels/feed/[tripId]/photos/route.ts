import { after, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { getTenant, getTenantAppUrl, runWithTenant } from "@/app/lib/tenant-context";
import { PHOTOS_MAX_PER_POST } from "@/app/lib/travels-feed-logic";
import { FEED_IMAGE_MAX_INPUT_BYTES } from "@/app/lib/travels-feed-image";
import { notifyAfterPublish, publishFeedPhotos, resolveFeedStaffContext } from "@/app/lib/travels-feed-store";

/**
 * Publication de photos (multipart) : champs `files` (1 à 10), `ack` (« aucun de ces élèves n'est reconnaissable »),
 * `caption` (facultatif). Les octets sont contrôlés, nettoyés (EXIF/GPS retirés) et redimensionnés CÔTÉ SERVEUR.
 */
const NO_STORE = { "Cache-Control": "private, no-store, max-age=0" };
const json = (body: unknown, status = 200) => NextResponse.json(body, { status, headers: NO_STORE });
const photoLimiter = createMemoryRateLimiter({ name: "voyfeed-photos", windowMs: 60 * 60 * 1000, max: 60 });
const TOTAL_MAX_BYTES = 40 * 1024 * 1024;

type Params = { params: Promise<{ tripId: string }> };

export async function POST(req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { tripId } = await params;
  try {
    const me = await safeCurrentUser();
    const ctx = await resolveFeedStaffContext(tripId, me);
    if (!ctx) return json({ error: "Accès refusé." }, 403);
    if (!(await photoLimiter.allow(`${me?.id ?? clientIpFromRequest(req)}:${tripId}`))) {
      return json({ error: "Trop de publications. Réessayez plus tard." }, 429);
    }

    // Taille contrôlée AVANT de lire le corps (jamais de mise en mémoire d'un envoi démesuré).
    const declared = Number(req.headers.get("content-length"));
    if (!Number.isFinite(declared) || declared <= 0) return json({ error: "Longueur de la requête requise." }, 411);
    if (declared > TOTAL_MAX_BYTES + 1024 * 1024) return json({ error: "Envoi trop volumineux." }, 413);
    const form = await req.formData().catch(() => null);
    if (!form) return json({ error: "Requête invalide." }, 400);
    const entries = form.getAll("files").filter((f): f is File => typeof f === "object" && f !== null && "arrayBuffer" in f);
    if (entries.length === 0) return json({ error: "Ajoutez au moins une photo." }, 400);
    if (entries.length > PHOTOS_MAX_PER_POST) return json({ error: `${PHOTOS_MAX_PER_POST} photos au maximum par publication.` }, 400);
    let total = 0;
    const files: Buffer[] = [];
    for (const f of entries) {
      if (f.size > FEED_IMAGE_MAX_INPUT_BYTES) return json({ error: "Photo trop volumineuse." }, 413);
      total += f.size;
      if (total > TOTAL_MAX_BYTES) return json({ error: "Envoi trop volumineux." }, 413);
      files.push(Buffer.from(await f.arrayBuffer()));
    }
    const res = await publishFeedPhotos(ctx, { files, acknowledged: form.get("ack") === "1", caption: form.get("caption") });
    if (!res.ok) return json({ error: res.error }, res.status);
    const tenant = await getTenant();
    const baseUrl = await getTenantAppUrl();
    after(() =>
      runWithTenant(tenant, () => notifyAfterPublish(ctx.etabId, ctx.trip.id, baseUrl)).catch((e) =>
        console.error("[travels/feed] notification", e instanceof Error ? e.message : "erreur"),
      ),
    );
    return json({ ok: true, postId: res.postId });
  } catch (e) {
    console.error("[travels/feed] photos", e instanceof Error ? e.message : "erreur");
    return json({ error: "Publication impossible." }, 500);
  }
}
