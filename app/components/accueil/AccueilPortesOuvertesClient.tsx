"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import type { PortesOuvertesRegistration } from "@/app/lib/portes-ouvertes-types";
import {
  PORTES_OUVERTES_CYCLE_LABELS,
  PORTES_OUVERTES_CYCLES,
  type PortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesSlot } from "@/app/lib/toolbox-types";

type SlotWithCount = PortesOuvertesSlot & {
  registeredCount: number;
  remaining: number | null;
  isPast?: boolean;
};

type RegistrationRow = PortesOuvertesRegistration & { upcoming: boolean };

type BoardPayload = {
  title: string;
  address: string;
  mapsUrl: string | null;
  publicEnabled: boolean;
  slots: SlotWithCount[];
  registrations: RegistrationRow[];
  classesByCycle: Record<PortesOuvertesCycle, string[]>;
  error?: string;
};

type EditDraft = {
  id: string;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  cycle: PortesOuvertesCycle;
  classeSouhaitee: string;
  slotId: string;
};

function formatSlotWhen(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "short",
    timeStyle: "short",
  });
}

function displaySlot(reg: PortesOuvertesRegistration, slots: SlotWithCount[]): string {
  if (reg.slotLabel && reg.slotStartAt) {
    return `${reg.slotLabel} (${formatSlotWhen(reg.slotStartAt)})`;
  }
  const s = slots.find((x) => x.id === reg.slotId);
  if (!s) return reg.slotId;
  return `${s.label} (${formatSlotWhen(s.startAt)})`;
}

