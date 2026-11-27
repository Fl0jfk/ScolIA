"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "@/app/components/ui/AppLink";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import SantaForm, { type SantaFormValues } from "@/app/components/secret-santa/SantaForm";
import SantaReveal from "@/app/components/secret-santa/SantaReveal";
import type { SantaView } from "@/app/lib/secret-santa";

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
    const onVisible = () => document.visibilityState === "visible" && void load();
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [pending, load]);

  const markOpened = useCallback(() => {
    void fetch(`/api/secret-santa/${id}/ouvert`, { method: "POST" }).catch(() => undefined);
  }, [id]);

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

  const invite = (mode: "new" | "unopened", count: number) => {
    if (!window.confirm(`Envoyer un e-mail à ${count} personne(s) ?`)) return;
    void act(async () => {
      const result = await call("/invitations", { method: "POST", body: JSON.stringify({ mode }) });
      setNotice(
        result.sent === 0 && result.failed === 0
          ? "Aucun e-mail à envoyer : ces personnes en ont reçu un il y a moins de 10 minutes."
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
  const status = new Map(organizer?.status.map((s) => [s.userId, s]) ?? []);
  const notInvited = organizer?.status.filter((s) => !s.invited).length ?? 0;
  const toRemind = organizer?.status.filter((s) => !s.opened).length ?? 0;
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
            description={`Tirage le ${new Date(view.revealAt).toLocaleString("fr-FR", {
              weekday: "long",
              day: "numeric",
              month: "long",
              hour: "2-digit",
              minute: "2-digit",
            })} · ${view.participants.length} participants`}
          />

          {showForm ? (
            <SantaForm
              initial={{
                title: view.title,
                budget: view.budget,
                message: view.message,
                revealAt: view.revealAt,
                participantIds: view.participants.map((p) => p.userId),
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
              <SantaReveal
                key={view.revealAt}
                revealAt={view.revealAt}
                serverOffsetMs={data.offsetMs}
                isParticipant={view.isParticipant}
                mine={view.mine}
                onDue={() => void load()}
                onOpened={markOpened}
              />

              {(view.budget || view.message) && (
                <section className="rounded-2xl border border-slate-200 bg-white p-5 text-sm text-slate-700">
                  {view.budget && (
                    <p>
                      <span className="font-bold">Budget :</span> {view.budget}
                    </p>
                  )}
                  {view.message && <p className="mt-2 whitespace-pre-line">{view.message}</p>}
                </section>
              )}

              <section className="rounded-2xl border border-slate-200 bg-white p-5">
                <h2 className="font-bold text-slate-900">Participants</h2>
                {view.exclusionCount > 0 && (
                  <p className="mt-1 text-xs text-slate-500">
                    {view.exclusionCount} paire(s) interdite(s) fixée(s) par la personne qui organise.
                  </p>
                )}
                <ul className="mt-3 grid gap-1 text-sm sm:grid-cols-2">
                  {view.participants.map((p) => {
                    const s = status.get(p.userId);
                    return (
                      <li key={p.userId} className="flex items-center justify-between gap-2 rounded-lg bg-slate-50 px-3 py-1.5">
                        <span>{p.displayName}</span>
                        {s && (
                          <span className="shrink-0 text-xs text-slate-500">
                            {s.opened ? "🎁 a ouvert" : s.invited ? "✉️ invité" : "pas encore invité"}
                          </span>
                        )}
                      </li>
                    );
                  })}
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
                      onClick={() => invite("new", notInvited)}
                      className="rounded-xl bg-red-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
                    >
                      Envoyer les invitations ({notInvited})
                    </button>
                    <button
                      type="button"
                      disabled={busy || toRemind === 0}
                      onClick={() => invite("unopened", toRemind)}
                      className="rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold disabled:opacity-50"
                    >
                      Relancer ({toRemind} n&apos;ont pas ouvert)
                    </button>
                    {!view.drawn && (
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => setEditing(true)}
                        className="rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-bold"
                      >
                        Modifier
                      </button>
                    )}
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
