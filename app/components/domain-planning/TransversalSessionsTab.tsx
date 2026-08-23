"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useAppContext } from "@/app/hooks/useAppContext";
import {
  ASSOCIATION_LOCKED_IDEA,
  ASSOCIATION_LOCKED_SUBJECT,
  canUserSignupOnSession,
  classesForTransversalNiveau,
  DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS,
  lockedSubjectForSession,
  sanitizeDomainPlanningClassesByPole,
  signupNeedsCoordinatorReview,
  signupRequiresSessionIdea,
  signupValidationStatus,
  TRANSVERSAL_NIVEAU_LABELS,
  TRANSVERSAL_NIVEAUX,
} from "@/app/lib/domain-planning-defaults";
import type { DomainPlanningSession, DomainPlanningSignup } from "@/app/lib/domain-planning-types";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { getSubjectColorPresentation } from "@/app/lib/prof-room-subject-colors";

const FALLBACK_CLASSES: Record<string, string[]> = {
  COLLÈGE: ["6A", "6B", "6C", "6D", "6E", "6F", "5A", "5B", "5C", "5D", "5E", "5F", "4A", "4B", "4C", "4D", "4E", "4F", "3A", "3B", "3C", "3D", "3E", "3F"],
};

type ReviewAction = "validate" | "changes_requested" | "reject";

type Props = {
  /** Responsable EVARS désignée dans le paramétrage (pas les admins org). */
  isCoordinator: boolean;
};

function constraintHint(session: DomainPlanningSession): string {
  switch (session.intervenantConstraint) {
    case "fixed_association":
      return "Géré par l'association — pas d'inscription sur l'intranet";
    case "svt_only":
      return "Inscription réservée aux professeurs de SVT — matière verrouillée, idée de séance à proposer";
    case "psy_inf":
      return "Inscription réservée aux psychologues et infirmières — matière verrouillée, idée de séance à proposer";
    default:
      return "Inscription libre : matière et idée de séance à valider par la responsable EVARS";
  }
}

function displayName(signup: DomainPlanningSignup): string {
  const name = [signup.firstName, signup.lastName].filter(Boolean).join(" ").trim();
  return name || signup.email || "—";
}

function ValidationStatusBadge({
  signup,
  session,
}: {
  signup: DomainPlanningSignup;
  session?: DomainPlanningSession | null;
}) {
  const status = signupValidationStatus(signup, session);
  if (status === "validated") {
    return (
      <span className="inline-block mt-1 text-[10px] font-black uppercase text-emerald-700">
        Validé
      </span>
    );
  }
  if (status === "pending") {
    return (
      <span className="inline-block mt-1 text-[10px] font-black uppercase text-amber-700">
        En attente de validation
      </span>
    );
  }
  if (status === "changes_requested") {
    return (
      <span className="inline-block mt-1 text-[10px] font-black uppercase text-orange-700">
        Modifications demandées
      </span>
    );
  }
  return (
    <span className="inline-block mt-1 text-[10px] font-black uppercase text-rose-700">
      Refusé
    </span>
  );
}

