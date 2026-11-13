"use client";

import Link from "@/app/components/ui/AppLink";
import type { InternatBuilding, InternatIncident, InternatRoom, InternatStudent } from "@/app/lib/internat-types";
import {
  buildRoomInsights,
  formatOccupantLine,
  ROOM_STATUS_LABELS,
  ROOM_STATUS_STYLES,
  studentInternLink,
  wingLabel,
  type RoomInsight,
} from "@/app/lib/internat-room-insights";
import { roomLocationLabel, sortInternatFloors, usableInternatFloors } from "@/app/lib/internat-types";

function BedSlots({ insight }: { insight: RoomInsight }) {
  const slots = Array.from({ length: insight.room.capacity }, (_, i) => insight.occupants[i] || null);
  return (
    <div className="flex flex-wrap gap-2 mt-3">
      {slots.map((student, idx) => (
        <div
          key={student?.id || `empty-${idx}`}
          className={`min-w-[7.5rem] flex-1 rounded-xl border px-2.5 py-2 text-xs ${
            student
              ? student.underWatch
                ? "border-amber-300 bg-amber-50"
                : "border-indigo-200 bg-white"
              : "border-dashed border-slate-200 bg-slate-50/80 text-slate-400"
          }`}
        >
          {student ? (
            <>
              <Link
                href={studentInternLink(student.id)}
                className="font-bold text-indigo-700 hover:underline leading-tight block"
              >
                {student.eleveRef.prenom} {student.eleveRef.nom}
              </Link>
              <p className="text-[10px] text-slate-500 mt-0.5">
                {student.classe} · {student.etablissement}
              </p>
              {student.underWatch && (
                <p className="text-[10px] text-amber-800 font-semibold mt-1">Sous surveillance</p>
              )}
            </>
          ) : (
            <span className="italic">Place libre</span>
          )}
        </div>
      ))}
    </div>
  );
}

function RoomPlanCard({ insight, compact }: { insight: RoomInsight; compact?: boolean }) {
  const styles = ROOM_STATUS_STYLES[insight.status];
  const problemBadges: string[] = [];
  if (insight.status === "over") problemBadges.push("Surbooking");
  else if (insight.status === "full") problemBadges.push("Pleine");
  if (insight.hasWatch) problemBadges.push("Surveillance");
  if (insight.sanctions30d > 0) problemBadges.push(`${insight.sanctions30d} sanction(s)`);
  if (insight.incidents30d > 0) problemBadges.push(`${insight.incidents30d} incident(s)`);

  return (
    <article
      className={`rounded-2xl border p-4 transition-shadow hover:shadow-md ${styles.card} ${
        insight.isProblematic ? "ring-2 ring-offset-1 ring-amber-400/60" : ""
      }`}
    >
      <div className="flex items-start justify-between gap-2">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <h4 className="font-black text-slate-900">{insight.room.label}</h4>
            <span className={`text-[10px] font-bold uppercase px-2 py-0.5 rounded-full ${styles.badge}`}>
              {ROOM_STATUS_LABELS[insight.status]}
            </span>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            {insight.room.capacity} place{insight.room.capacity > 1 ? "s" : ""} · {insight.occupants.length}{" "}
            occupant(s)
            {insight.room.wing ? ` · ${wingLabel(insight.room.wing)}` : ""}
          </p>
        </div>
        <span className={`h-2.5 w-2.5 rounded-full shrink-0 mt-1 ${styles.dot}`} title={insight.status} />
      </div>

      {problemBadges.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mt-2">
          {problemBadges.map((b) => (
            <span
              key={b}
              className="text-[10px] font-semibold px-2 py-0.5 rounded-lg bg-white/80 border border-slate-200 text-slate-700"
            >
              {b}
            </span>
          ))}
        </div>
      )}

      {!compact && <BedSlots insight={insight} />}

      {compact && insight.occupants.length > 0 && (
        <ul className="mt-2 space-y-1 text-xs text-slate-700">
          {insight.occupants.map((s) => (
            <li key={s.id}>
              <Link href={studentInternLink(s.id)} className="text-indigo-700 font-semibold hover:underline">
                {formatOccupantLine(s)}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </article>
  );
}

export default function InternatRoomsPlanView({
  rooms,
  buildings,
  students,
  incidents = [],
  compact = false,
  filterProblematicOnly = false,
  showUnassigned = true,
}: {
  rooms: InternatRoom[];
  buildings: InternatBuilding[];
  students: InternatStudent[];
  incidents?: InternatIncident[];
  compact?: boolean;
  filterProblematicOnly?: boolean;
  showUnassigned?: boolean;
}) {
  const insights = buildRoomInsights(rooms, students, incidents);
  const insightByRoomId = new Map(insights.map((i) => [i.room.id, i]));
  const filtered = filterProblematicOnly ? insights.filter((i) => i.isProblematic) : insights;

  const grouped = buildings.flatMap((building) => {
    const floors = sortInternatFloors(building.floors);
    return floors.map((floor) => ({
      key: `${building.id}-${floor.id}`,
      title: `${building.label} · ${floor.label}`,
      inUse: floor.inUse,
      rooms: rooms
        .filter((r) => r.buildingId === building.id && r.floorId === floor.id)
        .map((r) => insightByRoomId.get(r.id)!)
        .filter(Boolean),
    }));
  });

  const unassigned = rooms
    .filter((r) => !r.buildingId || !r.floorId)
    .map((r) => insightByRoomId.get(r.id)!)
    .filter(Boolean);

  if (filterProblematicOnly && filtered.length === 0) {
    return (
      <p className="text-sm text-emerald-700 bg-emerald-50 border border-emerald-100 rounded-xl px-4 py-3">
        Aucune chambre problématique détectée — tout semble en ordre.
      </p>
    );
  }

  if (!filterProblematicOnly && rooms.length === 0) {
    return <p className="text-sm text-slate-500">Aucune chambre configurée.</p>;
  }

  return (
    <div className="space-y-8">
      {!filterProblematicOnly &&
        grouped.map((section) =>
          section.rooms.length === 0 ? null : (
            <section key={section.key} className="space-y-3">
              <div className="flex flex-wrap items-baseline justify-between gap-2">
                <h3 className="font-black text-slate-900">{section.title}</h3>
                <p className="text-xs text-slate-500">
                  {section.rooms.length} chambre(s)
                  {!section.inUse ? " · étage inactif" : ""}
                </p>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                {section.rooms.map((insight) => (
                  <RoomPlanCard key={insight.room.id} insight={insight} compact={compact} />
                ))}
              </div>
            </section>
          ),
        )}

      {filterProblematicOnly && (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          {filtered.map((insight) => (
            <RoomPlanCard key={insight.room.id} insight={insight} compact={compact} />
          ))}
        </div>
      )}

      {showUnassigned && unassigned.length > 0 && !filterProblematicOnly && (
        <section className="space-y-3">
          <h3 className="font-black text-slate-900">Chambres non classées</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {unassigned.map((insight) => (
              <div key={insight.room.id}>
                <p className="text-[10px] text-slate-400 mb-1">{roomLocationLabel(buildings, insight.room)}</p>
                <RoomPlanCard insight={insight} compact={compact} />
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
