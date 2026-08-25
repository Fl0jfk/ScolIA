import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  deleteCalendrierEntry,
  deleteEdtCreneau,
  listCalendrierEntries,
  listEdtCreneaux,
  seedDefaultCalendrier,
  upsertCalendrierEntry,
  upsertEdtCreneau,
} from "@/app/lib/vs-calendrier-db";
import { listGroupes } from "@/app/lib/groupes-pedagogiques-db";
import { isProfesseurScopedDossierViewer } from "@/app/lib/eleve-dossier-scope";
import { listAssignedClassesForTeacher } from "@/app/lib/eleve-dossier-prof";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

function foldName(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

function teacherNameMatches(
  enseignantNom: string | null | undefined,
  user: { name?: string; firstName?: string; lastName?: string },
): boolean {
  const target = foldName(enseignantNom || "");
  if (!target) return false;
  const candidates = [
    user.name,
    `${user.firstName || ""} ${user.lastName || ""}`,
    `${user.lastName || ""} ${user.firstName || ""}`,
    user.lastName,
    user.firstName,
  ]
    .map((x) => foldName(String(x || "")))
    .filter((x) => x.length >= 2);
  return candidates.some((c) => target.includes(c) || c.includes(target));
}

function canManageCalendrier(roles: string[], orgAdmin: boolean): boolean {
  if (orgAdmin || hasGlobalAdminRole(roles) || roles.includes("admin")) return true;
  if (INTRANET_DIRECTION_SLUGS.some((s) => roles.includes(s))) return true;
  return (
    hasRole(roles, "administratif") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "surveillant")
  );
}

export async function GET(req: Request) {
  const gate = await requireModule("vs-calendrier");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const user = gate.ctx.user;
  const orgAdmin = isOrgAdminFromAppUser(user);
  const selfOnly = isProfesseurScopedDossierViewer({
    roles: user.roles,
    orgAdmin,
    platformAdmin: user.platformAdmin,
  });
  const canManage = canManageCalendrier(user.roles, orgAdmin);

  const url = new URL(req.url);
  const classe = selfOnly ? undefined : url.searchParams.get("classe")?.trim() || undefined;
  const groupeId = selfOnly ? undefined : url.searchParams.get("groupeId")?.trim() || undefined;

  const [calendrier, creneauxRaw, groupes, anneeCourante] = await Promise.all([
    listCalendrierEntries(etabId),
    listEdtCreneaux(etabId, { classe, groupeId }),
    listGroupes(etabId),
    import("@/app/lib/annees-scolaires-db").then((m) => m.resolveAnneeCouranteMeta(etabId)),
  ]);

  let creneaux = creneauxRaw;
  if (selfOnly) {
    const assigned = await listAssignedClassesForTeacher(user.businessUserId);
    const assignedSet = new Set(assigned.map((c) => c.trim().toUpperCase()));
    creneaux = creneauxRaw.filter((c) => {
      if (teacherNameMatches(c.enseignantNom, user)) return true;
      const cls = (c.classe || "").trim().toUpperCase();
      return Boolean(cls && assignedSet.has(cls));
    });
  }

  const { detectEdtConflicts } = await import("@/app/lib/edt-conflicts");
  const conflits = selfOnly
    ? []
    : detectEdtConflicts(
        creneaux.map((c) => ({
          id: c.id,
          jourSemaine: c.jourSemaine,
          heureDebut: c.heureDebut,
          heureFin: c.heureFin,
          classe: c.classe,
          groupeId: c.groupeId,
          groupeCode: c.groupeCode,
          enseignantNom: c.enseignantNom,
          salle: c.salle,
          semaine: c.semaine,
        })),
      );

  return NextResponse.json({
    calendrier,
    creneaux,
    groupes: selfOnly ? [] : groupes,
    anneeCourante,
    conflits,
    selfOnly,
    canManage,
  });
}

export async function POST(req: Request) {
  const gate = await requireModule("vs-calendrier");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const user = gate.ctx.user;
  const orgAdmin = isOrgAdminFromAppUser(user);
  if (!canManageCalendrier(user.roles, orgAdmin)) {
    return NextResponse.json(
      { error: "Consultation seule — modification réservée à la vie scolaire / direction." },
      { status: 403 },
    );
  }

  const body = await req.json().catch(() => null);
  const action = String(body?.action || "");

  try {
    if (action === "seedDefaults") {
      const result = await seedDefaultCalendrier(etabId);
      return NextResponse.json(result);
    }

    if (action === "upsertCalendrier") {
      const row = await upsertCalendrierEntry(etabId, body.entry || body);
      return NextResponse.json({ ok: true, row });
    }

    if (action === "deleteCalendrier") {
      const id = String(body?.id || "");
      if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });
      await deleteCalendrierEntry(etabId, id);
      return NextResponse.json({ ok: true });
    }

    if (action === "upsertCreneau") {
      const payload = body.creneau || body.entry || body;
      const row = await upsertEdtCreneau(etabId, {
        ...payload,
        force: Boolean(body.force || payload.force),
      });
      return NextResponse.json({ ok: true, row });
    }

    if (action === "deleteCreneau") {
      const id = String(body?.id || "");
      if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });
      await deleteEdtCreneau(etabId, id);
      return NextResponse.json({ ok: true });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Opération impossible." },
      { status: 400 },
    );
  }
}
