"use client";

import Link from "next/link";
import type { PersonnelDashboardData, DashboardPersonnelItem } from "@/app/lib/personnel-dashboard";

const URGENCY_STYLES = {
  high: "border-rose-200 bg-rose-50/80",
  medium: "border-amber-200 bg-amber-50/80",
  low: "border-slate-200 bg-slate-50/80",
};

const URGENCY_DOT = {
  high: "bg-rose-500",
  medium: "bg-amber-500",
  low: "bg-slate-400",
};

const URGENCY_ORDER = { high: 0, medium: 1, low: 2 };

function mergeByPerson(...lists: DashboardPersonnelItem[][]): DashboardPersonnelItem[] {
  const map = new Map<string, DashboardPersonnelItem>();
  for (const item of lists.flat()) {
    const key = item.link;
    const prev = map.get(key);
    if (!prev || URGENCY_ORDER[item.urgency] < URGENCY_ORDER[prev.urgency]) {
      map.set(key, item);
    }
  }
  return [...map.values()].sort((a, b) => {
    const ua = URGENCY_ORDER[a.urgency] - URGENCY_ORDER[b.urgency];
    if (ua !== 0) return ua;
    if (a.dueDate && b.dueDate) return new Date(a.dueDate).getTime() - new Date(b.dueDate).getTime();
    return a.displayName.localeCompare(b.displayName, "fr");
  });
}

function SectionCard({
  title,
  subtitle,
  icon,
  items,
  empty,
  accentClass,
}: {
  title: string;
  subtitle?: string;
  icon: string;
  items: DashboardPersonnelItem[];
  empty: string;
  accentClass: string;
}) {
  return (
    <div className="bg-white rounded-2xl border border-slate-100 shadow-sm overflow-hidden flex flex-col min-h-[200px]">
      <div className={`px-5 py-4 border-b border-slate-100 ${accentClass}`}>
        <div className="flex items-center gap-2">
          <span className="text-lg">{icon}</span>
          <div className="min-w-0 flex-1">
            <h3 className="font-black text-slate-800 text-sm uppercase tracking-wide">{title}</h3>
            {subtitle && <p className="text-xs text-slate-500 mt-0.5">{subtitle}</p>}
          </div>
          <span className="text-xs font-bold bg-white/80 px-2 py-0.5 rounded-full text-slate-600 shrink-0">
            {items.length}
          </span>
        </div>
      </div>
      <div className="flex-1 p-3 space-y-2 max-h-[280px] overflow-y-auto">
        {items.length === 0 ? (
          <p className="text-xs text-slate-400 italic p-3 text-center">{empty}</p>
        ) : (
          items.slice(0, 8).map((item) => (
            <Link
              key={item.id}
              href={item.link}
              className={`block rounded-xl border p-3 transition hover:shadow-md ${URGENCY_STYLES[item.urgency]}`}
            >
              <div className="flex items-start gap-2">
                <span className={`w-2 h-2 rounded-full mt-1.5 shrink-0 ${URGENCY_DOT[item.urgency]}`} />
                <div className="min-w-0">
                  <p className="font-bold text-slate-800 text-sm truncate">{item.displayName}</p>
                  <p className="text-xs text-slate-600 mt-0.5">{item.detail}</p>
                </div>
              </div>
            </Link>
          ))
        )}
      </div>
    </div>
  );
}

