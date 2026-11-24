import { jsonNoStore, uploadLimiter, withPublicContext } from "@/app/lib/travels-v2-public-api";
import { publicViewFor, removeFamilyDocument, storeFamilyDocument } from "@/app/lib/travels-v2-registration-store";
import { DOC_MAX_BYTES } from "@/app/lib/travels-v2-registration-types";

type Params = { params: Promise<{ token: string }> };

/** Dépôt d'une pièce (multipart : `docId` + `file`). PDF ou image, 8 Mo maximum, stockage objet privé. */
export async function POST(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token, { write: true });
  if (!g.ok) return g.response;
  try {
    if (!(await uploadLimiter.allow(g.ctx.inv.id))) {
      return jsonNoStore({ error: "Trop de dépôts. Réessayez plus tard." }, 429);
    }
    const declared = Number(req.headers.get("content-length") || 0);
    if (declared > DOC_MAX_BYTES + 512 * 1024) {
      return jsonNoStore({ error: "Fichier trop volumineux (8 Mo maximum)." }, 413);
    }
    const form = await req.formData().catch(() => null);
    const docId = String(form?.get("docId") || "");
    const file = form?.get("file");
    if (!docId || !(file instanceof File)) return jsonNoStore({ error: "Dépôt invalide." }, 400);
    if (file.size > DOC_MAX_BYTES) return jsonNoStore({ error: "Fichier trop volumineux (8 Mo maximum)." }, 413);
    const bytes = Buffer.from(await file.arrayBuffer());
    const res = await storeFamilyDocument(g.ctx, docId, file.name || "document", bytes);
    if (!res.ok) return jsonNoStore({ error: res.error }, res.status);
    return jsonNoStore(await publicViewFor(g.ctx));
  } catch (e) {
    console.error("[voyages-famille] upload", e);
    return jsonNoStore({ error: "Dépôt impossible." }, 500);
  }
}

/** Retrait d'une pièce déposée (tant que la réponse n'est pas signée). Corps JSON : { docId }. */
export async function DELETE(req: Request, { params }: Params) {
  const { token } = await params;
  const g = await withPublicContext(req, token, { write: true });
  if (!g.ok) return g.response;
  try {
    const body = (await req.json().catch(() => null)) as { docId?: unknown } | null;
    const docId = typeof body?.docId === "string" ? body.docId : "";
    if (!docId) return jsonNoStore({ error: "Requête invalide." }, 400);
    const res = await removeFamilyDocument(g.ctx, docId);
    if (!res.ok) return jsonNoStore({ error: res.error }, res.status);
    return jsonNoStore(await publicViewFor(g.ctx));
  } catch (e) {
    console.error("[voyages-famille] upload delete", e);
    return jsonNoStore({ error: "Suppression impossible." }, 500);
  }
}
