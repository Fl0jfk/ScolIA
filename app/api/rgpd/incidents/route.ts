import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import {
  listRgpdIncidents,
  loadRgpdWorkspace,
  newRgpdId,
  saveRgpdIncident,
  saveRgpdWorkspace,
} from "@/app/lib/rgpd-storage";
import type {
  RgpdDataBreachFields,
  RgpdIncident,
  RgpdIncidentKind,
  RgpdSecurityIncidentFields,
} from "@/app/lib/rgpd-types";


export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  if (!canAccessRgpdModule(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }
  const workspace = await loadRgpdWorkspace();
  const incidents = await listRgpdIncidents(workspace);
  return NextResponse.json({ incidents });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessRgpdModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  try {
    const body = await req.json();
    const kind = body.kind as RgpdIncidentKind;
    if (kind !== "violation_donnees" && kind !== "incident_securite") {
      return NextResponse.json({ error: "Type d'incident invalide." }, { status: 400 });
    }

    const now = new Date().toISOString();
    const userName =
      [user?.firstName, user?.lastName].filter(Boolean).join(" ") ||
      user?.emailAddresses?.[0]?.emailAddress ||
      "Utilisateur";

    const incident: RgpdIncident = {
      id: newRgpdId("inc"),
      kind,
      title: String(body.title || "Incident").trim(),
      createdAt: now,
      updatedAt: now,
      createdBy: { userId: user?.id || "", name: userName },
      fields: (body.fields || {}) as RgpdDataBreachFields | RgpdSecurityIncidentFields,
      chatHistory: Array.isArray(body.chatHistory) ? body.chatHistory : undefined,
    };

    await saveRgpdIncident(incident);
    const workspace = await loadRgpdWorkspace();
    workspace.incidents = [...(workspace.incidents ?? []), incident.id];
    workspace.history = [
      ...(workspace.history ?? []).slice(-99),
      {
        at: now,
        by: userName,
        action: "INCIDENT_CREATED",
        note: incident.title,
      },
    ];
    await saveRgpdWorkspace(workspace);

    return NextResponse.json({ incident });
  } catch (e) {
    console.error("[rgpd/incidents POST]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur enregistrement" },
      { status: 500 },
    );
  }
}
