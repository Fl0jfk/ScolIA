"use client";

import Link from "next/link";
import type { InternatDashboardStats } from "@/app/lib/internat-stats";
import { studentInternLink } from "@/app/lib/internat-room-insights";

function StatCard({
  label,
  value,
  hint,
  accent,
}: {
  label: string;
  value: string;
  hint?: string;
  accent?: string;
}) {
  return (
    <div className={`rounded-2xl border p-5 ${accent || "bg-white border-slate-200"}`}>
      <p className="text-[10px] font-bold uppercase tracking-wide text-slate-500">{label}</p>
      <p className="text-2xl font-black text-slate-900 mt-1">{value}</p>
      {hint && <p className="text-xs text-slate-500 mt-1">{hint}</p>}
    </div>
  );
}

function FillBar({ rate, label }: { rate: number | null; label: string }) {
  const pct = rate ?? 0;
  return (
    <div>
      <div className="flex justify-between text-xs mb-1">
        <span className="font-semibold text-slate-700">{label}</span>
        <span className="text-slate-500">{rate != null ? `${rate} %` : "—"}</span>
      </div>
      <div className="h-2.5 rounded-full bg-slate-100 overflow-hidden">
        <div
          className={`h-full rounded-full transition-all ${
            pct >= 95 ? "bg-amber-500" : pct >= 75 ? "bg-indigo-500" : "bg-emerald-500"
          }`}
          style={{ width: `${Math.min(100, pct)}%` }}
        />
      </div>
    </div>
  );
}

