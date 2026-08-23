"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import RhMoodPulseCard from "@/app/components/personnel/RhMoodPulseCard";
import RhSelfDepositPanel from "@/app/components/personnel/RhSelfDepositPanel";
import { canAccessHseModule, canCreateHseDemand } from "@/app/lib/demandes-hse-access";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { formatAbsencePeriod } from "@/app/lib/absence-period";

type MyAbsence = {
  id: string;
  createdAt: string;
  data: {
    startAt: string;
    endAt: string;
    reason: string;
    periodType?: string | null;
    startDate?: string;
    endDate?: string;
    startTime?: string | null;
    endTime?: string | null;
  };
  managerDecision?: string;
  workflowStatus?: string;
};

type MyHse = {
  id: string;
  status: string;
  resumeDemande: string;
  createdAt: string;
  nombreHeures?: number;
};

function statusAbsence(a: MyAbsence) {
  if (a.managerDecision === "VALIDEE") return { label: "Validée", className: "bg-emerald-50 text-emerald-800" };
  if (a.managerDecision === "REFUSEE") return { label: "Refusée", className: "bg-rose-50 text-rose-800" };
  return { label: "En attente", className: "bg-amber-50 text-amber-800" };
}

function statusHse(s: string) {
  if (s === "ACCEPTEE") return { label: "Acceptée", className: "bg-emerald-50 text-emerald-800" };
  if (s === "REFUSEE") return { label: "Refusée", className: "bg-rose-50 text-rose-800" };
  if (s === "ANNULEE") return { label: "Annulée", className: "bg-slate-100 text-slate-600" };
  return { label: "En attente", className: "bg-amber-50 text-amber-800" };
}