export default function TransversalSessionsTab({ isCoordinator }: Props) {
  const { user } = useSessionUser();
  const { data: appCtx } = useAppContext();
  const [sessions, setSessions] = useState<DomainPlanningSession[]>([]);
  const [signups, setSignups] = useState<DomainPlanningSignup[]>([]);
  const [evarsCoordinatorIds, setEvarsCoordinatorIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [modal, setModal] = useState<{ session: DomainPlanningSession; className: string } | null>(null);
  const [reviewModal, setReviewModal] = useState<{
    signup: DomainPlanningSignup;
    action: "changes_requested" | "reject";
  } | null>(null);
  const [reviewComment, setReviewComment] = useState("");
  const [editIdeaModal, setEditIdeaModal] = useState<DomainPlanningSignup | null>(null);
  const [editIdeaText, setEditIdeaText] = useState("");
  const [subject, setSubject] = useState("");
  const [sessionIdea, setSessionIdea] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const roles = useMemo(() => {
    const fromContext = appCtx?.session?.intranetRoles;
    if (Array.isArray(fromContext) && fromContext.length > 0) return fromContext;
    return intranetRolesFromMetadata(user?.publicMetadata);
  }, [appCtx?.session?.intranetRoles, user?.publicMetadata]);

  const sessionDisplayName = useMemo(() => {
    const first = user?.firstName?.trim() || "";
    const last = user?.lastName?.trim() || "";
    return [first, last].filter(Boolean).join(" ") || user?.primaryEmailAddress?.emailAddress || "";
  }, [user?.firstName, user?.lastName, user?.primaryEmailAddress?.emailAddress]);

  const classesByPole = useMemo(() => {
    const raw = appCtx?.domainPlanning?.classesByPole || FALLBACK_CLASSES;
    return sanitizeDomainPlanningClassesByPole(raw);
  }, [appCtx?.domainPlanning?.classesByPole]);

  const activityColors = {
    ...DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS,
    ...(appCtx?.domainPlanning?.activityColors || {}),
  };

  const reload = useCallback(async () => {
    const [sessionsRes, signupsRes, domainsRes] = await Promise.all([
      fetch("/api/domain-planning/sessions", { cache: "no-store" }),
      fetch("/api/domain-planning/signups", { cache: "no-store" }),
      fetch("/api/domain-planning/domains", { cache: "no-store" }),
    ]);
    if (!sessionsRes.ok) {
      const errText = await sessionsRes.text().catch(() => "");
      throw new Error(errText || `Erreur chargement séances (${sessionsRes.status})`);
    }
    if (!signupsRes.ok) {
      const errText = await signupsRes.text().catch(() => "");
      throw new Error(errText || `Erreur chargement positionnements (${signupsRes.status})`);
    }
    const sessionsJson = await sessionsRes.json();
    const signupsJson = await signupsRes.json();
    setSessions(sessionsJson.sessions || []);
    setSignups(signupsJson.signups || []);
    if (domainsRes.ok) {
      const domainsJson = await domainsRes.json();
      const evars = (domainsJson.domains || []).find((d: { id: string }) => d.id === "evars");
      setEvarsCoordinatorIds(evars?.coordinatorExternalUserIds || []);
    }
  }, []);

  useEffect(() => {
    reload()
      .catch((e: unknown) => {
        console.error(e);
        alert(e instanceof Error ? e.message : "Erreur de chargement EVARS");
      })
      .finally(() => setLoading(false));
  }, [reload]);

  const mySignups = useMemo(
    () => signups.filter((s) => s.userId === user?.id),
    [signups, user?.id],
  );

  const pendingForCoordinator = useMemo(() => {
    return signups
      .map((signup) => ({
        signup,
        session: sessions.find((s) => s.id === signup.sessionId) || null,
      }))
      .filter(({ signup, session }) => signupNeedsCoordinatorReview(signup, session));
  }, [signups, sessions]);

  function signupFor(sessionId: string, className: string) {
    return signups.find((s) => s.sessionId === sessionId && s.className === className);
  }

  function signupsForSession(sessionId: string) {
    return signups.filter((s) => s.sessionId === sessionId);
  }

  function openSignup(session: DomainPlanningSession, className: string) {
    setSubject(lockedSubjectForSession(session) || "");
    setSessionIdea("");
    setModal({ session, className });
  }

  async function confirmSignup() {
    if (!modal) return;
    if (signupRequiresSessionIdea(modal.session) && !sessionIdea.trim()) {
      alert("Merci de décrire votre idée de séance.");
      return;
    }
    setSubmitting(true);
    try {
      const res = await fetch("/api/domain-planning/signups/create", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          sessionId: modal.session.id,
          className: modal.className,
          subject,
          sessionIdea,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Erreur");
      setModal(null);
      await reload();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSubmitting(false);
    }
  }

  async function removeSignup(id: string) {
    if (!confirm("Retirer ce positionnement ?")) return;
    const res = await fetch("/api/domain-planning/signups/delete", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id }),
    });
    const data = await res.json();
    if (!res.ok) {
      alert(data.error || "Erreur");
      return;
    }
    await reload();
  }

  async function updateMySessionIdea(id: string, idea: string) {
    const res = await fetch("/api/domain-planning/signups/update-idea", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id, sessionIdea: idea }),
    });
    const data = await res.json();
    if (!res.ok) {
      alert(data.error || "Erreur");
      return;
    }
    setEditIdeaModal(null);
    setEditIdeaText("");
    await reload();
  }

  async function reviewSignup(id: string, action: ReviewAction, comment?: string) {
    const res = await fetch("/api/domain-planning/signups/validate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id, action, comment: comment || "" }),
    });
    const data = await res.json();
    if (!res.ok) {
      alert(data.error || "Erreur");
      return;
    }
    setReviewModal(null);
    setReviewComment("");
    await reload();
  }

  function coordinatorLabel() {
    if (evarsCoordinatorIds.length === 0) {
      return "Aucune responsable EVARS désignée — voir l'onglet Paramétrage.";
    }
    if (evarsCoordinatorIds.length === 1 && evarsCoordinatorIds[0] === user?.id) {
      return "Vous êtes la responsable EVARS.";
    }
    return `${evarsCoordinatorIds.length} responsable(s) EVARS désignée(s) — voir Paramétrage pour le détail.`;
  }

  if (loading) {
    return <div className="p-8 text-center text-slate-500 font-bold">Chargement des séances EVARS…</div>;
  }

  return (
    <div className="space-y-8 px-4 pb-8">
      <div className="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-900 leading-relaxed space-y-1">
        <p>
          <strong>EVARS — positionnement des intervenants.</strong> Vous proposez votre idée de séance en vous
          positionnant ; la responsable EVARS valide, demande des modifications ou refuse.
        </p>
        <p className="text-xs text-rose-800">{coordinatorLabel()}</p>
      </div>

      {isCoordinator && pendingForCoordinator.length > 0 && (
        <section className="rounded-2xl border-2 border-violet-300 bg-violet-50 p-4 space-y-3">
          <h3 className="text-sm font-black text-violet-800 uppercase">
            Validations en attente ({pendingForCoordinator.length})
          </h3>
          <p className="text-xs text-violet-900">
            L&apos;intervenant a proposé son idée de séance : validez, demandez des modifications ou refusez.
          </p>
          <div className="space-y-3">
            {pendingForCoordinator.map(({ signup, session }) => (
              <div
                key={signup.id}
                className="rounded-xl border border-violet-200 bg-white p-3 text-sm space-y-3"
              >
                <div>
                  <p className="font-black text-slate-900">
                    {TRANSVERSAL_NIVEAU_LABELS[session?.niveau || ""] || "—"} — Séance {session?.seanceNumber} —{" "}
                    {signup.className}
                  </p>
                  <p className="font-bold text-slate-700">{displayName(signup)}</p>
                  <p className="text-slate-600">{signup.subject}</p>
                  {signup.sessionIdea ? (
                    <p className="text-slate-600 mt-2 italic">&laquo; {signup.sessionIdea} &raquo;</p>
                  ) : session && signupRequiresSessionIdea(session) ? (
                    <p className="text-amber-700 text-xs mt-2 font-bold">Idée de séance non renseignée par l&apos;intervenant</p>
                  ) : null}
                  <ValidationStatusBadge signup={signup} session={session} />
                  {signup.validationComment && (
                    <p className="text-xs text-slate-500 mt-1">Votre dernier retour : {signup.validationComment}</p>
                  )}
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={() => void reviewSignup(signup.id, "validate")}
                    disabled={Boolean(session && signupRequiresSessionIdea(session) && !signup.sessionIdea?.trim())}
                    className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-black text-white hover:bg-emerald-700 disabled:opacity-50"
                  >
                    Valider
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setReviewComment(signup.validationComment || "");
                      setReviewModal({ signup, action: "changes_requested" });
                    }}
                    className="rounded-lg bg-orange-500 px-3 py-1.5 text-xs font-black text-white hover:bg-orange-600"
                  >
                    Demander modifications
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setReviewComment(signup.validationComment || "");
                      setReviewModal({ signup, action: "reject" });
                    }}
                    className="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-black text-white hover:bg-rose-700"
                  >
                    Refuser
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {mySignups.length > 0 && (
        <div className="rounded-2xl border border-violet-200 bg-white p-4">
          <h3 className="text-sm font-black text-violet-700 uppercase mb-3">Mes positionnements ({mySignups.length})</h3>
          <div className="flex flex-wrap gap-2">
            {mySignups.map((s) => {
              const session = sessions.find((x) => x.id === s.sessionId);
              return (
                <button
                  key={s.id}
                  type="button"
                  onClick={() => removeSignup(s.id)}
                  className="text-left rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs hover:border-rose-300"
                  title="Cliquer pour retirer"
                >
                  <span className="font-black text-slate-800">
                    {TRANSVERSAL_NIVEAU_LABELS[session?.niveau || ""] || ""} — Séance {session?.seanceNumber} — {s.className}
                  </span>
                  <span className="block text-slate-700 font-bold">{displayName(s)}</span>
                  <span className="block text-slate-500">{s.subject}</span>
                  <ValidationStatusBadge signup={s} session={session} />
                  {s.validationComment && (
                    <span className="block text-slate-500 mt-1">&laquo; {s.validationComment} &raquo;</span>
                  )}
                  {signupValidationStatus(s, session) === "changes_requested" && (
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        setEditIdeaText(s.sessionIdea || "");
                        setEditIdeaModal(s);
                      }}
                      className="block mt-1 text-[10px] font-black uppercase text-violet-700 hover:underline"
                    >
                      Modifier mon idée
                    </button>
                  )}
                </button>
              );
            })}
          </div>
        </div>
      )}

      {TRANSVERSAL_NIVEAUX.map((niveau) => {
        const niveauSessions = sessions
          .filter((s) => s.niveau === niveau)
          .sort((a, b) => a.seanceNumber - b.seanceNumber);
        const classes = classesForTransversalNiveau(niveau, classesByPole);

        return (
          <section key={niveau} className="space-y-4">
            <h2 className="text-2xl font-black text-slate-900">{TRANSVERSAL_NIVEAU_LABELS[niveau]}</h2>
            {niveauSessions.map((session) => {
              const colorKey = `Séance ${session.seanceNumber}`;
              const colorPresentation = getSubjectColorPresentation(
                activityColors[colorKey] || "bg-violet-600 text-white",
              );
              const canSignup = canUserSignupOnSession(session, roles, isCoordinator);
              const sessionSignups = signupsForSession(session.id);
              const positionedSummary =
                sessionSignups.length > 0
                  ? sessionSignups
                      .map((s) => `${s.className} : ${displayName(s)}`)
                      .join(" · ")
                  : null;

              return (
                <div key={session.id} className="rounded-3xl border border-slate-200 bg-white overflow-hidden shadow-sm">
                  <div
                    className={`px-4 py-3 ${colorPresentation.className}`}
                    style={colorPresentation.style}
                  >
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <div>
                        <p className="text-[10px] font-black uppercase tracking-widest opacity-80">
                          Séance {session.seanceNumber}
                        </p>
                        <p className="font-black text-lg leading-snug">{session.theme}</p>
                      </div>
                      <div className="text-right text-xs font-bold">
                        <p>{session.intervenantLabel}</p>
                        <p className="opacity-90">Mixte : {session.mixte ? "OUI" : "NON"}</p>
                      </div>
                    </div>
                    <p className="text-xs mt-2 opacity-90">{constraintHint(session)}</p>
                    {positionedSummary && (
                      <p className="text-xs mt-2 font-bold opacity-95">
                        Positionnés : {positionedSummary}
                      </p>
                    )}
                  </div>

                  <div className="p-4 overflow-x-auto">
                    <table className="w-full min-w-[32rem] text-sm">
                      <thead>
                        <tr className="text-left text-[10px] font-black uppercase tracking-widest text-slate-400">
                          <th className="pb-2 pr-3">Classe</th>
                          <th className="pb-2 pr-3">Intervenant</th>
                          <th className="pb-2 pr-3">Matière</th>
                          <th className="pb-2 pr-3">Idée de séance</th>
                          <th className="pb-2">Statut</th>
                        </tr>
                      </thead>
                      <tbody>
                        {classes.map((className) => {
                          const signup = signupFor(session.id, className);
                          const isMine = signup?.userId === user?.id;
                          const isAssociation = session.intervenantConstraint === "fixed_association";

                          return (
                            <tr key={className} className="border-t border-slate-100">
                              <td className="py-2 pr-3 font-black text-slate-800">{className}</td>
                              {isAssociation && !signup ? (
                                <>
                                  <td className="py-2 pr-3 text-slate-600 italic">{session.intervenantLabel}</td>
                                  <td className="py-2 pr-3 text-slate-600">{ASSOCIATION_LOCKED_SUBJECT}</td>
                                  <td className="py-2 pr-3 text-slate-600">{ASSOCIATION_LOCKED_IDEA}</td>
                                  <td className="py-2 text-slate-400">—</td>
                                </>
                              ) : signup ? (
                                <>
                                  <td className="py-2 pr-3 font-bold">
                                    {displayName(signup)}
                                    {(isMine || isCoordinator) && (
                                      <button
                                        type="button"
                                        onClick={() => removeSignup(signup.id)}
                                        className="ml-2 text-rose-600 text-[10px] font-black uppercase"
                                      >
                                        Retirer
                                      </button>
                                    )}
                                  </td>
                                  <td className="py-2 pr-3">{signup.subject}</td>
                                  <td className="py-2 pr-3 text-slate-600">{signup.sessionIdea || "—"}</td>
                                  <td className="py-2">
                                    <ValidationStatusBadge signup={signup} session={session} />
                                    {signup.validationComment && (
                                      <p className="text-[10px] text-slate-500 mt-1 max-w-[12rem]">
                                        {signup.validationComment}
                                      </p>
                                    )}
                                    {isCoordinator && signupNeedsCoordinatorReview(signup, session) && (
                                      <div className="flex flex-wrap gap-1 mt-2">
                                        <button
                                          type="button"
                                          onClick={() => void reviewSignup(signup.id, "validate")}
                                          disabled={Boolean(signupRequiresSessionIdea(session) && !signup.sessionIdea?.trim())}
                                          className="text-[10px] font-black uppercase text-emerald-700 hover:underline disabled:opacity-50"
                                        >
                                          Valider
                                        </button>
                                        <button
                                          type="button"
                                          onClick={() => {
                                            setReviewComment(signup.validationComment || "");
                                            setReviewModal({ signup, action: "changes_requested" });
                                          }}
                                          className="text-[10px] font-black uppercase text-orange-700 hover:underline"
                                        >
                                          Modifications
                                        </button>
                                        <button
                                          type="button"
                                          onClick={() => {
                                            setReviewComment(signup.validationComment || "");
                                            setReviewModal({ signup, action: "reject" });
                                          }}
                                          className="text-[10px] font-black uppercase text-rose-700 hover:underline"
                                        >
                                          Refuser
                                        </button>
                                      </div>
                                    )}
                                  </td>
                                </>
                              ) : canSignup ? (
                                <td colSpan={4} className="py-2">
                                  <button
                                    type="button"
                                    onClick={() => openSignup(session, className)}
                                    className="rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-black text-white hover:bg-violet-700"
                                  >
                                    Me positionner
                                  </button>
                                </td>
                              ) : (
                                <td colSpan={4} className="py-2 text-slate-400 italic">
                                  —
                                </td>
                              )}
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                </div>
              );
            })}
          </section>
        );
      })}

      {modal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl space-y-4">
            <h3 className="text-lg font-black text-slate-900">
              {TRANSVERSAL_NIVEAU_LABELS[modal.session.niveau]} — Séance {modal.session.seanceNumber} — {modal.className}
            </h3>
            <p className="text-sm text-slate-600">{modal.session.theme}</p>

            <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2">
              <span className="text-[10px] font-black uppercase text-slate-400">Vous</span>
              <p className="text-sm font-black text-slate-900">{sessionDisplayName}</p>
            </div>

            {lockedSubjectForSession(modal.session) ? (
              <div className="rounded-xl border border-violet-200 bg-violet-50 px-3 py-2">
                <span className="text-[10px] font-black uppercase text-violet-600">Matière (verrouillée)</span>
                <p className="text-sm font-bold text-violet-900">{lockedSubjectForSession(modal.session)}</p>
              </div>
            ) : (
              <label className="block">
                <span className="text-xs font-bold text-slate-500 uppercase">Matière *</span>
                <input
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-bold"
                  value={subject}
                  onChange={(e) => setSubject(e.target.value)}
                  placeholder="ex. EPS, Français…"
                />
              </label>
            )}

            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Idée de séance *</span>
              <textarea
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm min-h-[80px]"
                value={sessionIdea}
                onChange={(e) => setSessionIdea(e.target.value)}
                placeholder="Décrivez ce que vous proposez pour cette classe"
              />
              <p className="text-[10px] text-slate-500 mt-1">
                Votre idée sera examinée par la responsable EVARS (validation, modifications ou refus).
              </p>
            </label>

            <p className="text-xs text-amber-800 bg-amber-50 border border-amber-200 rounded-xl px-3 py-2 font-bold">
              Statut après envoi : en attente de validation
            </p>

            <div className="flex gap-2 pt-2">
              <button
                type="button"
                onClick={() => setModal(null)}
                className="flex-1 rounded-xl border border-slate-200 py-2 text-sm font-bold"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={submitting}
                onClick={() => void confirmSignup()}
                className="flex-1 rounded-xl bg-violet-600 py-2 text-sm font-black text-white disabled:opacity-50"
              >
                {submitting ? "Envoi…" : "Confirmer le positionnement"}
              </button>
            </div>
          </div>
        </div>
      )}

      {reviewModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl space-y-4">
            <h3 className="text-lg font-black text-slate-900">
              {reviewModal.action === "reject" ? "Refuser le positionnement" : "Demander des modifications"}
            </h3>
            <p className="text-sm text-slate-600">
              {displayName(reviewModal.signup)} — {reviewModal.signup.className}
            </p>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Commentaire pour l&apos;intervenant *</span>
              <textarea
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm min-h-[100px]"
                value={reviewComment}
                onChange={(e) => setReviewComment(e.target.value)}
                placeholder="Expliquez ce qui doit être modifié ou le motif du refus"
              />
            </label>
            <div className="flex gap-2 pt-2">
              <button
                type="button"
                onClick={() => {
                  setReviewModal(null);
                  setReviewComment("");
                }}
                className="flex-1 rounded-xl border border-slate-200 py-2 text-sm font-bold"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={submitting || !reviewComment.trim()}
                onClick={() =>
                  void reviewSignup(reviewModal.signup.id, reviewModal.action, reviewComment.trim()).finally(() =>
                    setSubmitting(false),
                  )
                }
                className="flex-1 rounded-xl bg-violet-600 py-2 text-sm font-black text-white disabled:opacity-50"
              >
                Confirmer
              </button>
            </div>
          </div>
        </div>
      )}

      {editIdeaModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl space-y-4">
            <h3 className="text-lg font-black text-slate-900">Modifier mon idée de séance</h3>
            {editIdeaModal.validationComment && (
              <p className="text-sm text-orange-800 bg-orange-50 border border-orange-200 rounded-xl px-3 py-2">
                Retour responsable EVARS : {editIdeaModal.validationComment}
              </p>
            )}
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Nouvelle idée de séance *</span>
              <textarea
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm min-h-[100px]"
                value={editIdeaText}
                onChange={(e) => setEditIdeaText(e.target.value)}
              />
            </label>
            <div className="flex gap-2 pt-2">
              <button
                type="button"
                onClick={() => {
                  setEditIdeaModal(null);
                  setEditIdeaText("");
                }}
                className="flex-1 rounded-xl border border-slate-200 py-2 text-sm font-bold"
              >
                Annuler
              </button>
              <button
                type="button"
                disabled={submitting || !editIdeaText.trim()}
                onClick={() => void updateMySessionIdea(editIdeaModal.id, editIdeaText.trim())}
                className="flex-1 rounded-xl bg-violet-600 py-2 text-sm font-black text-white disabled:opacity-50"
              >
                Renvoyer pour validation
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
