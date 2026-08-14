"use client";

import Link from "next/link";
import type { TravelsDirectionDashboard } from "@/app/lib/travels-direction-dashboard";
import { TRAVELS_STATUS_LABELS } from "@/app/lib/travels-types";

function StatCard({
  label,
  value,
  accent,
  sub,
}: {
  label: string;
  value: number;
  accent: string;
  sub?: string;
}) {
  return (
    <div className="rounded-2xl border border-white/15 bg-white/10 backdrop-blur-sm p-4 text-white shadow-lg">
      <p className="text-[10px] font-black uppercase tracking-widest text-white/70">{label}</p>
      <p className={`text-3xl font-black mt-1 ${accent}`}>{value}</p>
      {sub && <p className="text-[10px] text-white/60 mt-1">{sub}</p>}
    </div>
  );
}

export default function TravelsDirectionDashboardPanel({ data }: { data: TravelsDirectionDashboard }) {
  return (
    <div className="relative overflow-hidden rounded-3xl bg-gradient-to-br from-slate-900 via-indigo-950 to-violet-900 p-6 sm:p-8 shadow-2xl mb-10">
      <div className="absolute inset-0 opacity-25 bg-[radial-gradient(circle_at_10%_20%,white,transparent_45%)]" />
      <div className="relative z-10 space-y-6">
        <div className="flex flex-col sm:flex-row sm:items-end justify-between gap-4">
          <div>
            <p className="text-indigo-200 text-xs font-black uppercase tracking-widest">Pilotage direction</p>
            <h2 className="text-2xl sm:text-3xl font-black text-white mt-1">
              Voyages — {data.etablissement}
            </h2>
            <p className="text-white/70 text-sm mt-2">
              Année scolaire {data.schoolYearLabel} · {data.counts.tripsActive} dossier
              {data.counts.tripsActive > 1 ? "s" : ""} actif{data.counts.tripsActive > 1 ? "s" : ""}
            </p>
          </div>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-7 gap-3">
          <StatCard
            label="Signatures"
            value={data.counts.signaturesPending}
            accent={data.counts.signaturesPending > 0 ? "text-rose-300" : "text-emerald-300"}
            sub="en attente pour vous"
          />
          <StatCard label="Cette année" value={data.counts.tripsYear} accent="text-sky-200" />
          <StatCard label="Validés" value={data.counts.validatedYear} accent="text-emerald-200" />
          <StatCard label="En cours" value={data.counts.tripsActive} accent="text-amber-200" />
          <StatCard label="Pédagogie" value={data.counts.pendingPedagogy} accent="text-violet-200" />
          <StatCard label="Devis bus" value={data.counts.busQuotesWaiting} accent="text-orange-200" />
          <StatCard label="À venir" value={data.counts.upcomingTrips} accent="text-white" />
        </div>

        <div className="grid md:grid-cols-2 gap-4">
          <div className="bg-white/10 rounded-2xl border border-white/10 p-4">
            <h3 className="text-xs font-black uppercase tracking-wide text-rose-200 mb-3">
              Signatures & validations en attente
            </h3>
            {data.signaturesPending.length === 0 ? (
              <p className="text-sm text-white/60 italic">Rien en attente de votre signature.</p>
            ) : (
              <ul className="space-y-2">
                {data.signaturesPending.map((s) => (
                  <li key={`${s.id}-${s.reason}`}>
                    <Link
                      href={`/travels/${s.id}`}
                      className="block rounded-xl bg-white/10 hover:bg-white/15 px-3 py-2 transition"
                    >
                      <p className="text-sm font-bold text-white truncate">{s.title}</p>
                      <p className="text-[10px] text-rose-100">{s.reason}</p>
                    </Link>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <div className="bg-white/10 rounded-2xl border border-white/10 p-4">
            <h3 className="text-xs font-black uppercase tracking-wide text-sky-200 mb-3">Prochaines sorties</h3>
            {data.upcoming.length === 0 ? (
              <p className="text-sm text-white/60 italic">Aucune sortie à venir.</p>
            ) : (
              <ul className="space-y-2">
                {data.upcoming.map((u) => (
                  <li key={u.id}>
                    <Link
                      href={`/travels/${u.id}`}
                      className="block rounded-xl bg-white/10 hover:bg-white/15 px-3 py-2 transition"
                    >
                      <p className="text-sm font-bold text-white truncate">{u.title}</p>
                      <p className="text-[10px] text-sky-100">
                        {u.dateLabel} · {TRAVELS_STATUS_LABELS[u.status] || u.status.replace("EN_ATTENTE_", "").replace(/_/g, " ")}
                      </p>
                    </Link>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