export default function RhPersonnelHome({ canManage }: { canManage: boolean }) {
  const { user, isLoaded } = useSessionUser();
  const roles = useMemo(() => rolesFromUserLike(user), [user]);

  const showHse = canAccessHseModule(roles);
  const canCreateHse = canCreateHseDemand(roles);

  const [absences, setAbsences] = useState<MyAbsence[]>([]);
  const [hseItems, setHseItems] = useState<MyHse[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!isLoaded || !user) return;
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const [absRes, hseRes] = await Promise.all([
          fetch("/api/absences", { cache: "no-store" }),
          showHse ? fetch("/api/demandes-hse", { cache: "no-store" }) : Promise.resolve(null),
        ]);
        if (cancelled) return;

        if (absRes.ok) {
          const list = (await absRes.json()) as MyAbsence[];
          const mine = (Array.isArray(list) ? list : []).filter(
            (a) => (a as { createdBy?: { userId?: string } }).createdBy?.userId === user.id,
          );
          setAbsences(mine);
        }

        if (hseRes && hseRes.ok) {
          const j = await hseRes.json();
          const items = Array.isArray(j.items) ? (j.items as MyHse[]) : [];
          setHseItems(items.slice(0, 6));
        }
      } catch {
        /* ignore */
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [isLoaded, user, showHse]);

  const now = Date.now();
  const upcoming = absences
    .filter((a) => new Date(a.data.endAt).getTime() >= now)
    .sort((a, b) => +new Date(a.data.startAt) - +new Date(b.data.startAt))
    .slice(0, 5);
  const recent = absences
    .filter((a) => new Date(a.data.endAt).getTime() < now)
    .sort((a, b) => +new Date(b.data.startAt) - +new Date(a.data.startAt))
    .slice(0, 4);

  const firstName = user?.firstName || "vous";

  return (
    <div className="space-y-6">
      <RhMoodPulseCard />

      <section className="rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 via-white to-indigo-50/40 p-5 sm:p-6">
        <p className="text-[11px] font-black uppercase tracking-widest text-indigo-600">Espace personnel</p>
        <h2 className="text-2xl font-black text-slate-900 mt-1">Bonjour {firstName}</h2>
        <p className="text-sm text-slate-600 mt-1 max-w-2xl">
          Votre coin RH : déclarer une absence, suivre vos demandes et déposer un document.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          <Link
            href="/rh?tab=absences&view=se-declarer#nouvelle-absence"
            className="px-4 py-2.5 rounded-xl bg-indigo-600 text-white text-xs font-bold shadow-sm hover:bg-indigo-700"
          >
            Déclarer une absence
          </Link>
          {canCreateHse && (
            <Link
              href="/rh?tab=hse"
              className="px-4 py-2.5 rounded-xl bg-white border border-indigo-200 text-indigo-800 text-xs font-bold hover:bg-indigo-50"
            >
              Demande HSE
            </Link>
          )}
          <Link
            href="/rh?tab=demande"
            className="px-4 py-2.5 rounded-xl bg-white border border-violet-200 text-violet-800 text-xs font-bold hover:bg-violet-50"
          >
            Demande RH
          </Link>
          <Link
            href="/rh?tab=absences&view=se-declarer"
            className="px-4 py-2.5 rounded-xl bg-white border border-slate-200 text-slate-700 text-xs font-bold hover:bg-slate-50"
          >
            Mes absences
          </Link>
          {canManage && (
            <Link
              href="/rh?tab=registre"
              className="px-4 py-2.5 rounded-xl bg-white border border-slate-200 text-slate-500 text-xs font-bold hover:bg-slate-50"
            >
              Pilotage RH →
            </Link>
          )}
        </div>
      </section>

      <div className="grid lg:grid-cols-2 gap-4">
        <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
          <div className="flex items-center justify-between gap-2 mb-3">
            <h3 className="font-black text-slate-900">Mes absences</h3>
            <Link href="/rh?tab=absences&view=se-declarer" className="text-[11px] font-bold text-indigo-600 underline">
              Voir tout
            </Link>
          </div>
          {loading ? (
            <p className="text-sm text-slate-400">Chargement…</p>
          ) : upcoming.length === 0 && recent.length === 0 ? (
            <p className="text-sm text-slate-400 italic">Aucune absence déclarée pour le moment.</p>
          ) : (
            <div className="space-y-4">
              {upcoming.length > 0 && (
                <div>
                  <p className="text-[10px] font-black uppercase tracking-wide text-emerald-700 mb-2">
                    À venir / en cours
                  </p>
                  <ul className="space-y-2">
                    {upcoming.map((a) => {
                      const st = statusAbsence(a);
                      return (
                        <li key={a.id} className="rounded-xl border border-emerald-100 bg-emerald-50/50 px-3 py-2.5">
                          <div className="flex items-start justify-between gap-2">
                            <div>
                              <p className="text-sm font-bold text-slate-900">{a.data.reason || "Absence"}</p>
                              <p className="text-xs text-slate-600 mt-0.5">
                                {formatAbsencePeriod({
                                  periodType:
                                    a.data.periodType === "single_day" ? "single_day" : "multi_day",
                                  startDate: a.data.startDate || a.data.startAt.slice(0, 10),
                                  endDate: a.data.endDate || a.data.endAt.slice(0, 10),
                                  startTime: a.data.startTime ?? null,
                                  endTime: a.data.endTime ?? null,
                                })}
                              </p>
                            </div>
                            <span className={`shrink-0 text-[10px] font-black px-2 py-0.5 rounded-lg ${st.className}`}>
                              {st.label}
                            </span>
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              {recent.length > 0 && (
                <div>
                  <p className="text-[10px] font-black uppercase tracking-wide text-slate-500 mb-2">
                    Récentes
                  </p>
                  <ul className="space-y-2">
                    {recent.map((a) => {
                      const st = statusAbsence(a);
                      return (
                        <li key={a.id} className="rounded-xl border border-slate-100 px-3 py-2">
                          <div className="flex items-start justify-between gap-2">
                            <div>
                              <p className="text-sm font-bold text-slate-800">{a.data.reason || "Absence"}</p>
                              <p className="text-xs text-slate-500 mt-0.5">
                                {new Date(a.data.startAt).toLocaleDateString("fr-FR")}
                              </p>
                            </div>
                            <span className={`shrink-0 text-[10px] font-black px-2 py-0.5 rounded-lg ${st.className}`}>
                              {st.label}
                            </span>
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
            </div>
          )}
        </section>

        {showHse ? (
          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <div className="flex items-center justify-between gap-2 mb-3">
              <h3 className="font-black text-slate-900">
                {canCreateHse ? "Mes demandes HSE" : "HSE de mon établissement"}
              </h3>
              <Link href="/rh?tab=hse" className="text-[11px] font-bold text-indigo-600 underline">
                Ouvrir
              </Link>
            </div>
            {loading ? (
              <p className="text-sm text-slate-400">Chargement…</p>
            ) : hseItems.length === 0 ? (
              <div className="space-y-3">
                <p className="text-sm text-slate-400 italic">Aucune demande pour le moment.</p>
                {canCreateHse && (
                  <Link
                    href="/rh?tab=hse"
                    className="inline-flex px-3 py-2 rounded-xl bg-indigo-50 text-indigo-800 text-xs font-bold border border-indigo-100"
                  >
                    Faire une demande HSE
                  </Link>
                )}
              </div>
            ) : (
              <ul className="space-y-2">
                {hseItems.map((h) => {
                  const st = statusHse(h.status);
                  return (
                    <li key={h.id} className="rounded-xl border border-slate-100 px-3 py-2.5">
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-bold text-slate-900 truncate">{h.resumeDemande}</p>
                          <p className="text-xs text-slate-500 mt-0.5">
                            {new Date(h.createdAt).toLocaleDateString("fr-FR")}
                            {h.nombreHeures != null ? ` · ${h.nombreHeures} h` : ""}
                          </p>
                        </div>
                        <span className={`shrink-0 text-[10px] font-black px-2 py-0.5 rounded-lg ${st.className}`}>
                          {st.label}
                        </span>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
        ) : (
          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <h3 className="font-black text-slate-900 mb-2">Raccourcis</h3>
            <p className="text-sm text-slate-500 mb-3">
              Accédez rapidement à vos actions du quotidien.
            </p>
            <div className="flex flex-col gap-2">
              <Link
                href="/rh?tab=absences&view=se-declarer#nouvelle-absence"
                className="rounded-xl border border-slate-100 px-3 py-2.5 text-sm font-bold text-slate-800 hover:bg-slate-50"
              >
                Déclarer une absence →
              </Link>
              <Link
                href="/rh?tab=annuaire"
                className="rounded-xl border border-slate-100 px-3 py-2.5 text-sm font-bold text-slate-800 hover:bg-slate-50"
              >
                Annuaire →
              </Link>
            </div>
          </section>
        )}
      </div>

      <RhSelfDepositPanel />
    </div>
  );
}
