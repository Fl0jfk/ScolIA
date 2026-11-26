import { NextResponse } from "next/server";
import { requireInternatAccess } from "@/app/api/internat/_auth";
import { parisStamp, vueSortie } from "@/app/lib/sorties-classe";
import { listerSorties } from "@/app/lib/sorties-classe-store";

/** Consultation seule pour l'équipe internat : sorties de classe d'aujourd'hui et à venir. */
export async function GET() {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;
  const aujourdhui = parisStamp(new Date()).slice(0, 10);
  const sorties = (await listerSorties())
    .filter((s) => s.date >= aujourdhui)
    .reverse()
    .map((s) => vueSortie(s, null));
  return NextResponse.json({ sorties, aujourdhui });
}
