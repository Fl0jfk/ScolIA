import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { searchElevesRegistry } from "@/app/lib/eleves-registry";
import { loadEnseignantsRegistry } from "@/app/lib/enseignants-registry";
import { listDirectoryMembers } from "@/app/lib/directory-members";

export async function GET(req: Request) {
  const gate = await requireModule("accueil-portes-ouvertes");
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const kind = url.searchParams.get("kind") || "eleve";
  const q = (url.searchParams.get("q") || "").trim();
  if (q.length < 2) {
    return NextResponse.json({ results: [] });
  }

  if (kind === "eleve") {
    const eleves = await searchElevesRegistry(q, 20);
    return NextResponse.json({
      results: eleves.map((e) => ({
        refId: e.ine || e.id || `${e.nom}-${e.prenom}`,
        displayName: `${e.prenom} ${e.nom}`.trim(),
        meta: { classe: e.classe || "", ine: e.ine || "" },
      })),
    });
  }

  if (kind === "enseignant") {
    const registry = await loadEnseignantsRegistry();
    const needle = q.toLowerCase();
    const results = registry
      .filter((e) => {
        const hay = `${e.prenom || ""} ${e.nom || ""} ${e.email || ""}`.toLowerCase();
        return hay.includes(needle);
      })
      .slice(0, 20)
      .map((e) => ({
        refId: String(e.id || e.email || `${e.nom}-${e.prenom}`),
        displayName: `${e.prenom || ""} ${e.nom || ""}`.trim(),
        meta: { secteur: e.secteur || "", email: e.email || "" },
      }));
    return NextResponse.json({ results });
  }

  const members = await listDirectoryMembers();
  const needle = q.toLowerCase();
  const results = members
    .filter((m) => {
      const label = m.displayName || `${m.firstName || ""} ${m.lastName || ""}`.trim() || m.email;
      const hay = `${label} ${m.email || ""}`.toLowerCase();
      return hay.includes(needle);
    })
    .slice(0, 20)
    .map((m) => ({
      refId: m.userId || m.externalUserId,
      displayName:
        m.displayName || `${m.firstName || ""} ${m.lastName || ""}`.trim() || m.email || m.externalUserId,
      meta: { email: m.email || "" },
    }));
  return NextResponse.json({ results });
}
