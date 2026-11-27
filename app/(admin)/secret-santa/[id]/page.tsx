"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "@/app/components/ui/AppLink";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import SantaForm, { type SantaFormValues } from "@/app/components/secret-santa/SantaForm";
import SantaInfoForm from "@/app/components/secret-santa/SantaInfoForm";
import SantaReveal from "@/app/components/secret-santa/SantaReveal";
import type { SantaResponse, SantaView } from "@/app/lib/secret-santa";

const longDate = (iso: string) =>
  new Date(iso).toLocaleString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  });

const RESPONSE_LABELS: Record<SantaResponse, string> = {
  pending: "⏳ sans réponse",
  accepted: "✅ participe",
  declined: "✖️ ne participe pas",
  withdrawn: "↩️ retiré",
};

const BLOCKER_TEXT = {
  not_enough: "Moins de 3 personnes ont accepté : le tirage n'a pas eu lieu.",
  too_many_exclusions:
    "Avec les personnes qui ont accepté, les paires interdites sont trop nombreuses pour garder le secret : le tirage n'a pas eu lieu.",
  confidential_rules:
    "Le tirage ne peut pas avoir lieu avec les personnes qui ont accepté. La personne qui organise doit contacter l'administrateur.",
};

type Loaded = { view: SantaView; offsetMs: number };