export default function InternatDashboardPanel({ stats }: { stats: InternatDashboardStats | null }) {
  if (!stats) {
    return <p className="text-slate-500 text-sm">Chargement des indicateurs…</p>;
  }

  const tonightLabel =
    stats.tonightRollCall.status === "non_applicable"
      ? "Pas ce soir"
      : stats.tonightRollCall.status === "validee"
        ? "Validé"
        : stats.tonightRollCall.status === "en_cours"
          ? "En cours"
          : "Non démarré";

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          label="Internes actifs"
          value={String(stats.activeStudents)}
          hint={`${stats.students.boys} garçon(s) · ${stats.students.girls} fille(s)`}
          accent="bg-indigo-50 border-indigo-100"
        />
        <StatCard
          label="Par établissement"
          value={`${stats.students.college} / ${stats.students.lycee}`}
          hint="Collège · Lycée"
        />
        <StatCard
          label="Taux de remplissage"
          value={stats.occupancy.fillRate != null ? `${stats.occupancy.fillRate} %` : "—"}
          hint={`${stats.occupancy.occupiedBeds} / ${stats.occupancy.totalBeds} place(s)`}
          accent="bg-emerald-50 border-emerald-100"
        />
        <StatCard
          label="Appel ce soir"
          value={tonightLabel}
          hint={
            stats.tonightRollCall.status === "non_applicable"
              ? "Uniquement lundi → jeudi"
              : undefined
          }
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <section className="lg:col-span-2 bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="font-black text-slate-900">Hébergement</h3>
            <Link
              href="/gestion-internat?tab=chambres&view=plan"
              className="text-xs font-bold text-indigo-600 hover:underline"
            >
              Voir le plan des chambres →
            </Link>
          </div>
          <FillBar rate={stats.occupancy.fillRate} label="Taux global des lits" />
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
            <div className="rounded-xl bg-slate-50 px-3 py-2">
              <p className="text-[10px] uppercase font-bold text-slate-500">Chambres</p>
              <p className="font-black text-slate-900">{stats.roomCount}</p>
            </div>
            <div className="rounded-xl bg-amber-50 px-3 py-2">
              <p className="text-[10px] uppercase font-bold text-amber-700">Pleines</p>
              <p className="font-black text-amber-950">{stats.occupancy.roomsFull}</p>
            </div>
            <div className="rounded-xl bg-sky-50 px-3 py-2">
              <p className="text-[10px] uppercase font-bold text-sky-700">Partielles</p>
              <p className="font-black text-sky-950">{stats.occupancy.roomsPartial}</p>
            </div>
            <div className="rounded-xl bg-slate-50 px-3 py-2">
              <p className="text-[10px] uppercase font-bold text-slate-500">Vides</p>
              <p className="font-black text-slate-900">{stats.occupancy.roomsEmpty}</p>
            </div>
          </div>
          {stats.students.withoutRoom > 0 && (
            <p className="text-sm text-amber-800 bg-amber-50 border border-amber-100 rounded-xl px-3 py-2">
              {stats.students.withoutRoom} interne(s) actif(s) sans chambre affectée.
            </p>
          )}
          {stats.roomsOverCapacity.length > 0 ? (
            <div className="text-sm text-red-800 bg-red-50 border border-red-100 rounded-xl px-3 py-2">
              <p className="font-bold">Surbooking ({stats.roomsOverCapacity.length})</p>
              <ul className="mt-1 space-y-0.5 text-xs">
                {stats.roomsOverCapacity.map((r) => (
                  <li key={r.roomId}>
                    {r.label} : {r.count}/{r.capacity}
                  </li>
                ))}
              </ul>
            </div>
          ) : (
            <p className="text-sm text-emerald-700">Aucune chambre en surbooking.</p>
          )}
          {stats.problematicRoomCount > 0 && (
            <Link
              href="/gestion-internat?tab=chambres&view=plan&filter=problem"
              className="inline-flex text-sm font-bold text-amber-800 hover:underline"
            >
              {stats.problematicRoomCount} chambre(s) à surveiller (sanctions, incidents, surveillance) →
            </Link>
          )}
        </section>

        <section className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
          <h3 className="font-black text-slate-900">Par aile</h3>
          {stats.wingOccupancy.length === 0 ? (
            <p className="text-sm text-slate-500">Aucune chambre avec genre renseigné.</p>
          ) : (
            stats.wingOccupancy.map((w) => (
              <FillBar
                key={w.wing}
                rate={w.fillRate}
                label={`${w.label} (${w.occupied}/${w.beds} lits)`}
              />
            ))
          )}
        </section>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard label="Incidents 30 j" value={String(stats.incidents30d.total)} />
        <StatCard label="Sanctions 30 j" value={String(stats.incidents30d.sanction)} />
        <StatCard label="Valorisation 30 j" value={String(stats.incidents30d.valorisation)} />
        <StatCard label="Sous surveillance" value={String(stats.studentsUnderWatch.length)} />
      </div>

      <div className="flex justify-end">
        <a
          href="/api/internat/stats/export"
          className="text-sm font-bold text-indigo-600 bg-white border border-indigo-200 px-4 py-2 rounded-xl hover:bg-indigo-50"
        >
          Exporter le rapport direction
        </a>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <section className="bg-white rounded-2xl border border-slate-200 p-5">
          <h3 className="font-black text-slate-900 mb-3">Ce soir</h3>
          <ul className="text-sm text-slate-600 space-y-1">
            <li>Présents : {stats.tonightRollCall.presentCount}</li>
            <li>Activité ext. : {stats.tonightRollCall.activityCount}</li>
            <li>Absents : {stats.tonightRollCall.absentCount}</li>
            <li>Excusés : {stats.tonightRollCall.excusedCount}</li>
            <li>Section garçons : {stats.tonightRollCall.boysComplete ? "complète" : "en attente"}</li>
            <li>Section filles : {stats.tonightRollCall.girlsComplete ? "complète" : "en attente"}</li>
          </ul>
        </section>

        <section className="bg-white rounded-2xl border border-slate-200 p-5">
          <h3 className="font-black text-slate-900 mb-3">Disponibilités</h3>
          <ul className="text-sm text-slate-600 space-y-1">
            <li>{stats.roomsWithFreeSlots} chambre(s) avec au moins une place libre</li>
            <li>{stats.occupancy.roomsFull} chambre(s) pleine(s)</li>
            <li>
              Taux présence 7 j :{" "}
              {stats.presenceRate7d != null ? `${stats.presenceRate7d} %` : "—"}
            </li>
          </ul>
        </section>
      </div>

      {stats.lastValidatedRollCall && (
        <p className="text-xs text-slate-500">
          Dernier appel validé : {stats.lastValidatedRollCall.date}
          {stats.lastValidatedRollCall.validatedBy ? ` par ${stats.lastValidatedRollCall.validatedBy}` : ""}
        </p>
      )}

      {stats.studentsUnderWatch.length > 0 && (
        <section className="bg-amber-50 border border-amber-100 rounded-2xl p-5">
          <h3 className="font-black text-amber-900 mb-3">Élèves sous surveillance</h3>
          <ul className="text-sm text-amber-950 space-y-2">
            {stats.studentsUnderWatch.map((s) => (
              <li key={s.id}>
                <Link href={studentInternLink(s.id)} className="font-semibold hover:underline">
                  {s.name}
                </Link>
                <span className="text-amber-800/80"> — {s.classe}</span>
                {s.note && <p className="text-xs mt-0.5">{s.note}</p>}
              </li>
            ))}
          </ul>
        </section>
      )}

      {stats.weeklySummary && (
        <section className="bg-indigo-50 border border-indigo-100 rounded-2xl p-5">
          <h3 className="font-black text-indigo-900 mb-2">Synthèse hebdomadaire</h3>
          <pre className="text-sm text-indigo-900/90 whitespace-pre-wrap font-sans">{stats.weeklySummary}</pre>
        </section>
      )}
    </div>
  );
}