export default function PersonnelDashboard({
  data,
  onNewStaff,
}: {
  data: PersonnelDashboardData;
  onNewStaff: () => void;
}) {
  const medecineItems = mergeByPerson(data.medecineYear, data.medecine);
  const entretienItems = mergeByPerson(data.entretiensYear, data.entretiens);

  const followUp =
    data.counts.onboardings +
    data.counts.signatures +
    data.counts.habilitations +
    data.counts.formations;

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-3xl bg-gradient-to-br from-indigo-700 via-violet-700 to-fuchsia-700 p-6 sm:p-8 shadow-xl">
        <div className="absolute inset-0 opacity-20 bg-[radial-gradient(circle_at_20%_20%,white,transparent_40%)]" />
        <div className="relative z-10">
          <div className="flex flex-col sm:flex-row sm:items-end justify-between gap-4 mb-6">
            <div>
              <p className="text-indigo-200 text-xs font-black uppercase tracking-widest">Module RH</p>
              <h1 className="text-2xl sm:text-3xl font-black text-white mt-1">Tableau de bord RH</h1>
              <p className="text-white/75 text-sm mt-2">
                {data.staffTotal} collaborateur{data.staffTotal > 1 ? "s" : ""} actif{data.staffTotal > 1 ? "s" : ""}
                {followUp > 0 && ` · ${followUp} suivi${followUp > 1 ? "s" : ""} administratif${followUp > 1 ? "s" : ""}`}
              </p>
              <p className="text-white/60 text-xs mt-2">
                {data.year} — {data.counts.medecineDueThisYear} visite{data.counts.medecineDueThisYear > 1 ? "s" : ""} médicale{data.counts.medecineDueThisYear > 1 ? "s" : ""} à prévoir
                {data.counts.medecineOverdue > 0 && ` (${data.counts.medecineOverdue} en retard)`}
                {" · "}
                {data.counts.entretiensToPosition} entretien{data.counts.entretiensToPosition > 1 ? "s" : ""} à positionner
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={onNewStaff}
                className="bg-white text-indigo-700 px-5 py-2.5 rounded-xl text-sm font-black shadow-lg hover:bg-indigo-50 transition"
              >
                + Nouvelle entrée
              </button>
              <Link
                href="/rh/moi"
                className="bg-white/15 text-white border border-white/30 px-5 py-2.5 rounded-xl text-sm font-bold hover:bg-white/25 transition"
              >
                Mon dossier
              </Link>
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <div className="rounded-2xl border border-white/20 bg-white/10 backdrop-blur-sm p-4 text-white">
              <p className="text-[10px] font-black uppercase tracking-widest text-white/70">Entrées</p>
              <p className="text-3xl font-black mt-1 text-amber-200">{data.counts.onboardings}</p>
            </div>
            <div className="rounded-2xl border border-white/20 bg-white/10 backdrop-blur-sm p-4 text-white">
              <p className="text-[10px] font-black uppercase tracking-widest text-white/70">Signatures</p>
              <p className="text-3xl font-black mt-1 text-rose-200">{data.counts.signatures}</p>
            </div>
            <div className="rounded-2xl border border-white/20 bg-white/10 backdrop-blur-sm p-4 text-white">
              <p className="text-[10px] font-black uppercase tracking-widest text-white/70">Habilitations</p>
              <p className="text-3xl font-black mt-1 text-orange-200">{data.counts.habilitations}</p>
            </div>
            <div className="rounded-2xl border border-white/20 bg-white/10 backdrop-blur-sm p-4 text-white">
              <p className="text-[10px] font-black uppercase tracking-widest text-white/70">Formations</p>
              <p className="text-3xl font-black mt-1 text-sky-200">{data.counts.formations}</p>
            </div>
          </div>
        </div>
      </div>

      {data.absencesToday.length > 0 && (
        <div className="rounded-2xl border border-rose-100 bg-gradient-to-r from-rose-50 to-orange-50 p-5 shadow-sm">
          <div className="flex items-center gap-2 mb-3">
            <span className="text-xl">🩺</span>
            <h2 className="font-black text-rose-800">Absents aujourd&apos;hui</h2>
            <Link href="/rh?tab=absences" className="ml-auto text-xs font-bold text-rose-600 underline">
              Voir absences
            </Link>
          </div>
          <div className="flex flex-wrap gap-2">
            {data.absencesToday.map((a) => (
              <div key={a.id} className="bg-white rounded-xl border border-rose-100 px-4 py-2 shadow-sm">
                <p className="font-bold text-slate-800 text-sm">{a.displayName}</p>
                <p className="text-xs text-slate-500">{a.reason} · {a.periodLabel}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="grid md:grid-cols-2 gap-4">
        <SectionCard
          title="Médecine du travail"
          subtitle={`Échéances ${data.year} et visites à court terme`}
          icon="🏥"
          items={medecineItems}
          empty="Rien à prévoir pour le moment"
          accentClass="bg-emerald-50"
        />
        <SectionCard
          title="Entretiens professionnels"
          subtitle={`Cycles ${data.year} et entretiens en cours`}
          icon="💬"
          items={entretienItems}
          empty="Tout est à jour"
          accentClass="bg-violet-50"
        />
        <SectionCard
          title="Habilitations"
          subtitle="Échéances à 90 j — renouvellement à prévoir"
          icon="🦺"
          items={data.habilitations}
          empty="Aucune habilitation à renouveler prochainement"
          accentClass="bg-orange-50"
        />
        <SectionCard
          title="Formations"
          subtitle="Demandes et sessions planifiées à venir"
          icon="📚"
          items={data.formations}
          empty="Aucune formation planifiée"
          accentClass="bg-sky-50"
        />
        <SectionCard
          title="Entrées en cours"
          subtitle="Onboarding et intégration"
          icon="🚀"
          items={data.onboardings}
          empty="Aucune entrée en cours"
          accentClass="bg-indigo-50"
        />
        <SectionCard
          title="Signatures en attente"
          subtitle="Documents d'entrée à faire signer"
          icon="✍️"
          items={data.signatures}
          empty="Aucune signature en attente"
          accentClass="bg-rose-50"
        />
      </div>
    </div>
  );
}
