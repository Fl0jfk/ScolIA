import { NextResponse } from "next/server";
import { and, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveFoyerLink, foyer } from "@/db/schema";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { loadAppConfig } from "@/app/lib/app-config";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { userIsAnyDirection } from "@/app/lib/establishment-catalog";
import { userHasComptaRole } from "@/app/lib/travels-rbac-server";
import { canReassignTravelsOwner } from "@/app/lib/travels-roles";
import { getJson } from "@/app/lib/s3-storage";
import { chunkArray } from "@/app/lib/db-in-chunks";
import {
  comptaApelSchoolYearBounds,
  formatTravelDateLabel,
  tripInApelSchoolYear,
} from "@/app/lib/travels-compta-apel-summary";
import { eleveParticipantKey } from "@/app/lib/travels-eleves-list";
import {
  buildCommuneRecap,
  type CommuneRecapInputTrip,
} from "@/app/lib/travels-commune-recap";
import type { TravelsTrip } from "@/app/lib/travels-types";

/**
 * Récapitulatif : pour chaque commune (ville du foyer de l'élève), les élèves partis
 * en sortie / voyage depuis le début de l'année scolaire et les voyages concernés.
 * Réservé à la direction, la comptabilité et l'administratif (données nominatives).
 */
export async function GET() {
  const user = await safeCurrentUser();
  if (!user) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });

  const bundle = await loadAppConfig();
  const roles = rolesFromUserLike(user);
  const allowed =
    userHasComptaRole(user) ||
    canReassignTravelsOwner(user) ||
    userIsAnyDirection(user, bundle.establishments, roles);
  if (!allowed) {
    return NextResponse.json(
      { error: "Réservé à la direction, la comptabilité ou l'administratif." },
      { status: 403 },
    );
  }

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant.response;
  const etabId = tenant.ctx.etablissementId;

  try {
    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    const loaded = await Promise.all(
      index.map(async (s) => (await getJson<TravelsTrip>(`travels/${s.id}.json`))?.data ?? null),
    );
    const year = comptaApelSchoolYearBounds();
    const trips = loaded.filter(
      (t): t is TravelsTrip =>
        t != null &&
        t.status !== "ANNULE" &&
        t.status !== "SEANCE_ANNULEE" &&
        Array.isArray(t.data?.participantEleves) &&
        (t.data.participantEleves?.length ?? 0) > 0 &&
        tripInApelSchoolYear(t, year.start, year.end),
    );

    // eleveId direct, sinon résolution par INE.
    const eleveIds = new Set<string>();
    const ines = new Set<string>();
    for (const t of trips) {
      for (const p of t.data.participantEleves ?? []) {
        if (p.eleveId) eleveIds.add(p.eleveId);
        else if (p.ine?.trim()) ines.add(p.ine.trim());
      }
    }
    const db = getDb();
    const idByIne = new Map<string, string>();
    for (const chunk of chunkArray([...ines])) {
      const rows = await db
        .select({ id: eleve.id, ine: eleve.ine })
        .from(eleve)
        .where(and(eq(eleve.etablissementId, etabId), inArray(eleve.ine, chunk)));
      for (const r of rows) {
        if (r.ine) {
          idByIne.set(r.ine.trim(), r.id);
          eleveIds.add(r.id);
        }
      }
    }
    // Foyer « principal » en priorité ; à défaut, le premier foyer lié ayant une ville.
    const villeByEleve = new Map<string, string>();
    for (const chunk of chunkArray([...eleveIds])) {
      const rows = await db
        .select({ eleveId: eleveFoyerLink.eleveId, relation: eleveFoyerLink.relation, ville: foyer.ville })
        .from(eleveFoyerLink)
        .innerJoin(foyer, eq(foyer.id, eleveFoyerLink.foyerId))
        .where(
          and(eq(eleveFoyerLink.etablissementId, etabId), inArray(eleveFoyerLink.eleveId, chunk)),
        );
      const sorted = [...rows].sort(
        (a, b) => Number(b.relation === "principal") - Number(a.relation === "principal"),
      );
      for (const r of sorted) {
        const v = r.ville?.trim();
        if (v && !villeByEleve.has(r.eleveId)) villeByEleve.set(r.eleveId, v);
      }
    }

    const input: CommuneRecapInputTrip[] = trips
      .map((t) => ({
        tripId: t.id,
        title: String(t.data?.title || t.data?.destination || "Sortie sans titre"),
        dateLabel: formatTravelDateLabel(t),
        sortDate: String(t.data?.startDate || t.data?.date || ""),
        participants: (t.data.participantEleves ?? []).map((p) => {
          const id = p.eleveId || (p.ine ? idByIne.get(p.ine.trim()) : undefined);
          return {
            key: eleveParticipantKey(p),
            nom: p.nom,
            prenom: p.prenom,
            classe: p.classe,
            commune: id ? villeByEleve.get(id) ?? null : null,
          };
        }),
      }))
      .sort((a, b) => a.sortDate.localeCompare(b.sortDate))
      .map(({ sortDate: _s, ...rest }) => rest);

    return NextResponse.json(buildCommuneRecap(input, year.label));
  } catch (e) {
    console.error("[travels/recap-communes]", e);
    return NextResponse.json({ error: "Impossible de charger le récapitulatif." }, { status: 500 });
  }
}
