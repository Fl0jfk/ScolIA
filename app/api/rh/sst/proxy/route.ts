import { NextResponse } from "next/server";
import { z } from "zod";
import { requireAppUser } from "@/app/lib/app-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { canManageSstRegistre } from "@/app/lib/sst-registre/access";
import {
  listSstProxyPeople,
  listSstStaffOptions,
  setSstProxyPeople,
} from "@/app/lib/sst-registre/db";

export async function GET() {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canManageSstRegistre(appUser.user.roles)) {
    return NextResponse.json({ error: "Accès réservé au pilotage RH / direction." }, { status: 403 });
  }

  try {
    const [people, staff] = await Promise.all([
      listSstProxyPeople(scope.ctx.etablissementId),
      listSstStaffOptions(scope.ctx.etablissementId),
    ]);
    return NextResponse.json({ people, staff });
  } catch (e) {
    console.error("[rh/sst/proxy GET]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur paramétrage SST." },
      { status: 500 },
    );
  }
}

const SaveSchema = z.object({
  people: z
    .array(
      z.object({
        userId: z.string().min(1).max(200),
        firstName: z.string().max(120).optional().default(""),
        lastName: z.string().max(120).optional().default(""),
        email: z.string().max(200).optional().default(""),
      }),
    )
    .max(200),
});

export async function PUT(req: Request) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  if (!canManageSstRegistre(appUser.user.roles)) {
    return NextResponse.json({ error: "Accès réservé au pilotage RH / direction." }, { status: 403 });
  }

  let json: unknown;
  try {
    json = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  const parsed = SaveSchema.safeParse(json);
  if (!parsed.success) {
    return NextResponse.json({ error: "Données invalides." }, { status: 400 });
  }

  try {
    const staff = await listSstStaffOptions(scope.ctx.etablissementId);
    const byId = new Map(staff.map((s) => [s.userId, s]));
    const people = parsed.data.people.map((p) => {
      const hit = byId.get(p.userId.trim());
      return {
        userId: p.userId.trim(),
        firstName: (p.firstName || hit?.firstName || "").trim(),
        lastName: (p.lastName || hit?.lastName || "").trim(),
        email: (p.email || hit?.email || "").trim(),
      };
    });
    const saved = await setSstProxyPeople({
      etablissementId: scope.ctx.etablissementId,
      people,
    });
    return NextResponse.json({ ok: true, people: saved });
  } catch (e) {
    console.error("[rh/sst/proxy PUT]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur enregistrement." },
      { status: 500 },
    );
  }
}
