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

type PortesOuvertesStaffRole = "ambassadeur" | "enseignant" | "personnel";

type StaffRow = {
  id: string;
  slotId: string;
  role: PortesOuvertesStaffRole;
  refId: string;
  displayName: string;
  meta?: Record<string, string>;
  createdAt: string;
};

type SearchKind = "eleve" | "enseignant" | "personnel";

type SearchHit = {
  refId: string;
  displayName: string;
  meta?: Record<string, string>;
};

type SlotWithCount = PortesOuvertesSlot & {
  registeredCount: number;
  remaining: number | null;
  registeredByCycle?: Record<PortesOuvertesCycle, number>;
  remainingByCycle?: Record<PortesOuvertesCycle, number | null>;
  isPast?: boolean;
};

type RegistrationRow = PortesOuvertesRegistration & { upcoming: boolean };

type BoardPayload = {
  title: string;
  address: string;
  mapsUrl: string | null;
  preinscriptionUrl: string | null;
  followUpDelayMinutes: number;
  publicEnabled: boolean;
  slots: SlotWithCount[];
  registrations: RegistrationRow[];
  staff: StaffRow[];
  availableCycles: PortesOuvertesCycle[];
  cycleLabels: Partial<Record<PortesOuvertesCycle, string>>;
  classesByCycle: Partial<Record<PortesOuvertesCycle, string[]>>;
  error?: string;
};

type EditDraft = {
  id: string;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  childFirstName: string;
  childLastName: string;
  cycle: PortesOuvertesCycle;
  classeSouhaitee: string;
  slotId: string;
};

const ROLE_LABELS: Record<PortesOuvertesStaffRole, string> = {
  ambassadeur: "Ambassadeur (élève)",
  enseignant: "Enseignant",
  personnel: "Personnel",
};

const SEARCH_KIND_TO_ROLE: Record<SearchKind, PortesOuvertesStaffRole> = {
  eleve: "ambassadeur",
  enseignant: "enseignant",
  personnel: "personnel",
};

function formatSlotWhen(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "short",
    timeStyle: "short",
  });
}

