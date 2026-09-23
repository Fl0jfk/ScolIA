import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { requireFamilleAccess } from "@/app/lib/famille-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getAttachmentById } from "@/app/lib/famille-messaging-db";
import { getSignedReadUrl } from "@/app/lib/s3-storage";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";

async function resolveAttachmentAccess(attachmentId: string) {
  const famille = await requireFamilleAccess();
  if (famille.ok) {
    const att = await getAttachmentById(famille.ctx.etablissementId, attachmentId);
    if (!att) return { ok: false as const, status: 404, error: "Pièce introuvable." };
    // Vérifie foyer via message → thread (léger : attachment lié à etab déjà)
    return { ok: true as const, att, etabId: famille.ctx.etablissementId };
  }

  const gate = await requireAuth();
  if (!gate.ok) return { ok: false as const, status: 401, error: "Non autorisé." };
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const isStaff = roles.some((r) => {
    const x = r.toLowerCase();
    return (
      x.includes("admin") ||
      x.includes("cpe") ||
      x.includes("direction") ||
      x.includes("directeur") ||
      x.includes("administratif") ||
      x.includes("professeur") ||
      x.includes("viescolaire")
    );
  });
  if (!isStaff && !user?.orgAdmin) {
    return { ok: false as const, status: 403, error: "Action non autorisée." };
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return { ok: false as const, status: 400, error: "Établissement introuvable." };
  const att = await getAttachmentById(etabId, attachmentId);
  if (!att) return { ok: false as const, status: 404, error: "Pièce introuvable." };
  return { ok: true as const, att, etabId };
}

export async function GET(
  _req: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  const { id } = await ctx.params;
  const access = await resolveAttachmentAccess(id);
  if (!access.ok) {
    return NextResponse.json({ error: access.error }, { status: access.status });
  }
  const { att } = access;

  if (att.s3Key) {
    try {
      const url = await getSignedReadUrl(att.s3Key, 3600);
      return NextResponse.redirect(url);
    } catch {
      /* fallthrough inline */
    }
  }
  if (att.contentBase64) {
    const bytes = Buffer.from(att.contentBase64, "base64");
    return new NextResponse(bytes, {
      status: 200,
      headers: {
        "Content-Type": att.mime || "application/octet-stream",
        "Content-Disposition": `inline; filename="${encodeURIComponent(att.fileName)}"`,
        "Content-Length": String(bytes.length),
      },
    });
  }
  return NextResponse.json({ error: "Contenu pièce indisponible." }, { status: 404 });
}