export default function AccueilPortesOuvertesClient() {
  const [board, setBoard] = useState<BoardPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [listFilter, setListFilter] = useState<"all" | "upcoming" | "past">("all");
  const [edit, setEdit] = useState<EditDraft | null>(null);

  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [cycle, setCycle] = useState<PortesOuvertesCycle>("college");
  const [classeSouhaitee, setClasseSouhaitee] = useState("");
  const [slotId, setSlotId] = useState("");

  const load = useCallback(async () => {
    const res = await fetch("/api/accueil/portes-ouvertes", { cache: "no-store" });
    const data = (await res.json()) as BoardPayload;
    if (!res.ok) throw new Error(data.error || "Chargement impossible");
    setBoard(data);
    setSlotId((prev) => {
      if (prev && data.slots.some((s) => s.id === prev && !s.isPast)) return prev;
      const open = data.slots.find(
        (s) => !s.isPast && (s.remaining === null || s.remaining > 0),
      );
      return open?.id || data.slots.find((s) => !s.isPast)?.id || "";
    });
  }, []);

  useEffect(() => {
    void load()
      .catch((e: unknown) => setError(e instanceof Error ? e.message : "Erreur"))
      .finally(() => setLoading(false));
  }, [load]);

  const classes = board?.classesByCycle[cycle] || [];
  const editClasses = edit && board ? board.classesByCycle[edit.cycle] || [] : [];

  useEffect(() => {
    if (!classes.length) {
      setClasseSouhaitee("");
      return;
    }
    setClasseSouhaitee((prev) => (classes.includes(prev) ? prev : classes[0]));
  }, [cycle, classes]);

  useEffect(() => {
    if (!edit) return;
    if (!editClasses.length) return;
    if (!editClasses.includes(edit.classeSouhaitee)) {
      setEdit({ ...edit, classeSouhaitee: editClasses[0] });
    }
  }, [edit, editClasses]);

  const filteredRegs = useMemo(() => {
    const regs = board?.registrations || [];
    if (listFilter === "upcoming") return regs.filter((r) => r.upcoming);
    if (listFilter === "past") return regs.filter((r) => !r.upcoming);
    return regs;
  }, [board?.registrations, listFilter]);

  const byCycle = useMemo(() => {
    const groups: Record<PortesOuvertesCycle | "autre", RegistrationRow[]> = {
      ecole: [],
      college: [],
      lycee: [],
      autre: [],
    };
    for (const r of filteredRegs) {
      if (r.cycle === "ecole" || r.cycle === "college" || r.cycle === "lycee") {
        groups[r.cycle].push(r);
      } else {
        groups.autre.push(r);
      }
    }
    for (const key of Object.keys(groups) as Array<keyof typeof groups>) {
      groups[key].sort((a, b) => {
        const ca = (a.classeSouhaitee || "").localeCompare(b.classeSouhaitee || "", "fr");
        if (ca !== 0) return ca;
        const aStart = a.slotStartAt || a.createdAt;
        const bStart = b.slotStartAt || b.createdAt;
        return bStart.localeCompare(aStart);
      });
    }
    return groups;
  }, [filteredRegs]);

  const upcomingSlots = useMemo(
    () => (board?.slots || []).filter((s) => !s.isPast),
    [board?.slots],
  );

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/accueil/portes-ouvertes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          firstName,
          lastName,
          email,
          phone,
          cycle,
          classeSouhaitee,
          slotId,
        }),
      });
      const data = (await res.json()) as { error?: string; mailSent?: boolean };
      if (!res.ok) throw new Error(data.error || "Enregistrement impossible");
      setMessage(
        data.mailSent === false
          ? "Inscription enregistrée. Attention : l’e-mail/.ics n’a pas pu être envoyé (SMTP)."
          : "Inscription enregistrée — e-mail de confirmation avec .ics envoyé.",
      );
      setFirstName("");
      setLastName("");
      setEmail("");
      setPhone("");
      await load();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  function openEdit(r: RegistrationRow) {
    if (!r.upcoming) return;
    setEdit({
      id: r.id,
      firstName: r.firstName,
      lastName: r.lastName,
      email: r.email,
      phone: r.phone || "",
      cycle: r.cycle || "college",
      classeSouhaitee: r.classeSouhaitee || "",
      slotId: r.slotId,
    });
    setError(null);
    setMessage(null);
  }

  async function saveEdit(e: React.FormEvent) {
    e.preventDefault();
    if (!edit) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/accueil/portes-ouvertes", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(edit),
      });
      const data = (await res.json()) as { error?: string; mailSent?: boolean };
      if (!res.ok) throw new Error(data.error || "Modification impossible");
      setMessage(
        data.mailSent === false
          ? "Créneau mis à jour. Attention : le nouvel e-mail/.ics n’a pas pu être envoyé (SMTP)."
          : "Créneau modifié — nouvel e-mail de confirmation avec .ics envoyé.",
      );
      setEdit(null);
      await load();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  function renderTable(rows: RegistrationRow[], variant: "cycle" | "autre") {
    if (rows.length === 0) {
      return <p className="px-4 py-6 text-sm text-slate-500">Aucune inscription.</p>;
    }
    return (
      <div className="overflow-x-auto">
        <table className="min-w-full text-sm">
          <thead className="bg-white text-left text-xs uppercase text-slate-500">
            <tr>
              <th className="px-4 py-2 font-bold">{variant === "autre" ? "Info" : "Classe"}</th>
              <th className="px-4 py-2 font-bold">Nom</th>
              <th className="px-4 py-2 font-bold">Téléphone</th>
              <th className="px-4 py-2 font-bold">E-mail</th>
              <th className="px-4 py-2 font-bold">Créneau</th>
              <th className="px-4 py-2 font-bold">Statut</th>
              <th className="px-4 py-2 font-bold">Actions</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.id} className="border-t border-slate-100">
                <td className="px-4 py-2 font-semibold text-slate-900">
                  {variant === "autre" ? r.childrenInfo || "—" : r.classeSouhaitee || "—"}
                </td>
                <td className="px-4 py-2">
                  {r.firstName} {r.lastName}
                </td>
                <td className="px-4 py-2">{r.phone || "—"}</td>
                <td className="px-4 py-2">{r.email}</td>
                <td className="px-4 py-2">{board ? displaySlot(r, board.slots) : r.slotId}</td>
                <td className="px-4 py-2">
                  {r.upcoming ? (
                    <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-xs font-semibold text-emerald-800">
                      À venir
                    </span>
                  ) : (
                    <span className="rounded-full bg-slate-100 px-2 py-0.5 text-xs font-semibold text-slate-600">
                      Passée
                    </span>
                  )}
                </td>
                <td className="px-4 py-2">
                  {r.upcoming ? (
                    <button
                      type="button"
                      onClick={() => openEdit(r)}
                      className="text-xs font-bold text-violet-700 underline"
                    >
                      Modifier
                    </button>
                  ) : (
                    <span className="text-xs text-slate-400">Historique</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  return (
    <ModulePageShell>
      <ModulePageHeader
        title="Portes ouvertes — Accueil"
        description="Saisie téléphone / présentiel, historique de toutes les sessions, modification de créneau avec renvoi .ics."
      />

      {loading ? <p className="text-sm text-slate-500">Chargement…</p> : null}
      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">{error}</p>
      ) : null}
      {message ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
          {message}
        </p>
      ) : null}

      {board ? (
        <div className="space-y-8">
          {!board.publicEnabled ? (
            <p className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
              La page publique <code className="font-mono">/portes-ouvertes</code> est désactivée.
              La saisie Accueil reste possible tant que des créneaux sont configurés.
            </p>
          ) : null}

          {upcomingSlots.length === 0 ? (
            <p className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
              Aucun créneau à venir. Un admin peut en générer dans{" "}
              <a href="/etablissement/evenements?tab=portes-ouvertes" className="font-semibold underline">
                Événements → Portes ouvertes
              </a>
              . L’historique des inscrits reste visible ci-dessous.
            </p>
          ) : (
            <form
              onSubmit={(ev) => void submit(ev)}
              className="rounded-2xl border border-slate-200 bg-white p-5 md:p-6 space-y-4 shadow-sm"
            >
              <h2 className="text-lg font-bold text-slate-900">{board.title || "Nouvelle inscription"}</h2>
              {board.address ? (
                <p className="text-sm text-slate-600">
                  {board.address}
                  {board.mapsUrl ? (
                    <>
                      {" "}
                      —{" "}
                      <a
                        href={board.mapsUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-violet-700 underline"
                      >
                        Itinéraire
                      </a>
                    </>
                  ) : null}
                </p>
              ) : null}

              <div className="grid gap-4 sm:grid-cols-2">
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Prénom</span>
                  <input
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                    value={firstName}
                    onChange={(e) => setFirstName(e.target.value)}
                    autoComplete="given-name"
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Nom</span>
                  <input
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                    value={lastName}
                    onChange={(e) => setLastName(e.target.value)}
                    autoComplete="family-name"
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">E-mail (destinataire .ics)</span>
                  <input
                    required
                    type="email"
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    autoComplete="email"
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Téléphone</span>
                  <input
                    required
                    type="tel"
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                    value={phone}
                    onChange={(e) => setPhone(e.target.value)}
                    autoComplete="tel"
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Cycle</span>
                  <select
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                    value={cycle}
                    onChange={(e) => setCycle(e.target.value as PortesOuvertesCycle)}
                  >
                    {PORTES_OUVERTES_CYCLES.map((c) => (
                      <option key={c} value={c}>
                        {PORTES_OUVERTES_CYCLE_LABELS[c]}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Classe souhaitée</span>
                  <select
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                    value={classeSouhaitee}
                    onChange={(e) => setClasseSouhaitee(e.target.value)}
                  >
                    {classes.map((c) => (
                      <option key={c} value={c}>
                        {c}
                      </option>
                    ))}
                  </select>
                </label>
              </div>

              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Créneau</span>
                <select
                  required
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                  value={slotId}
                  onChange={(e) => setSlotId(e.target.value)}
                >
                  {upcomingSlots.map((s) => {
                    const full = s.remaining === 0;
                    const places =
                      s.remaining === null
                        ? `${s.registeredCount} inscrit(s)`
                        : `${s.remaining} place(s) restante(s)`;
                    return (
                      <option key={s.id} value={s.id} disabled={full}>
                        {s.label} — {formatSlotWhen(s.startAt)} ({places})
                        {full ? " — complet" : ""}
                      </option>
                    );
                  })}
                </select>
              </label>

              <button
                type="submit"
                disabled={busy || !slotId}
                className="rounded-xl bg-violet-700 px-5 py-2.5 text-sm font-bold text-white disabled:opacity-50"
              >
                {busy && !edit ? "Enregistrement…" : "Valider et envoyer le .ics"}
              </button>
            </form>
          )}

          {edit ? (
            <form
              onSubmit={(ev) => void saveEdit(ev)}
              className="rounded-2xl border border-violet-200 bg-violet-50/40 p-5 md:p-6 space-y-4"
            >
              <div className="flex items-start justify-between gap-3">
                <h2 className="text-lg font-bold text-violet-950">Modifier l’inscription</h2>
                <button
                  type="button"
                  className="text-xs font-bold text-slate-600 underline"
                  onClick={() => setEdit(null)}
                >
                  Annuler
                </button>
              </div>
              <p className="text-sm text-violet-900">
                Changement de créneau / coordonnées → nouvel e-mail de confirmation avec .ics.
              </p>
              <div className="grid gap-4 sm:grid-cols-2">
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Prénom</span>
                  <input
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.firstName}
                    onChange={(e) => setEdit({ ...edit, firstName: e.target.value })}
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Nom</span>
                  <input
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.lastName}
                    onChange={(e) => setEdit({ ...edit, lastName: e.target.value })}
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">E-mail</span>
                  <input
                    required
                    type="email"
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.email}
                    onChange={(e) => setEdit({ ...edit, email: e.target.value })}
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Téléphone</span>
                  <input
                    required
                    type="tel"
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.phone}
                    onChange={(e) => setEdit({ ...edit, phone: e.target.value })}
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Cycle</span>
                  <select
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm font-semibold"
                    value={edit.cycle}
                    onChange={(e) =>
                      setEdit({ ...edit, cycle: e.target.value as PortesOuvertesCycle })
                    }
                  >
                    {PORTES_OUVERTES_CYCLES.map((c) => (
                      <option key={c} value={c}>
                        {PORTES_OUVERTES_CYCLE_LABELS[c]}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Classe</span>
                  <select
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm font-semibold"
                    value={edit.classeSouhaitee}
                    onChange={(e) => setEdit({ ...edit, classeSouhaitee: e.target.value })}
                  >
                    {editClasses.map((c) => (
                      <option key={c} value={c}>
                        {c}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Nouveau créneau</span>
                <select
                  required
                  className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm font-semibold"
                  value={edit.slotId}
                  onChange={(e) => setEdit({ ...edit, slotId: e.target.value })}
                >
                  {upcomingSlots.map((s) => {
                    const full = s.remaining === 0 && s.id !== edit.slotId;
                    return (
                      <option key={s.id} value={s.id} disabled={full}>
                        {s.label} — {formatSlotWhen(s.startAt)}
                        {full ? " — complet" : ""}
                      </option>
                    );
                  })}
                </select>
              </label>
              <button
                type="submit"
                disabled={busy}
                className="rounded-xl bg-violet-700 px-5 py-2.5 text-sm font-bold text-white disabled:opacity-50"
              >
                {busy ? "Envoi…" : "Enregistrer et renvoyer le .ics"}
              </button>
            </form>
          ) : null}

          <section className="space-y-4">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <h2 className="text-lg font-bold text-slate-900">
                Tous les inscrits ({board.registrations.length})
              </h2>
              <div className="flex flex-wrap gap-2">
                {(
                  [
                    ["all", "Toutes les sessions"],
                    ["upcoming", "À venir"],
                    ["past", "Passées"],
                  ] as const
                ).map(([id, label]) => (
                  <button
                    key={id}
                    type="button"
                    onClick={() => setListFilter(id)}
                    className={`rounded-lg px-3 py-1.5 text-xs font-bold ${
                      listFilter === id
                        ? "bg-slate-900 text-white"
                        : "bg-slate-100 text-slate-700"
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
            <p className="text-sm text-slate-600">
              Historique conservé même après la session — utile pour recontacter. Modification
              possible uniquement sur les créneaux encore à venir.
            </p>

            {PORTES_OUVERTES_CYCLES.map((c) => {
              const rows = byCycle[c];
              return (
                <div key={c} className="rounded-2xl border border-slate-200 bg-white overflow-hidden">
                  <div className="border-b border-slate-100 bg-slate-50 px-4 py-3 flex items-center justify-between">
                    <h3 className="font-bold text-slate-900">{PORTES_OUVERTES_CYCLE_LABELS[c]}</h3>
                    <span className="text-xs font-semibold text-slate-500">
                      {rows.length} inscription(s)
                    </span>
                  </div>
                  {renderTable(rows, "cycle")}
                </div>
              );
            })}

            {byCycle.autre.length > 0 ? (
              <div className="rounded-2xl border border-slate-200 bg-white overflow-hidden">
                <div className="border-b border-slate-100 bg-slate-50 px-4 py-3">
                  <h3 className="font-bold text-slate-900">Autres (sans cycle)</h3>
                </div>
                {renderTable(byCycle.autre, "autre")}
              </div>
            ) : null}
          </section>
        </div>
      ) : null}
    </ModulePageShell>
  );
}
