import * as XLSX from "xlsx";
import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { isWorkflowV2 } from "@/app/lib/travels-v2";
import { canRead, canSeeAmount, resolveAccess } from "@/app/lib/travels-v2-access";
import { toCsv } from "@/app/lib/travels-v2-lot3";
import { buildRegistrationRows, listInvitations, listStudentCases } from "@/app/lib/travels-v2-registration-store";

/** Export du suivi (CSV ou Excel). Mêmes droits de lecture que la vue ; le montant seulement compta / direction / admin. */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const url = new URL(req.url);
  const tripId = url.searchParams.get("tripId")?.trim() || "";
  const format = url.searchParams.get("format") === "xlsx" ? "xlsx" : "csv";
  if (!tripId) return NextResponse.json({ error: "Voyage manquant." }, { status: 400 });
  try {
    const trip = await getTravelTrip(tripId);
    if (!trip || !isWorkflowV2(trip)) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });
    const access = await resolveAccess(trip);
    if (!access || !canRead(access)) return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
    const withAmount = canSeeAmount(access);
    const rows = buildRegistrationRows({
      trip,
      invitations: await listInvitations(tripId),
      includeAmount: withAmount,
      cases: await listStudentCases(tripId),
      includeReason: false,
    });
    const header = [
      "Élève",
      "Classe",
      "Responsable (e-mail)",
      "Statut",
      "Liste officielle",
      "Réponse papier",
      "Pièces manquantes",
      "Signé le",
      "Signataire",
      ...(withAmount ? ["Montant (€)"] : []),
    ];
    const body: unknown[][] = rows.map((r) => [
      r.studentName,
      r.classe ?? "",
      r.email ?? "",
      r.statusLabel,
      r.official ? "oui" : "non",
      r.paper ? "papier" : "",
      r.documents.filter((d) => !d.uploaded && !d.exempt).map((d) => d.label).join(" | "),
      r.signedAt ? new Date(r.signedAt).toLocaleString("fr-FR", { timeZone: "Europe/Paris" }) : "",
      r.signerName ?? "",
      ...(withAmount ? [typeof r.amount === "number" ? r.amount : ""] : []),
    ]);
    const stamp = new Date().toISOString().slice(0, 10);
    const headers = { "Cache-Control": "no-store" };
    if (format === "xlsx") {
      // Protection contre l'injection de formules dans Excel.
      const safe = body.map((row) => row.map((c) => (typeof c === "string" && /^[=+\-@\t\r]/.test(c) ? `'${c}` : c)));
      const sheet = XLSX.utils.aoa_to_sheet([header, ...safe]);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, sheet, "Inscriptions");
      const buf = XLSX.write(wb, { type: "buffer", bookType: "xlsx" }) as Buffer;
      return new NextResponse(new Uint8Array(buf), {
        headers: {
          ...headers,
          "Content-Type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
          "Content-Disposition": `attachment; filename="inscriptions-${stamp}.xlsx"`,
        },
      });
    }
    return new NextResponse(toCsv(header, body), {
      headers: {
        ...headers,
        "Content-Type": "text/csv; charset=utf-8",
        "Content-Disposition": `attachment; filename="inscriptions-${stamp}.csv"`,
      },
    });
  } catch (e) {
    console.error("[travels/v2/invitations/export]", e);
    return NextResponse.json({ error: "Export impossible." }, { status: 500 });
  }
}
