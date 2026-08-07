import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  canEditOrganigramme,
  loadOrganigramConfig,
  parseOrganigramConfig,
  saveOrganigramConfig,
  type OrganigramConfig,
} from "@/app/lib/organigramme-config";
import {
  suggestSlotsFromPersonnel,
  suggestSlotsFromRouting,
} from "@/app/lib/organigramme-resolve";
import { getPersonnelIndex } from "@/app/lib/personnel-storage";
import { getRequestsRoutingConfig } from "@/app/lib/requests-routing-config";

function rolesFromUser(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const rolesRaw = user?.publicMetadata?.role;
  return Array.isArray(rolesRaw) ? rolesRaw.map(String) : rolesRaw ? [String(rolesRaw)] : [];
}

async function assertCanEdit() {
  const gate = await requireAuth();
  if (!gate.ok) return gate;
  const user = await safeCurrentUser();
  const roles = rolesFromUser(user);
  if (!canEditOrganigramme(roles)) {
    return {
      ok: false as const,
      response: NextResponse.json({ error: "Édition réservée à l'administratif / RH." }, { status: 403 }),
    };
  }
  return gate;
}

export async function GET(req: Request) {
  const gate = await assertCanEdit();
  if (!gate.ok) return gate.response;

  try {
    const config = await loadOrganigramConfig();
    const url = new URL(req.url);
    const suggest = url.searchParams.get("suggest") === "1";

    if (!suggest) {
      return NextResponse.json({ config });
    }

    const [personnelIndex, routing] = await Promise.all([
      getPersonnelIndex().catch(() => []),
      getRequestsRoutingConfig().catch(() => null),
    ]);

    const fromRouting = routing ? suggestSlotsFromRouting(config, routing) : [];
    const fromPersonnel = suggestSlotsFromPersonnel(config, personnelIndex);

    return NextResponse.json({
      config,
      suggestions: [...fromRouting, ...fromPersonnel],
    });
  } catch (e) {
    console.error("[organigramme/config] GET", e);
    return NextResponse.json({ error: "Impossible de charger la config." }, { status: 500 });
  }
}

export async function PUT(req: Request) {
  const gate = await assertCanEdit();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const action = typeof body?.action === "string" ? body.action : "save";

    if (action === "applySuggestions") {
      const current = await loadOrganigramConfig();
      const [personnelIndex, routing] = await Promise.all([
        getPersonnelIndex().catch(() => []),
        getRequestsRoutingConfig().catch(() => null),
      ]);
      const fromRouting = routing ? suggestSlotsFromRouting(current, routing) : [];
      const fromPersonnel = suggestSlotsFromPersonnel(current, personnelIndex);
      const merged: OrganigramConfig = {
        ...current,
        slots: [...current.slots, ...fromRouting, ...fromPersonnel],
      };
      const saved = await saveOrganigramConfig(merged);
      return NextResponse.json({ success: true, config: saved, added: fromRouting.length + fromPersonnel.length });
    }

    const config = parseOrganigramConfig(body?.config ?? body);
    const saved = await saveOrganigramConfig(config);
    return NextResponse.json({ success: true, config: saved });
  } catch (e) {
    console.error("[organigramme/config] PUT", e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
