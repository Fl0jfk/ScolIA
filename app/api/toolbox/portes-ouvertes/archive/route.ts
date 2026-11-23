import { NextResponse } from "next/server";
import { z } from "zod";
import { requireModule } from "@/app/lib/intranet-auth";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { canManagePortesOuvertesParametrage } from "@/app/lib/portes-ouvertes-access";
import {
  archiveAndResetPortesOuvertes,
  getPortesOuvertesArchive,
  listPortesOuvertesArchives,
} from "@/app/lib/portes-ouvertes-archive";

const PostSchema = z.object({
  newFirstDay: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  /** Garde-fou : doit valoir true (confirmation explicite côté écran). */
  confirm: z.literal(true),
});

export async function GET(req: Request) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  try {
    const id = new URL(req.url).searchParams.get("id");
    if (id) {
      const archive = await getPortesOuvertesArchive(id);
      if (!archive) return NextResponse.json({ error: "Archive introuvable." }, { status: 404 });
      return NextResponse.json({ archive });
    }
    return NextResponse.json({ archives: await listPortesOuvertesArchives() });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;
  const user = gate.ctx.user;
  if (!canManagePortesOuvertesParametrage(user.roles, isOrgAdminFromAppUser(user))) {
    return NextResponse.json({ error: "Droits insuffisants." }, { status: 403 });
  }
  const parsed = PostSchema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "Date ou confirmation manquante." }, { status: 400 });
  }
  try {
    const name =
      [user.firstName, user.lastName].filter(Boolean).join(" ") || user.name || "Utilisateur";
    const result = await archiveAndResetPortesOuvertes({
      newFirstDay: parsed.data.newFirstDay,
      actor: { userId: user.id, name },
    });
    return NextResponse.json({ success: true, ...result });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : String(e) },
      { status: 400 },
    );
  }
}