function formatVisitedAt(iso: string): string {
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

function slotMatchesCycle(s: SlotWithCount, forCycle: PortesOuvertesCycle): boolean {
  return !s.cycle || s.cycle === forCycle;
}

export default function AccueilPortesOuvertesClient() {
  const [board, setBoard] = useState<BoardPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [listFilter, setListFilter] = useState<"all" | "upcoming" | "past">("all");
  const [edit, setEdit] = useState<EditDraft | null>(null);
  const [staffOpenSlotId, setStaffOpenSlotId] = useState<string | null>(null);

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
    setBoard({
      ...data,
      staff: Array.isArray(data.staff) ? data.staff : [],
      followUpDelayMinutes:
        typeof data.followUpDelayMinutes === "number" && data.followUpDelayMinutes > 0
          ? data.followUpDelayMinutes
          : 60,
      preinscriptionUrl: data.preinscriptionUrl ?? null,
    });
    const cycles = data.availableCycles?.length
      ? data.availableCycles
      : (["college"] as PortesOuvertesCycle[]);
    setCycle((prev) => {
      const next = cycles.includes(prev) ? prev : cycles[0];
      setSlotId((prevSlot) => {
        if (
          prevSlot &&
          data.slots.some((s) => s.id === prevSlot && !s.isPast && slotMatchesCycle(s, next))
        ) {
          const slot = data.slots.find((s) => s.id === prevSlot);
          const rem = slot?.remainingByCycle?.[next];
          if (rem === null || rem === undefined || rem > 0) return prevSlot;
        }
        const open = data.slots.find((s) => {
          if (s.isPast || !slotMatchesCycle(s, next)) return false;
          const rem = s.remainingByCycle?.[next];
          return rem === null || rem === undefined || rem > 0;
        });
        return open?.id || data.slots.find((s) => !s.isPast && slotMatchesCycle(s, next))?.id || "";
      });
      return next;
    });
  }, []);

  useEffect(() => {
    void load()
      .catch((e: unknown) => setError(e instanceof Error ? e.message : "Erreur"))
      .finally(() => setLoading(false));
  }, [load]);

  const availableCycles = board?.availableCycles?.length
    ? board.availableCycles
    : (["college"] as PortesOuvertesCycle[]);
  const classes = board?.classesByCycle[cycle] || [];
  const editClasses = edit && board ? board.classesByCycle[edit.cycle] || [] : [];

  function remainingForSlot(s: SlotWithCount, forCycle: PortesOuvertesCycle): number | null {
    if (s.remainingByCycle && forCycle in s.remainingByCycle) {
      return s.remainingByCycle[forCycle];
    }
    return s.remaining;
  }

  function startManualAdd(forCycle: PortesOuvertesCycle) {
    setCycle(forCycle);
    const open = (board?.slots || []).find((s) => {
      if (s.isPast || !slotMatchesCycle(s, forCycle)) return false;
      const rem = remainingForSlot(s, forCycle);
      return rem === null || rem > 0;
    });
    if (open) setSlotId(open.id);
    setMessage(null);
    setError(null);
    requestAnimationFrame(() => {
      document.getElementById("po-saisie")?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }

  useEffect(() => {
    if (!classes.length) {
      setClasseSouhaitee("");
      return;
    }
    setClasseSouhaitee((prev) => (classes.includes(prev) ? prev : classes[0]));
  }, [cycle, classes]);

  useEffect(() => {
    if (!board) return;
    const current = board.slots.find((s) => s.id === slotId);
    const rem =
      current && !current.isPast && slotMatchesCycle(current, cycle)
        ? remainingForSlot(current, cycle)
        : null;
    if (current && !current.isPast && slotMatchesCycle(current, cycle) && (rem === null || rem > 0)) {
      return;
    }
    const open = board.slots.find((s) => {
      if (s.isPast || !slotMatchesCycle(s, cycle)) return false;
      const r = remainingForSlot(s, cycle);
      return r === null || r > 0;
    });
    if (open && open.id !== slotId) setSlotId(open.id);
    else if (!open) setSlotId("");
  }, [board, cycle, slotId]);

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

  const upcomingSlotsForCycle = useMemo(
    () => upcomingSlots.filter((s) => slotMatchesCycle(s, cycle)),
    [upcomingSlots, cycle],
  );

  const editUpcomingSlots = useMemo(() => {
    if (!edit) return upcomingSlotsForCycle;
    return upcomingSlots.filter((s) => slotMatchesCycle(s, edit.cycle));
  }, [edit, upcomingSlots, upcomingSlotsForCycle]);

  const staffingSlots = useMemo(
    () => upcomingSlots.filter((s) => availableCycles.some((c) => slotMatchesCycle(s, c))),
    [upcomingSlots, availableCycles],
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
      childFirstName: r.childFirstName || "",
      childLastName: r.childLastName || "",
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

  async function removeRegistration(r: RegistrationRow) {
    if (!r.upcoming) return;
    const label = [r.firstName, r.lastName].filter(Boolean).join(" ");
    const slotLabel = board ? displaySlot(r, board.slots) : r.slotId;
    const ok = window.confirm(
      `Supprimer l’inscription de ${label} (${slotLabel}) ?\n\nUn e-mail d’annulation avec .ics sera envoyé à ${r.email}.`,
    );
    if (!ok) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/accueil/portes-ouvertes", {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: r.id }),
      });
      const data = (await res.json()) as { error?: string; mailSent?: boolean };
      if (!res.ok) throw new Error(data.error || "Suppression impossible");
      setMessage(
        data.mailSent === false
          ? "Inscription supprimée. Attention : l’e-mail d’annulation/.ics n’a pas pu être envoyé (SMTP)."
          : "Inscription supprimée — e-mail d’annulation avec .ics envoyé.",
      );
      if (edit?.id === r.id) setEdit(null);
      await load();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function toggleVisited(r: RegistrationRow, visited: boolean) {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/accueil/portes-ouvertes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: r.id, visited }),
      });
      const data = (await res.json()) as {
        error?: string;
        followUpDelayMinutes?: number;
      };
      if (!res.ok) throw new Error(data.error || "Check-in impossible");
      const delay = data.followUpDelayMinutes ?? board?.followUpDelayMinutes ?? 60;
      setMessage(
        visited
          ? `Visite enregistrée. Un mail de suivi est prévu dans environ ${delay} min.`
          : "Visite décochée.",
      );
      await load();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function addStaff(params: {
    slotId: string;
    role: PortesOuvertesStaffRole;
    refId: string;
    displayName: string;
    meta?: Record<string, string>;
  }) {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/accueil/portes-ouvertes/staffing", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(params),
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Ajout staffing impossible");
      setMessage(`${params.displayName} ajouté(e) au créneau.`);
      await load();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function removeStaff(id: string) {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/accueil/portes-ouvertes/staffing?id=${encodeURIComponent(id)}`, {
        method: "DELETE",
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Suppression impossible");
      setMessage("Personne retirée du staffing.");
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
              <th className="px-4 py-2 font-bold">Visite</th>
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
                  {r.childFirstName || r.childLastName ? (
                    <span>
                      <span className="font-semibold text-slate-900">
                        {[r.childFirstName, r.childLastName].filter(Boolean).join(" ")}
                      </span>
                      <span className="mt-0.5 block text-xs text-slate-500">
                        Contact : {r.firstName} {r.lastName}
                      </span>
                    </span>
                  ) : (
                    <span>
                      {r.firstName} {r.lastName}
                    </span>
                  )}
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
                  <label className="flex flex-col gap-1 text-xs">
                    <span className="inline-flex items-center gap-2 font-semibold text-slate-700">
                      <input
                        type="checkbox"
                        className="h-4 w-4"
                        checked={Boolean(r.visitedAt)}
                        disabled={busy}
                        onChange={(e) => void toggleVisited(r, e.target.checked)}
                      />
                      Visite effectuée
                    </span>
                    {r.visitedAt ? (
                      <span className="text-emerald-700">Check-in {formatVisitedAt(r.visitedAt)}</span>
                    ) : board?.followUpDelayMinutes ? (
                      <span className="text-slate-400">
                        Suivi e-mail ~{board.followUpDelayMinutes} min après check-in
                      </span>
                    ) : null}
                  </label>
                </td>
                <td className="px-4 py-2">
                  {r.upcoming ? (
                    <div className="flex flex-col items-start gap-1">
                      <button
                        type="button"
                        onClick={() => openEdit(r)}
                        disabled={busy}
                        className="text-xs font-bold text-violet-700 underline disabled:opacity-50"
                      >
                        Modifier
                      </button>
                      <button
                        type="button"
                        onClick={() => void removeRegistration(r)}
                        disabled={busy}
                        className="text-xs font-bold text-rose-700 underline disabled:opacity-50"
                      >
                        Supprimer
                      </button>
                    </div>
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
        description="Saisie téléphone / présentiel, staffing, check-in visite, historique et export planning PDF."
        actions={
          <a
            href="/api/accueil/portes-ouvertes/planning-pdf"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex rounded-xl bg-violet-700 px-4 py-2 text-sm font-bold text-white hover:bg-violet-800"
          >
            Exporter PDF
          </a>
        }
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
              id="po-saisie"
              onSubmit={(ev) => void submit(ev)}
              className="rounded-2xl border border-slate-200 bg-white p-5 md:p-6 space-y-4 shadow-sm scroll-mt-4"
            >
              <h2 className="text-lg font-bold text-slate-900">{board.title || "Nouvelle inscription"}</h2>
              <p className="text-sm text-slate-600">
                Saisie manuelle (téléphone / présentiel). Les places sont comptées{" "}
                <strong>par établissement</strong> sur chaque créneau.
              </p>
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
                  <span className="text-xs font-bold uppercase text-slate-500">Établissement</span>
                  <select
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                    value={cycle}
                    onChange={(e) => setCycle(e.target.value as PortesOuvertesCycle)}
                  >
                    {availableCycles.map((c) => (
                      <option key={c} value={c}>
                        {board?.cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Classe souhaitée</span>
                  {classes.length > 0 ? (
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
                  ) : (
                    <input
                      required
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                      value={classeSouhaitee}
                      onChange={(e) => setClasseSouhaitee(e.target.value)}
                      placeholder="Classe / niveau"
                    />
                  )}
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
                  {upcomingSlotsForCycle.map((s) => {
                    const rem = remainingForSlot(s, cycle);
                    const full = rem === 0;
                    const places =
                      rem === null
                        ? `${s.registeredByCycle?.[cycle] ?? s.registeredCount} inscrit(s)`
                        : `${rem} place(s) restante(s)`;
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
                Changement de créneau, de coordonnées ou de prénom → nouvel e-mail « créneau modifié »
                avec .ics.
              </p>
              <div className="grid gap-4 sm:grid-cols-2">
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">
                    Prénom contact
                  </span>
                  <input
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.firstName}
                    onChange={(e) => setEdit({ ...edit, firstName: e.target.value })}
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Nom contact</span>
                  <input
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.lastName}
                    onChange={(e) => setEdit({ ...edit, lastName: e.target.value })}
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">
                    Prénom enfant
                  </span>
                  <input
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.childFirstName}
                    onChange={(e) => setEdit({ ...edit, childFirstName: e.target.value })}
                    placeholder="Optionnel"
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Nom enfant</span>
                  <input
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                    value={edit.childLastName}
                    onChange={(e) => setEdit({ ...edit, childLastName: e.target.value })}
                    placeholder="Optionnel"
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
                  <span className="text-xs font-bold uppercase text-slate-500">Établissement</span>
                  <select
                    className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm font-semibold"
                    value={edit.cycle}
                    onChange={(e) =>
                      setEdit({ ...edit, cycle: e.target.value as PortesOuvertesCycle })
                    }
                  >
                    {Array.from(
                      new Set<PortesOuvertesCycle>([...availableCycles, edit.cycle]),
                    ).map((c) => (
                      <option key={c} value={c}>
                        {board?.cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="block">
                  <span className="text-xs font-bold uppercase text-slate-500">Classe</span>
                  {editClasses.length > 0 ? (
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
                  ) : (
                    <input
                      required
                      className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2.5 text-sm"
                      value={edit.classeSouhaitee}
                      onChange={(e) => setEdit({ ...edit, classeSouhaitee: e.target.value })}
                    />
                  )}
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
                  {editUpcomingSlots.map((s) => {
                    const rem = remainingForSlot(s, edit.cycle);
                    const full = rem === 0 && s.id !== edit.slotId;
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

          {staffingSlots.length > 0 ? (
            <section className="space-y-4">
              <div>
                <h2 className="text-lg font-bold text-slate-900">Staffing des créneaux</h2>
                <p className="text-sm text-slate-600">
                  Ambassadeurs (élèves), enseignants et personnel par créneau à venir — exportés
                  dans le planning PDF.
                </p>
              </div>
              {staffingSlots.map((s) => {
                const open = staffOpenSlotId === s.id;
                const slotStaff = (board.staff || []).filter((x) => x.slotId === s.id);
                return (
                  <div
                    key={s.id}
                    className="rounded-2xl border border-slate-200 bg-white overflow-hidden"
                  >
                    <button
                      type="button"
                      className="w-full flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 bg-slate-50 px-4 py-3 text-left"
                      onClick={() => setStaffOpenSlotId(open ? null : s.id)}
                    >
                      <div>
                        <h3 className="font-bold text-slate-900">
                          {s.label}
                          {s.cycle
                            ? ` — ${board.cycleLabels[s.cycle] || PORTES_OUVERTES_CYCLE_LABELS[s.cycle]}`
                            : ""}
                        </h3>
                        <p className="text-xs text-slate-500">
                          {formatSlotWhen(s.startAt)} · {slotStaff.length} personne(s)
                        </p>
                      </div>
                      <span className="text-xs font-bold text-violet-700">
                        {open ? "Replier" : "Gérer"}
                      </span>
                    </button>
                    {open ? (
                      <div className="p-4 space-y-4">
                        {(["eleve", "enseignant", "personnel"] as const).map((kind) => (
                          <StaffSearchPicker
                            key={kind}
                            kind={kind}
                            disabled={busy}
                            onPick={(hit) =>
                              void addStaff({
                                slotId: s.id,
                                role: SEARCH_KIND_TO_ROLE[kind],
                                refId: hit.refId,
                                displayName: hit.displayName,
                                meta: hit.meta,
                              })
                            }
                          />
                        ))}
                        {slotStaff.length === 0 ? (
                          <p className="text-sm text-slate-500">Personne assignée pour l’instant.</p>
                        ) : (
                          <ul className="divide-y divide-slate-100 rounded-xl border border-slate-100">
                            {slotStaff.map((row) => (
                              <li
                                key={row.id}
                                className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm"
                              >
                                <div>
                                  <span className="font-semibold text-slate-900">
                                    {row.displayName}
                                  </span>
                                  <span className="ml-2 text-xs font-semibold text-violet-700">
                                    {ROLE_LABELS[row.role]}
                                  </span>
                                  {row.meta?.classe ? (
                                    <span className="ml-2 text-xs text-slate-500">
                                      {row.meta.classe}
                                    </span>
                                  ) : null}
                                </div>
                                <button
                                  type="button"
                                  disabled={busy}
                                  className="text-xs font-bold text-rose-600 disabled:opacity-50"
                                  onClick={() => void removeStaff(row.id)}
                                >
                                  Retirer
                                </button>
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    ) : null}
                  </div>
                );
              })}
            </section>
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
              Historique conservé même après la session — utile pour recontacter. Cochez « Visite
              effectuée » pour déclencher le suivi (délai {board.followUpDelayMinutes} min).
            </p>

            {availableCycles.map((c) => {
              const rows = byCycle[c];
              const cycleUpcoming = upcomingSlots.filter((s) => slotMatchesCycle(s, c));
              return (
                <div key={c} className="rounded-2xl border border-slate-200 bg-white overflow-hidden">
                  <div className="border-b border-slate-100 bg-slate-50 px-4 py-3 flex flex-wrap items-center justify-between gap-2">
                    <div>
                      <h3 className="font-bold text-slate-900">
                        {board.cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                      </h3>
                      <p className="text-xs text-slate-500">
                        {rows.length} inscription(s)
                        {cycleUpcoming[0]?.maxPlaces
                          ? ` — plafond ${cycleUpcoming[0].maxPlaces} places / créneau`
                          : ""}
                      </p>
                    </div>
                    <button
                      type="button"
                      onClick={() => startManualAdd(c)}
                      disabled={cycleUpcoming.length === 0}
                      className="rounded-lg bg-violet-700 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-40"
                    >
                      Ajouter une inscription
                    </button>
                  </div>
                  {renderTable(rows, "cycle")}
                </div>
              );
            })}

            {PORTES_OUVERTES_CYCLES.filter((c) => !availableCycles.includes(c) && byCycle[c].length > 0).map(
              (c) => {
                const rows = byCycle[c];
                return (
                  <div key={`hist-${c}`} className="rounded-2xl border border-slate-200 bg-white overflow-hidden">
                    <div className="border-b border-slate-100 bg-slate-50 px-4 py-3 flex items-center justify-between">
                      <h3 className="font-bold text-slate-900">
                        {board.cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}{" "}
                        <span className="text-xs font-semibold text-slate-500">(historique)</span>
                      </h3>
                      <span className="text-xs font-semibold text-slate-500">
                        {rows.length} inscription(s)
                      </span>
                    </div>
                    {renderTable(rows, "cycle")}
                  </div>
                );
              },
            )}

            {byCycle.autre.length > 0 ? (
              <div className="rounded-2xl border border-slate-200 bg-white overflow-hidden">
                <div className="border-b border-slate-100 bg-slate-50 px-4 py-3">
                  <h3 className="font-bold text-slate-900">Autres (sans établissement)</h3>
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

function StaffSearchPicker({
  kind,
  disabled,
  onPick,
}: {
  kind: SearchKind;
  disabled: boolean;
  onPick: (hit: SearchHit) => void;
}) {
  const [q, setQ] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    const needle = q.trim();
    if (needle.length < 2) {
      setResults([]);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      setSearching(true);
      void fetch(
        `/api/accueil/portes-ouvertes/search?kind=${encodeURIComponent(kind)}&q=${encodeURIComponent(needle)}`,
        { cache: "no-store" },
      )
        .then(async (res) => {
          const data = (await res.json()) as { results?: SearchHit[]; error?: string };
          if (!res.ok) throw new Error(data.error || "Recherche impossible");
          if (!cancelled) setResults(Array.isArray(data.results) ? data.results : []);
        })
        .catch(() => {
          if (!cancelled) setResults([]);
        })
        .finally(() => {
          if (!cancelled) setSearching(false);
        });
    }, 250);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [q, kind]);

  const title =
    kind === "eleve" ? "Ambassadeur (élève)" : kind === "enseignant" ? "Enseignant" : "Personnel";

  return (
    <div className="space-y-2">
      <label className="block">
        <span className="text-xs font-bold uppercase text-slate-500">{title}</span>
        <input
          className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
          placeholder="Rechercher (2 caractères min.)"
          value={q}
          disabled={disabled}
          onChange={(e) => setQ(e.target.value)}
        />
      </label>
      {searching ? <p className="text-xs text-slate-400">Recherche…</p> : null}
      {results.length > 0 ? (
        <ul className="max-h-40 overflow-y-auto rounded-xl border border-violet-100 bg-violet-50/40">
          {results.map((hit) => (
            <li key={`${hit.refId}-${hit.displayName}`}>
              <button
                type="button"
                disabled={disabled}
                className="w-full px-3 py-2 text-left text-sm hover:bg-violet-100 disabled:opacity-50"
                onClick={() => {
                  onPick(hit);
                  setQ("");
                  setResults([]);
                }}
              >
                <span className="font-semibold text-slate-900">{hit.displayName}</span>
                {hit.meta?.classe ? (
                  <span className="ml-2 text-xs text-slate-500">{hit.meta.classe}</span>
                ) : hit.meta?.email ? (
                  <span className="ml-2 text-xs text-slate-500">{hit.meta.email}</span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
