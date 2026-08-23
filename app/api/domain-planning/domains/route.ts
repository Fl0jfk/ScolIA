import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { requireDomainPlanningSettingsAdmin } from "@/app/lib/domain-planning-auth";
import { loadDomains, saveDomains } from "@/app/lib/domain-planning-storage";
import type { DomainPlanningDomain } from "@/app/lib/domain-planning-types";

function parseDomainsBody(body: unknown): DomainPlanningDomain[] {
  const raw = Array.isArray(body) ? body : (body as { domains?: unknown[] })?.domains;
  if (!Array.isArray(raw)) throw new Error("Format invalide : attendu { domains: [...] }");
  const out: DomainPlanningDomain[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const o = item as Record<string, unknown>;
    const id = typeof o.id === "string" ? o.id.trim() : "";
    const name = typeof o.name === "string" ? o.name.trim() : "";
    if (!id || !name) continue;
    out.push({
      id,
      name,
      description: typeof o.description === "string" ? o.description.trim() : undefined,
      color: typeof o.color === "string" ? o.color.trim() : undefined,
      coordinatorExternalUserIds: Array.isArray(o.coordinatorExternalUserIds)
        ? o.coordinatorExternalUserIds.map((x) => String(x).trim()).filter(Boolean)
        : [],
    });
  }
  if (out.length === 0) throw new Error("Au moins un domaine est requis.");
  return out;
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const domains = await loadDomains();
    return NextResponse.json({ domains });
  } catch (err: unknown) {
    console.error("GET /domain-planning/domains:", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Erreur serveur" },
      { status: 500 },
    );
  }
}

export async function PUT(req: Request) {
  const gate = await requireDomainPlanningSettingsAdmin();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const domains = parseDomainsBody(body);
    await saveDomains(domains);
    return NextResponse.json({ success: true, domains });
  } catch (err: unknown) {
    console.error("PUT /domain-planning/domains:", err);
    const msg = err instanceof Error ? err.message : "Impossible d'enregistrer les domaines.";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
