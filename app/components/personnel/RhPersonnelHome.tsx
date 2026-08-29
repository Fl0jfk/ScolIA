"use client";

import Link from "next/link";
import dynamic from "next/dynamic";
import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import RhMoodPulseCard from "@/app/components/personnel/RhMoodPulseCard";
import RhPersonnelOnboarding from "@/app/components/personnel/RhPersonnelOnboarding";
import RhSelfDepositPanel from "@/app/components/personnel/RhSelfDepositPanel";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import { canCreateHseDemand, canCreateHseOnBehalf } from "@/app/lib/demandes-hse-access";
import { canAccessRhStaffRequest } from "@/app/lib/rh/rh-hub-access";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { formatAbsencePeriod } from "@/app/lib/absence-period";
import type { PersonnelRecord } from "@/app/lib/personnel-types";
import type { RhEspacePhase } from "@/app/lib/rh/rh-space-status";

const AbsencesPageClient = dynamic(() => import("@/app/(admin)/absences/AbsencesPageClient"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const DemandesHsePanel = dynamic(() => import("@/app/components/demandes-hse/DemandesHsePanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const RhDemandePanel = dynamic(() => import("@/app/components/personnel/RhDemandePanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const RhPlanningPanel = dynamic(() => import("@/app/components/personnel/RhPlanningPanel"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});

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

type MyRhRequest = {
  id: string;
  status: string;
  subject: string;
  createdAt?: string;
};

type EspaceData = {
  phase: RhEspacePhase;
  record: PersonnelRecord | null;
  identityComplete: boolean;
  submittedAt: string | null;
  validationNote: string | null;
  metaSummary: {
    birthDate: string | null;
    birthPlace: string | null;
    displayName: string;
  } | null;
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

function requestStatusLabel(s: string) {
  if (s === "TERMINEE") return { label: "Terminée", className: "bg-emerald-50 text-emerald-800" };
  if (s === "EN_COURS" || s === "EN_ATTENTE") return { label: "En cours", className: "bg-amber-50 text-amber-800" };
  return { label: s, className: "bg-slate-100 text-slate-600" };
}

export default function RhPersonnelHome({
  dashboardSection,
}: {
  dashboardSection?: string | null;
}) {
  const router = useRouter();
  const { user, isLoaded } = useSessionUser();
  const roles = useMemo(() => rolesFromUserLike(user), [user]);

  const canCreateHse = canCreateHseDemand(roles) || canCreateHseOnBehalf(roles);
  const canAccessDemandeRh = canAccessRhStaffRequest(roles);

  const [espace, setEspace] = useState<EspaceData | null>(null);
  const [espaceLoading, setEspaceLoading] = useState(true);
  const [absences, setAbsences] = useState<MyAbsence[]>([]);
  const [hseItems, setHseItems] = useState<MyHse[]>([]);
  const [rhRequests, setRhRequests] = useState<MyRhRequest[]>([]);
  const [loading, setLoading] = useState(true);

  const loadEspace = useCallback(async () => {
    setEspaceLoading(true);
    try {
      const res = await fetch("/api/rh/espace", { cache: "no-store" });
      const j = await res.json();
      if (res.ok) setEspace(j as EspaceData);
    } catch {
      setEspace(null);
    } finally {
      setEspaceLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!isLoaded) return;
    void loadEspace();
  }, [isLoaded, loadEspace]);

  useEffect(() => {
    if (!isLoaded || !user || espace?.phase !== "active") return;
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const [absRes, hseRes, reqRes] = await Promise.all([
          fetch("/api/absences", { cache: "no-store" }),
          canCreateHse ? fetch("/api/demandes-hse", { cache: "no-store" }) : Promise.resolve(null),
          fetch("/api/requests/list?scope=submitted", { cache: "no-store" }),
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
          setHseItems(Array.isArray(j.items) ? (j.items as MyHse[]).slice(0, 6) : []);
        }

        if (reqRes.ok) {
          const list = await reqRes.json();
          const rh = (Array.isArray(list) ? list : [])
            .filter((r: { category?: string; subject?: string }) => r.category === "RH" || r.subject?.startsWith("[RH]"))
            .slice(0, 6)
            .map((r: { id: string; status: string; subject: string; createdAt?: string }) => ({
              id: r.id,
              status: r.status,
              subject: r.subject,
              createdAt: r.createdAt,
            }));
          setRhRequests(rh);
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
  }, [isLoaded, user, canCreateHse, espace?.phase]);

  const firstName = user?.firstName || "vous";
  const phase = espace?.phase ?? "onboarding";
  const record = espace?.record ?? null;
  const birthDate = record?.profile?.birthDate ?? espace?.metaSummary?.birthDate ?? null;
  const birthPlace = record?.profile?.birthPlace ?? espace?.metaSummary?.birthPlace ?? null;

  if (dashboardSection === "absences") {
    return (
      <Suspense fallback={<ModuleTabFallback />}>
        <AbsencesPageClient embeddedInRh />
      </Suspense>
    );
  }
  if (dashboardSection === "hse" && canCreateHse) {
    return (
      <Suspense fallback={<ModuleTabFallback />}>
        <DemandesHsePanel embeddedInRh />
      </Suspense>
    );
  }
  if (dashboardSection === "demande" && canAccessDemandeRh) {
    return (
      <Suspense fallback={<ModuleTabFallback />}>
        <RhDemandePanel />
      </Suspense>
    );
  }
  if (dashboardSection === "planning") {
    return (
      <Suspense fallback={<ModuleTabFallback />}>
        <RhPlanningPanel />
      </Suspense>
    );
  }

  if (espaceLoading) {
    return <p className="text-sm text-slate-500 py-10 text-center">Chargement de votre espace…</p>;
  }

  if (phase === "onboarding") {
    return (
      <RhPersonnelOnboarding
        record={record}
        identityComplete={espace?.identityComplete ?? false}
        onRefresh={loadEspace}
      />
    );
  }

  if (phase === "pending_validation") {
    return (
      <div className="rounded-2xl border border-amber-200 bg-amber-50/60 p-8 text-center space-y-3 max-w-xl mx-auto">
        <p className="text-[11px] font-black uppercase tracking-widest text-amber-800">En attente</p>
        <h2 className="text-xl font-black text-slate-900">Dossier transmis à la RH</h2>
        <p className="text-sm text-slate-600">
          Votre espace personnel sera débloqué dès validation par la RH ou l&apos;administratif de votre
          établissement.
        </p>
        {espace?.submittedAt && (
          <p className="text-xs text-slate-500">
            Envoyé le {new Date(espace.submittedAt).toLocaleString("fr-FR")}
          </p>
        )}
        {espace?.validationNote && (
          <p className="text-sm text-rose-800 bg-white rounded-xl px-3 py-2 border border-rose-100">
            {espace.validationNote}
          </p>
        )}
      </div>
    );
  }

  const now = Date.now();
  const upcoming = absences
    .filter((a) => new Date(a.data.endAt).getTime() >= now)
    .sort((a, b) => +new Date(a.data.startAt) - +new Date(b.data.startAt))
    .slice(0, 5);
  const recent = absences
    .filter((a) => new Date(a.data.endAt).getTime() < now)
    .sort((a, b) => +new Date(b.data.startAt) - +new Date(a.data.startAt))
    .slice(0, 4);

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 via-white to-indigo-50/40 p-5 sm:p-6">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="min-w-0 flex-1">
            <p className="text-[11px] font-black uppercase tracking-widest text-indigo-600">Mon dossier RH</p>
            <h2 className="text-2xl font-black text-slate-900 mt-1">Bonjour {firstName}</h2>
            {(birthDate || birthPlace) && (
              <p className="text-sm text-slate-600 mt-1">
                {birthDate && `Né(e) le ${new Date(birthDate).toLocaleDateString("fr-FR")}`}
                {birthPlace ? ` · ${birthPlace}` : ""}
              </p>
            )}
            <p className="text-sm text-slate-500 mt-2 max-w-2xl">
              Votre espace personnel : absences, HSE, demandes RH et coffre documents.
            </p>
          </div>
          <div className="w-full shrink-0 lg:w-52 xl:w-56">
            <RhMoodPulseCard />
          </div>
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <Link
            href="/rh?tab=dashboard&section=absences#nouvelle-absence"
            className="px-4 py-2.5 rounded-xl bg-indigo-600 text-white text-xs font-bold shadow-sm hover:bg-indigo-700"
          >
            Demander une autorisation d&apos;absence
          </Link>
          {canCreateHse && (
            <Link
              href="/rh?tab=dashboard&section=hse"
              className="px-4 py-2.5 rounded-xl bg-white border border-indigo-200 text-indigo-800 text-xs font-bold hover:bg-indigo-50"
            >
              Faire une demande HSE
            </Link>
          )}
          {canAccessDemandeRh && (
            <Link
              href="/rh?tab=dashboard&section=demande"
              className="px-4 py-2.5 rounded-xl bg-white border border-violet-200 text-violet-800 text-xs font-bold hover:bg-violet-50"
            >
              Demande RH
            </Link>
          )}
          <Link
            href="/mon-planning"
            className="px-4 py-2.5 rounded-xl bg-white border border-slate-200 text-slate-700 text-xs font-bold hover:bg-slate-50"
          >
            Mon planning
          </Link>
          <Link
            href="/rh/moi"
            className="px-4 py-2.5 rounded-xl bg-white border border-slate-200 text-slate-700 text-xs font-bold hover:bg-slate-50"
          >
            Voir mon dossier complet →
          </Link>
        </div>
      </section>

      <div className="grid lg:grid-cols-2 xl:grid-cols-3 gap-4">
        <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
          <div className="flex items-center justify-between gap-2 mb-3">
            <h3 className="font-black text-slate-900">Mes absences</h3>
            <button
              type="button"
              onClick={() => router.push("/rh?tab=dashboard&section=absences")}
              className="text-[11px] font-bold text-indigo-600 underline"
            >
              Voir tout
            </button>
          </div>
          {loading ? (
            <p className="text-sm text-slate-400">Chargement…</p>
          ) : upcoming.length === 0 && recent.length === 0 ? (
            <p className="text-sm text-slate-400 italic">Aucune demande pour le moment.</p>
          ) : (
            <ul className="space-y-2">
              {[...upcoming, ...recent].slice(0, 5).map((a) => {
                const st = statusAbsence(a);
                return (
                  <li key={a.id} className="rounded-xl border border-slate-100 px-3 py-2">
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <p className="text-sm font-bold text-slate-900">{a.data.reason || "Absence"}</p>
                        <p className="text-xs text-slate-500 mt-0.5">
                          {formatAbsencePeriod({
                            periodType: a.data.periodType === "single_day" ? "single_day" : "multi_day",
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
          )}
        </section>

        {canCreateHse ? (
          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <div className="flex items-center justify-between gap-2 mb-3">
              <h3 className="font-black text-slate-900">Mes demandes HSE</h3>
              <button
                type="button"
                onClick={() => router.push("/rh?tab=dashboard&section=hse")}
                className="text-[11px] font-bold text-indigo-600 underline"
              >
                Ouvrir
              </button>
            </div>
            {loading ? (
              <p className="text-sm text-slate-400">Chargement…</p>
            ) : hseItems.length === 0 ? (
              <Link
                href="/rh?tab=dashboard&section=hse"
                className="inline-flex px-3 py-2 rounded-xl bg-indigo-50 text-indigo-800 text-xs font-bold border border-indigo-100"
              >
                Faire une demande HSE
              </Link>
            ) : (
              <ul className="space-y-2">
                {hseItems.map((h) => {
                  const st = statusHse(h.status);
                  return (
                    <li key={h.id} className="rounded-xl border border-slate-100 px-3 py-2">
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
        ) : null}

        <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
          <div className="flex items-center justify-between gap-2 mb-3">
            <h3 className="font-black text-slate-900">Mes demandes RH</h3>
            {canAccessDemandeRh && (
              <button
                type="button"
                onClick={() => router.push("/rh?tab=dashboard&section=demande")}
                className="text-[11px] font-bold text-indigo-600 underline"
              >
                Nouvelle
              </button>
            )}
          </div>
          {loading ? (
            <p className="text-sm text-slate-400">Chargement…</p>
          ) : rhRequests.length === 0 ? (
            <p className="text-sm text-slate-400 italic">Aucune demande RH récente.</p>
          ) : (
            <ul className="space-y-2">
              {rhRequests.map((r) => {
                const st = requestStatusLabel(r.status);
                return (
                  <li key={r.id} className="rounded-xl border border-slate-100 px-3 py-2">
                    <div className="flex items-start justify-between gap-2">
                      <p className="text-sm font-bold text-slate-900 truncate">{r.subject}</p>
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
      </div>

      <RhSelfDepositPanel />
    </div>
  );
}
