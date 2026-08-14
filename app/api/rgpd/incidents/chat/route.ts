import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { chatRgpdIncidentWithAi } from "@/app/lib/rgpd-ai";
import type {
  RgpdDataBreachFields,
  RgpdIncidentKind,
  RgpdSecurityIncidentFields,
} from "@/app/lib/rgpd-types";

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

    const message = String(body.message || "").trim();
    if (!message) {
      return NextResponse.json({ error: "Message requis." }, { status: 400 });
    }

    const history = Array.isArray(body.history) ? body.history : [];
    const currentFields = (body.fields || {}) as
      | RgpdDataBreachFields
      | RgpdSecurityIncidentFields;

    const result = await chatRgpdIncidentWithAi({
      kind,
      message,
      history,
      currentFields,
    });

    return NextResponse.json(result);
  } catch (e) {
    console.error("[rgpd/incidents/chat]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Assistant indisponible" },
      { status: 500 },
    );
  }
}
