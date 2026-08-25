import { NextResponse } from "next/server";
import { requireMobileStaffAccess } from "@/app/lib/mobile-auth";
import {
  jourSemaineFromIsoDate,
  listEdtCreneauxForJour,
} from "@/app/lib/vs-calendrier-db";
import { listAppelsForDate, listAppelsManquants } from "@/app/lib/vs-absences-db";
import { parisDateKey } from "@/app/lib/paris-time";

/** Journée staff-lite : créneaux + appels (sous-ensemble VS). */
export async function GET(req: Request) {
  const gate = await requireMobileStaffAccess();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const date =
    url.searchParams.get("date")?.trim() || parisDateKey(new Date());
  const jour = jourSemaineFromIsoDate(date);
  const etabId = gate.ctx.etablissementId;

  const [creneaux, appels, manquants] = await Promise.all([
    listEdtCreneauxForJour(etabId, jour),
    listAppelsForDate(etabId, date),
    listAppelsManquants(etabId, { dateAppel: date }),
  ]);

  // Filtre soft sur le nom enseignant si présent dans les créneaux.
  const nameFold = gate.ctx.name
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  const mine = creneaux.filter((c) => {
    const ens = String(c.enseignantNom || "")
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase();
    if (!ens || !nameFold) return true;
    return ens.includes(nameFold.split(/\s+/)[0] || "") || nameFold.includes(ens.split(/\s+/)[0] || "");
  });

  return NextResponse.json({
    channel: "mobile",
    date,
    jourSemaine: jour,
    creneaux: mine.length ? mine : creneaux,
    filteredToSelf: mine.length > 0 && mine.length < creneaux.length,
    appels,
    manquants,
  });
}