export default function SecretSantaDetailPage() {
  const { id } = useParams<{ id: string }>();
  const router = useRouter();
  const [data, setData] = useState<Loaded | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);

  /** Une seule lecture à la fois : au moment du tirage, tout le monde interroge le serveur. */
  const loading = useRef(false);

  const load = useCallback(async () => {
    if (loading.current) return;
    loading.current = true;
    try {
      const sentAt = Date.now();
      const res = await fetch(`/api/secret-santa/${id}`, { cache: "no-store" });
      const view = await res.json();
      if (!res.ok) throw new Error(view?.error || "Chargement impossible.");
      // Milieu de l'aller-retour : le décalage reste juste même sur une connexion lente.
      const offsetMs = new Date(view.serverNow).getTime() - (sentAt + Date.now()) / 2;
      setData({ view, offsetMs });
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible.");
    } finally {
      loading.current = false;
    }
  }, [id]);

  useEffect(() => {
    void load();
  }, [load]);

  // Avant le tirage, on suit les changements de l'organisateur (date, participants).
  const pending = Boolean(data && !data.view.drawn);
  useEffect(() => {
    if (!pending) return;
    const timer = window.setInterval(() => void load(), 60_000);
    return () => window.clearInterval(timer);
  }, [pending, load]);

  const markOpened = useCallback(
    (seen: string) => {
      void fetch(`/api/secret-santa/${id}/ouvert`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ seen }),
      })
        .then((res) => res.json())
        .then((body) => (body?.stale ? load() : undefined))
        .catch(() => undefined);
    },
    [id, load],
  );

  // De retour sur l'onglet, on relit : un nom a pu changer après un désistement.
  useEffect(() => {
    const onVisible = () => document.visibilityState === "visible" && void load();
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [load]);

  async function call(path: string, init: RequestInit) {
    const res = await fetch(`/api/secret-santa/${id}${path}`, {
      ...init,
      headers: { "Content-Type": "application/json" },
    });
    const body = await res.json();
    if (!res.ok) throw new Error(body?.error || "Action impossible.");
    return body;
  }

  async function act(run: () => Promise<void>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await run();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Action impossible.");
    } finally {
      setBusy(false);
    }
  }

  const respond = (response: "accepted" | "declined") =>
    void act(async () => {
      await call("/reponse", { method: "POST", body: JSON.stringify({ response }) });
      await load();
    });

  const withdraw = (userId: string, name: string) => {
    const warning =
      `Retirer ${name} du tirage ?\n\n` +
      "Une ou plusieurs personnes recevront un nouveau nom et un e-mail les invitant à rouvrir leur cadeau " +
      "(peut-être après un achat). Elles pourront deviner qui s'est retiré. C'est définitif.";
    if (!window.confirm(warning)) return;
    void act(async () => {
      await call("/retrait", { method: "POST", body: JSON.stringify({ userId }) });
      setNotice(
        `${name} est retiré du tirage. Les personnes concernées sont prévenues par e-mail. En cas de doute, « Prévenir d'un changement » invite tout le monde à vérifier son nom.`,
      );
      await load();
    });
  };

  const invite = (mode: "new" | "reminder" | "update", question: string) => {
    if (!window.confirm(question)) return;
    void act(async () => {
      const result = await call("/invitations", { method: "POST", body: JSON.stringify({ mode }) });
      setNotice(
        result.sent === 0 && result.failed === 0
          ? "Aucun e-mail à envoyer : personne n'est concerné, ou un e-mail est parti il y a moins de 10 minutes."
          : `${result.sent} e-mail(s) envoyé(s)${result.failed ? `, ${result.failed} en échec (à renvoyer)` : ""}.`,
      );
      await load();
    });
  };

  const remove = () => {
    if (!window.confirm("Supprimer ce Secret Santa ? Le tirage sera perdu pour tout le monde.")) return;
    void act(async () => {
      await call("", { method: "DELETE" });
      router.push("/secret-santa");
    });
  };

  const view = data?.view;
  const organizer = view?.organizer ?? null;
  const invitees = organizer?.invitees ?? [];
  const open = Boolean(view && !view.drawn && !view.blocker);
  const notInvited = invitees.filter((s) => !s.invited && s.response !== "withdrawn").length;
  const toRemind = invitees.filter((s) =>
    view?.drawn ? s.response === "accepted" && !s.opened : s.response === "pending",
  ).length;
  const toUpdate = invitees.filter(
    (s) => s.invited && (s.response === "accepted" || (open && s.response === "pending")),
  ).length;
  const count = (response: SantaResponse) => invitees.filter((s) => s.response === response).length;
  // Le formulaire se ferme tout seul si le tirage a lieu pendant la modification.
  const showForm = editing && view && !view.drawn;

  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <Link href="/secret-santa" className="text-sm font-bold text-slate-500 hover:text-slate-800">
        ← Mes Secret Santa
      </Link>

      {error && (
        <p role="alert" className="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      )}
      {notice && (
        <p role="status" className="mt-4 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
          {notice}
        </p>
      )}

      {!view || !data ? (
        !error && <p className="mt-6 text-sm text-slate-400">Chargement…</p>
      ) : (
        <div className="mt-4 space-y-6">
          <ModulePageHeader
            eyebrow={`Organisé par ${view.organizerName}`}
            title={view.title}
            description={`Tirage le ${longDate(view.revealAt)} · ${view.participants.length} participant(s) sur ${view.invitedCount} invité(s)`}
          />

          {editing && view.drawn && view.isOrganizer ? (
            <SantaInfoForm
              initial={{
                eventPlace: view.eventPlace,
                eventAt: view.eventAt ?? "",
                budget: view.budget,
                message: view.message,
              }}
              onCancel={() => setEditing(false)}
              onSubmit={async (values) => {
                await call("", { method: "PATCH", body: JSON.stringify({ ...values, textsOnly: true }) });
                setEditing(false);
                setNotice("Enregistré. Utilisez « Prévenir d'un changement » pour envoyer un e-mail.");
                await load();
              }}
            />
          ) : showForm ? (
            <SantaForm
              initial={{
                title: view.title,
                budget: view.budget,
                message: view.message,
                revealAt: view.revealAt,
                eventPlace: view.eventPlace,
                eventAt: view.eventAt ?? "",
                participantIds: invitees.map((p) => p.userId),
                exclusions: view.organizer?.exclusions ?? [],
              }}
              submitLabel="Enregistrer"
              onCancel={() => setEditing(false)}
              onSubmit={async (values: SantaFormValues) => {
                await call("", { method: "PATCH", body: JSON.stringify(values) });
                setEditing(false);
                await load();
              }}
            />
          ) : (
            <>
              {view.canRespond && (
                <section className="rounded-2xl border-2 border-amber-300 bg-amber-50 p-5 text-center">
                  <h2 className="text-lg font-black text-slate-900">
                    {view.myResponse === "pending"
                      ? `${view.organizerName} vous invite : participez-vous ?`
                      : view.myResponse === "accepted"
                        ? "Vous participez 🎉"
                        : "Vous ne participez pas"}
                  </h2>
                  <p className="mt-1 text-sm text-slate-600">
                    La participation est libre. Vous pouvez changer d&apos;avis jusqu&apos;au {longDate(view.revealAt)}.
                  </p>
                  <div className="mt-3 flex flex-wrap justify-center gap-3">
                    {view.myResponse !== "accepted" && (
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => respond("accepted")}
                        className="rounded-xl bg-emerald-700 px-6 py-3 text-sm font-bold text-white disabled:opacity-50"
                      >
                        Je participe
                      </button>
                    )}
                    {view.myResponse !== "declined" && (
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => respond("declined")}
                        className="rounded-xl border border-slate-300 bg-white px-6 py-3 text-sm font-bold disabled:opacity-50"
                      >
                        {view.myResponse === "accepted" ? "Je me retire" : "Non merci"}
                      </button>
                    )}
                  </div>
                </section>
              )}

              {view.blocker ? (
                <section className="rounded-2xl border border-amber-300 bg-amber-50 p-6 text-center">
                  <p className="text-lg font-black text-slate-900">{BLOCKER_TEXT[view.blocker]}</p>
                  <p className="mt-2 text-sm text-slate-600">
                    {view.isOrganizer
                      ? "Cliquez sur « Modifier » pour choisir une nouvelle heure de tirage (et, au besoin, revoir les invités ou les paires interdites). Les réponses sont conservées."
                      : "La personne qui organise doit choisir une nouvelle heure."}
                  </p>
                </section>
              ) : view.myResponse === "withdrawn" ? (
                <section className="rounded-2xl border border-slate-200 bg-white p-6 text-center text-slate-700">
                  Vous ne participez plus à ce Secret Santa.
                </section>
              ) : (
                <SantaReveal
                  key={view.revealAt}
                  revealAt={view.revealAt}
                  serverOffsetMs={data.offsetMs}
                  isParticipant={view.isParticipant}
                  mine={view.mine}
                  onDue={() => void load()}
                  onOpened={markOpened}
                />
              )}

              {(view.eventPlace || view.eventAt || view.budget || view.message) && (
                <section className="rounded-2xl border border-slate-200 bg-white p-5 text-sm text-slate-700">
                  {(view.eventPlace || view.eventAt) && (
                    <p>
                      <span className="font-bold">Rendez-vous :</span>{" "}
                      {[view.eventAt ? longDate(view.eventAt) : "", view.eventPlace].filter(Boolean).join(", ")}
                    </p>
                  )}
                  {view.budget && (
                    <p className="mt-1">
                      <span className="font-bold">Budget :</span> {view.budget}
                    </p>
                  )}
                  {view.message && <p className="mt-2 whitespace-pre-line">{view.message}</p>}
                </section>
              )}

              <section className="rounded-2xl border border-slate-200 bg-white p-5">
                <h2 className="font-bold text-slate-900">
                  {organizer ? "Réponses des invités" : "Ils participent"}
                </h2>
                {organizer && (
                  <p className="mt-1 text-xs text-slate-500">
                    {count("accepted")} oui · {count("declined")} non · {count("pending")} sans réponse. Vous seul voyez
                    les refus.
                  </p>
                )}
                {view.exclusionCount > 0 && (
                  <p className="mt-1 text-xs text-slate-500">
                    {view.exclusionCount} paire(s) interdite(s) fixée(s) par la personne qui organise.
                  </p>
                )}
                {!organizer && view.participants.length === 0 && (
                  <p className="mt-2 text-sm text-slate-500">Personne n&apos;a encore répondu oui.</p>
                )}
                <ul className="mt-3 grid gap-1 text-sm sm:grid-cols-2">
                  {!organizer &&
                    view.participants.map((p) => (
                      <li key={p.userId} className="rounded-lg bg-slate-50 px-3 py-1.5">
                        {p.displayName}
                      </li>
                    ))}
                  {invitees.map((p) => (
                    <li key={p.userId} className="flex items-center justify-between gap-2 rounded-lg bg-slate-50 px-3 py-1.5">
                      <span>{p.displayName}</span>
                      <span className="flex shrink-0 flex-wrap items-center justify-end gap-2 text-xs text-slate-500">
                        <span>
                          {RESPONSE_LABELS[p.response]}
                          {!p.invited && " · pas encore invité"}
                          {p.opened && p.response === "accepted" && " · 🎁 a ouvert"}
                        </span>
                        {view.drawn && p.response === "accepted" && (
                          <button
                            type="button"
                            disabled={busy}
                            onClick={() => withdraw(p.userId, p.displayName)}
                            className="font-bold text-rose-700 underline"
                          >
                            Retirer
                          </button>
                        )}
                      </span>
                    </li>
                  ))}
                </ul>
              </section>

              {organizer && (
                <section className="rounded-2xl border border-red-200 bg-red-50/60 p-5">
                  <h2 className="font-bold text-slate-900">Organisation</h2>
                  <p className="mt-1 text-xs text-slate-600">
                    Même vous ne voyez pas qui offre à qui. Le tirage se fait à l&apos;heure prévue.
                  </p>
                  <div className="mt-3 flex flex-wrap gap-2">
                    <button
                      type="button"
                      disabled={busy || notInvited === 0}
                      onClick={() => invite("new", `Envoyer l'invitation à ${notInvited} personne(s) ?`)}
                      className="rounded-xl bg-red-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
                    >
                      Envoyer les invitations ({notInvited})
                    </button>
                    <button
                      type="button"
                      disabled={busy || toRemind === 0 || Boolean(view.blocker)}
                      onClick={() => invite("reminder", `Relancer ${toRemind} personne(s) par e-mail ?`)}
                      className="rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold disabled:opacity-50"
                    >
                      {view.drawn ? `Relancer (${toRemind} n'ont pas ouvert)` : `Relancer (${toRemind} sans réponse)`}
                    </button>
                    <button
                      type="button"
                      disabled={busy || toUpdate === 0}
                      onClick={() =>
                        invite("update", `Prévenir ${toUpdate} personne(s) que les informations ont changé ?`)
                      }
                      className="rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold disabled:opacity-50"
                    >
                      Prévenir d&apos;un changement
                    </button>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => setEditing(true)}
                      className="rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold"
                    >
                      {view.drawn ? "Modifier le rendez-vous" : "Modifier"}
                    </button>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={remove}
                      className="px-2 py-2 text-sm font-bold text-rose-700 underline"
                    >
                      Supprimer
                    </button>
                  </div>
                </section>
              )}
            </>
          )}
        </div>
      )}
    </ModulePageShell>
  );
}
