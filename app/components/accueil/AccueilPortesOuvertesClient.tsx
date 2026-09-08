"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import { formatParisHm, formatParisTimeLabel, parisDateKey, parisWallTimeToDate } from "@/app/lib/paris-time";
import type { PortesOuvertesRegistration } from "@/app/lib/portes-ouvertes-types";
import {
  PORTES_OUVERTES_CYCLE_LABELS,
  PORTES_OUVERTES_CYCLES,
  PORTES_OUVERTES_MAX_AMBASSADEURS,
  PORTES_OUVERTES_MAX_ENCADRANTS,
  PORTES_OUVERTES_STAFF_ROLE_LABELS,
  type PortesOuvertesCycle,
  type PortesOuvertesStaffRole,
} from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesSlot } from "@/app/lib/toolbox-types";

type AccueilPoVue = "planning" | "parametrage";

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
  canManageParametrage?: boolean;
  slots: SlotWithCount[];
  registrations: RegistrationRow[];
  staff: StaffRow[];
  availableCycles: PortesOuvertesCycle[];
  cycleLabels: Partial<Record<PortesOuvertesCycle, string>>;
  classesByCycle: Partial<Record<PortesOuvertesCycle, string[]>>;
  error?: string;
};

type AddDraft = {
  slotId: string;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  cycle: PortesOuvertesCycle;
  classeSouhaitee: string;
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

const ROLE_LABELS = PORTES_OUVERTES_STAFF_ROLE_LABELS;

function slotMatchesCycle(s: SlotWithCount, forCycle: PortesOuvertesCycle): boolean {
  return !s.cycle || s.cycle === forCycle;
}

function formatSlotDayLabel(isoOrDayKey: string): string {
  const d =
    /^\d{4}-\d{2}-\d{2}$/.test(isoOrDayKey)
      ? parisWallTimeToDate(isoOrDayKey, 12, 0) || new Date(`${isoOrDayKey}T12:00:00`)
      : new Date(isoOrDayKey);
  return d.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function visitorLabel(r: PortesOuvertesRegistration): string {
  const child = [r.childFirstName, r.childLastName].filter(Boolean).join(" ");
  if (child) return child;
  return `${r.firstName} ${r.lastName}`.trim();
}

function emptyAddDraft(
  slotId: string,
  cycle: PortesOuvertesCycle,
  classe: string,
): AddDraft {
  return {
    slotId,
    firstName: "",
    lastName: "",
    email: "",
    phone: "",
    cycle,
    classeSouhaitee: classe,
  };
}

export default function AccueilPortesOuvertesClient({
  initialVue = "planning",
}: {
  initialVue?: AccueilPoVue;
}) {
  const [vue, setVue] = useState<AccueilPoVue>(initialVue);
  const [board, setBoard] = useState<BoardPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [dayKey, setDayKey] = useState<string>("");
  const [cycleFilter, setCycleFilter] = useState<PortesOuvertesCycle | "all">("all");
  const [addDraft, setAddDraft] = useState<AddDraft | null>(null);
  const [edit, setEdit] = useState<EditDraft | null>(null);
  const [staffOpenSlotId, setStaffOpenSlotId] = useState<string | null>(null);

  const load = useCallback(async () => {
    const res = await fetch("/api/accueil/portes-ouvertes", { cache: "no-store" });
    const data = (await res.json()) as BoardPayload;
    if (!res.ok) throw new Error(data.error || "Chargement impossible");
    setBoard({
      ...data,
      staff: Array.isArray(data.staff) ? data.staff : [],
      canManageParametrage: Boolean(data.canManageParametrage),
      followUpDelayMinutes:
        typeof data.followUpDelayMinutes === "number" && data.followUpDelayMinutes > 0
          ? data.followUpDelayMinutes
          : 60,
      preinscriptionUrl: data.preinscriptionUrl ?? null,
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

  const dayOptions = useMemo(() => {
    const keys = new Set<string>();
    for (const s of board?.slots || []) {
      keys.add(parisDateKey(s.startAt));
    }
    return [...keys].sort();
  }, [board?.slots]);

  useEffect(() => {
    if (!dayOptions.length) {
      setDayKey("");
      return;
    }
    setDayKey((prev) => {
      if (prev && dayOptions.includes(prev)) return prev;
      const today = parisDateKey(new Date());
      if (dayOptions.includes(today)) return today;
      const upcoming = dayOptions.find((k) => k >= today);
      return upcoming || dayOptions[dayOptions.length - 1];
    });
  }, [dayOptions]);

  const daySlots = useMemo(() => {
    const slots = (board?.slots || [])
      .filter((s) => parisDateKey(s.startAt) === dayKey)
      .filter((s) => {
        if (cycleFilter === "all") return true;
        return slotMatchesCycle(s, cycleFilter);
      })
      .sort((a, b) => a.startAt.localeCompare(b.startAt));
    return slots;
  }, [board?.slots, dayKey, cycleFilter]);

  function remainingForSlot(s: SlotWithCount, forCycle: PortesOuvertesCycle): number | null {
    if (s.remainingByCycle && forCycle in s.remainingByCycle) {
      return s.remainingByCycle[forCycle];
    }
    return s.remaining;
  }

  function regsForSlot(slotId: string): RegistrationRow[] {
    return (board?.registrations || [])
      .filter((r) => r.slotId === slotId)
      .sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  }

  function openAdd(slot: SlotWithCount) {
    const cycle =
      cycleFilter !== "all"
        ? cycleFilter
        : slot.cycle && availableCycles.includes(slot.cycle)
          ? slot.cycle
          : availableCycles[0];
    const classes = board?.classesByCycle[cycle] || [];
    setAddDraft(emptyAddDraft(slot.id, cycle, classes[0] || ""));
    setError(null);
    setMessage(null);
  }

  async function submitAdd(e: React.FormEvent) {
    e.preventDefault();
    if (!addDraft) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/accueil/portes-ouvertes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          slotId: addDraft.slotId,
          firstName: addDraft.firstName.trim(),
          lastName: addDraft.lastName.trim(),
          email: addDraft.email.trim(),
          phone: addDraft.phone.trim(),
          cycle: addDraft.cycle,
          classeSouhaitee: addDraft.classeSouhaitee.trim(),
        }),
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Inscription impossible");
      setMessage("Visiteur ajouté au créneau (confirmation e-mail envoyée si SMTP actif).");
      setAddDraft(null);
      await load();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  function openEdit(r: RegistrationRow) {
    setEdit({
      id: r.id,
      firstName: r.firstName,
      lastName: r.lastName,
      email: r.email,
      phone: r.phone || "",
      childFirstName: r.childFirstName || "",
      childLastName: r.childLastName || "",
      cycle: r.cycle && PORTES_OUVERTES_CYCLES.includes(r.cycle) ? r.cycle : availableCycles[0],
      classeSouhaitee: r.classeSouhaitee || "",
      slotId: r.slotId,
    });
  }

  async function submitEdit(e: React.FormEvent) {
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
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Modification impossible");
      setMessage("Inscription mise à jour.");
      setEdit(null);
      await load();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function removeRegistration(r: RegistrationRow) {
    if (!window.confirm(`Supprimer l’inscription de ${visitorLabel(r)} ?`)) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/accueil/portes-ouvertes", {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: r.id, notifyVisitor: true }),
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Suppression impossible");
      setMessage("Inscription annulée.");
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
    try {
      const res = await fetch("/api/accueil/portes-ouvertes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: r.id, visited }),
      });
      const data = (await res.json()) as { error?: string; followUpDelayMinutes?: number };
      if (!res.ok) throw new Error(data.error || "Check-in impossible");
      const delay = data.followUpDelayMinutes ?? board?.followUpDelayMinutes ?? 60;
      setMessage(
        visited
          ? `Visite enregistrée. Suivi prévu dans ~${delay} min.`
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

  function changeVue(next: AccueilPoVue) {
    if (next === "parametrage" && !(board?.canManageParametrage)) {
      setError("Paramétrage réservé à la direction et aux admins.");
      return;
    }
    setVue(next);
    const params = new URLSearchParams(window.location.search);
    const qs = params.toString();
    const base =
      next === "parametrage"
        ? "/accueil/portes-ouvertes/parametrage"
        : "/accueil/portes-ouvertes";
    window.history.replaceState(null, "", qs ? `${base}?${qs}` : base);
  }

  const addClasses = addDraft && board ? board.classesByCycle[addDraft.cycle] || [] : [];
  const editClasses = edit && board ? board.classesByCycle[edit.cycle] || [] : [];
  const editDaySlots = useMemo(() => {
    if (!edit || !board) return [];
    return board.slots
      .filter((s) => slotMatchesCycle(s, edit.cycle))
      .sort((a, b) => a.startAt.localeCompare(b.startAt));
  }, [edit, board]);

  useEffect(() => {
    if (!addDraft || !addClasses.length) return;
    if (!addClasses.includes(addDraft.classeSouhaitee)) {
      setAddDraft({ ...addDraft, classeSouhaitee: addClasses[0] });
    }
  }, [addDraft, addClasses]);

  useEffect(() => {
    if (!edit || !editClasses.length) return;
    if (!editClasses.includes(edit.classeSouhaitee)) {
      setEdit({ ...edit, classeSouhaitee: editClasses[0] });
    }
  }, [edit, editClasses]);

  const staffingSlots = useMemo(() => {
    return (board?.slots || [])
      .filter((s) => !s.isPast)
      .filter((s) => availableCycles.some((c) => slotMatchesCycle(s, c)))
      .sort((a, b) => a.startAt.localeCompare(b.startAt));
  }, [board?.slots, availableCycles]);

  const canParam = Boolean(board?.canManageParametrage);

  useEffect(() => {
    if (!board) return;
    if (vue === "parametrage" && !canParam) {
      setVue("planning");
      const params = new URLSearchParams(window.location.search);
      const qs = params.toString();
      window.history.replaceState(
        null,
        "",
        qs ? `/accueil/portes-ouvertes?${qs}` : "/accueil/portes-ouvertes",
      );
    }
  }, [board, vue, canParam]);

  return (
    <ModulePageShell>
      <ModulePageHeader
        title="Portes ouvertes"
        description={
          vue === "planning"
            ? "Planning du jour style tableur : ajoutez un visiteur dès qu’un parent appelle."
            : "Équipe des créneaux (profs, OGEC, ambassadeurs). La grille horaire se configure dans Événements."
        }
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

      <ModuleTabNav
        tabs={[
          { id: "planning", label: "Planning du jour" },
          { id: "parametrage", label: "Paramétrage", hidden: !canParam },
        ]}
        active={vue}
        onChange={changeVue}
      />

      {loading ? <p className="text-sm text-slate-500">Chargement…</p> : null}
      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
          {error}
        </p>
      ) : null}
      {message ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
          {message}
        </p>
      ) : null}

      {board && vue === "planning" ? (
        <div className="space-y-4">
          {!board.publicEnabled ? (
            <p className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
              Les portes ouvertes ne sont pas marquées activées côté paramétrage — vérifiez Événements.
            </p>
          ) : null}

          <div className="flex flex-wrap items-end gap-3 rounded-2xl border border-slate-200 bg-white p-4">
            <label className="block">
              <span className="text-[11px] font-bold uppercase text-slate-500">Journée</span>
              <select
                className="mt-1 block min-w-[14rem] rounded-xl border border-slate-200 px-3 py-2 text-sm font-semibold"
                value={dayKey}
                onChange={(e) => setDayKey(e.target.value)}
              >
                {dayOptions.length === 0 ? (
                  <option value="">Aucune journée</option>
                ) : (
                  dayOptions.map((k) => (
                    <option key={k} value={k}>
                      {formatSlotDayLabel(k)}
                    </option>
                  ))
                )}
              </select>
            </label>
            <label className="block">
              <span className="text-[11px] font-bold uppercase text-slate-500">Établissement</span>
              <select
                className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-semibold"
                value={cycleFilter}
                onChange={(e) =>
                  setCycleFilter(e.target.value === "all" ? "all" : (e.target.value as PortesOuvertesCycle))
                }
              >
                <option value="all">Tous</option>
                {availableCycles.map((c) => (
                  <option key={c} value={c}>
                    {board.cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                  </option>
                ))}
              </select>
            </label>
            <p className="pb-2 text-sm text-slate-600">
              {daySlots.length} créneau(x)
              {dayKey ? ` — ${formatSlotDayLabel(dayKey)}` : ""}
            </p>
          </div>

          {daySlots.length === 0 ? (
            <p className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
              Aucun créneau pour cette journée. Configurez la grille dans{" "}
              <Link
                href="/etablissement/evenements?tab=portes-ouvertes"
                className="font-semibold underline"
              >
                Événements → Portes ouvertes
              </Link>{" "}
              ou l’onglet Paramétrage.
            </p>
          ) : (
            <div className="overflow-x-auto rounded-2xl border border-slate-200 bg-white shadow-sm">
              <table className="min-w-full border-collapse text-sm">
                <thead>
                  <tr className="bg-slate-50 text-left text-[11px] uppercase tracking-wide text-slate-500">
                    <th className="sticky left-0 z-10 border-b border-r border-slate-200 bg-slate-50 px-3 py-3 font-bold">
                      Départ
                    </th>
                    <th className="border-b border-slate-200 px-3 py-3 font-bold">Visite</th>
                    <th className="border-b border-slate-200 px-3 py-3 font-bold">Places</th>
                    <th className="border-b border-slate-200 px-3 py-3 font-bold">Équipe</th>
                    <th className="border-b border-slate-200 px-3 py-3 font-bold min-w-[22rem]">
                      Visiteurs
                    </th>
                    <th className="border-b border-slate-200 px-3 py-3 font-bold">Ajouter</th>
                  </tr>
                </thead>
                <tbody>
                  {daySlots.map((slot) => {
                    const regs = regsForSlot(slot.id);
                    const staff = (board.staff || []).filter((x) => x.slotId === slot.id);
                    const encadrants = staff.filter(
                      (x) => x.role === "enseignant" || x.role === "personnel",
                    );
                    const ambassadeurs = staff.filter((x) => x.role === "ambassadeur");
                    const rem =
                      cycleFilter === "all"
                        ? slot.remaining
                        : remainingForSlot(slot, cycleFilter);
                    const full = rem === 0;
                    return (
                      <tr key={slot.id} className="align-top border-b border-slate-100">
                        <td className="sticky left-0 z-10 border-r border-slate-100 bg-white px-3 py-3">
                          <div className="font-black text-violet-900">
                            {formatParisTimeLabel(slot.startAt)}
                          </div>
                          <div className="text-xs text-slate-500">{slot.label}</div>
                          {slot.cycle ? (
                            <div className="mt-1 text-[11px] font-semibold text-slate-500">
                              {board.cycleLabels[slot.cycle] ||
                                PORTES_OUVERTES_CYCLE_LABELS[slot.cycle]}
                            </div>
                          ) : null}
                        </td>
                        <td className="px-3 py-3 whitespace-nowrap text-slate-700">
                          {formatParisHm(slot.startAt)} – {formatParisHm(slot.endAt)}
                        </td>
                        <td className="px-3 py-3">
                          <span
                            className={`inline-flex rounded-lg px-2 py-1 text-xs font-bold ${
                              full
                                ? "bg-rose-100 text-rose-800"
                                : "bg-emerald-50 text-emerald-800"
                            }`}
                          >
                            {slot.maxPlaces
                              ? `${slot.registeredCount}/${slot.maxPlaces}`
                              : `${slot.registeredCount}`}
                          </span>
                        </td>
                        <td className="px-3 py-3 text-xs text-slate-600">
                          {encadrants.length === 0 && ambassadeurs.length === 0 ? (
                            <span className="text-slate-400">—</span>
                          ) : (
                            <div className="space-y-1">
                              {encadrants.map((p) => (
                                <div key={p.id}>
                                  <span className="font-semibold text-slate-800">
                                    {p.displayName}
                                  </span>
                                  <span className="ml-1 text-slate-400">
                                    {p.role === "personnel" ? "OGEC" : "prof"}
                                  </span>
                                </div>
                              ))}
                              {ambassadeurs.map((p) => (
                                <div key={p.id}>
                                  <span className="font-semibold text-slate-800">
                                    {p.displayName}
                                  </span>
                                  <span className="ml-1 text-slate-400">élève</span>
                                </div>
                              ))}
                            </div>
                          )}
                        </td>
                        <td className="px-3 py-3">
                          {regs.length === 0 ? (
                            <p className="text-xs text-slate-400">Aucun visiteur</p>
                          ) : (
                            <ul className="space-y-2">
                              {regs.map((r) => (
                                <li
                                  key={r.id}
                                  className="rounded-lg border border-slate-100 bg-slate-50/80 px-2.5 py-2"
                                >
                                  <div className="flex flex-wrap items-start justify-between gap-2">
                                    <div>
                                      <div className="font-semibold text-slate-900">
                                        {visitorLabel(r)}
                                        {r.visitedAt ? (
                                          <span className="ml-1 text-emerald-700">✓</span>
                                        ) : null}
                                      </div>
                                      <div className="text-[11px] text-slate-500">
                                        {[r.classeSouhaitee, r.phone, r.email]
                                          .filter(Boolean)
                                          .join(" · ")}
                                      </div>
                                      {r.childFirstName || r.childLastName ? (
                                        <div className="text-[11px] text-slate-500">
                                          Contact : {r.firstName} {r.lastName}
                                        </div>
                                      ) : null}
                                    </div>
                                    <div className="flex flex-col items-end gap-1">
                                      <label className="flex items-center gap-1 text-[11px] font-semibold text-slate-600">
                                        <input
                                          type="checkbox"
                                          checked={Boolean(r.visitedAt)}
                                          disabled={busy}
                                          onChange={(e) =>
                                            void toggleVisited(r, e.target.checked)
                                          }
                                        />
                                        Visite
                                      </label>
                                      {r.upcoming ? (
                                        <>
                                          <button
                                            type="button"
                                            disabled={busy}
                                            className="text-[11px] font-bold text-violet-700 underline disabled:opacity-50"
                                            onClick={() => openEdit(r)}
                                          >
                                            Modifier
                                          </button>
                                          <button
                                            type="button"
                                            disabled={busy}
                                            className="text-[11px] font-bold text-rose-700 underline disabled:opacity-50"
                                            onClick={() => void removeRegistration(r)}
                                          >
                                            Supprimer
                                          </button>
                                        </>
                                      ) : null}
                                    </div>
                                  </div>
                                </li>
                              ))}
                            </ul>
                          )}
                        </td>
                        <td className="px-3 py-3">
                          <button
                            type="button"
                            disabled={busy || full || Boolean(slot.isPast)}
                            onClick={() => openAdd(slot)}
                            className="rounded-lg bg-violet-700 px-3 py-2 text-xs font-bold text-white disabled:opacity-40"
                          >
                            + Ajouter
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      ) : null}

      {board && vue === "parametrage" && canParam ? (
        <div className="space-y-6">
          <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-3">
            <h2 className="text-lg font-bold text-slate-900">Configuration de la grille</h2>
            <p className="text-sm text-slate-600">
              Jours, horaires, rythme des départs, durée de visite et textes publics se règlent
              dans Événements (admin). Ici : équipe par créneau pour le jour J.
            </p>
            <div className="flex flex-wrap gap-2">
              <Link
                href="/etablissement/evenements?tab=portes-ouvertes"
                className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white"
              >
                Ouvrir le paramétrage complet →
              </Link>
              {board.publicEnabled ? (
                <a
                  href="/portes-ouvertes"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-800"
                >
                  Page publique
                </a>
              ) : null}
            </div>
            <dl className="grid gap-2 sm:grid-cols-2 text-sm">
              <div>
                <dt className="text-xs font-bold uppercase text-slate-500">Titre</dt>
                <dd className="font-semibold text-slate-900">{board.title}</dd>
              </div>
              <div>
                <dt className="text-xs font-bold uppercase text-slate-500">Suivi après visite</dt>
                <dd className="font-semibold text-slate-900">
                  {board.followUpDelayMinutes} min
                </dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="text-xs font-bold uppercase text-slate-500">Adresse</dt>
                <dd className="text-slate-800">{board.address || "—"}</dd>
              </div>
            </dl>
          </section>

          <section className="space-y-4">
            <div>
              <h2 className="text-lg font-bold text-slate-900">Équipe des créneaux</h2>
              <p className="text-sm text-slate-600">
                1–2 encadrants (professeur ou personnel OGEC) et jusqu’à 2 élèves ambassadeurs
                par départ.
              </p>
            </div>
            {staffingSlots.length === 0 ? (
              <p className="text-sm text-slate-500">Aucun créneau à venir.</p>
            ) : (
              staffingSlots.map((s) => {
                const open = staffOpenSlotId === s.id;
                const slotStaff = (board.staff || []).filter((x) => x.slotId === s.id);
                const ambassadeurs = slotStaff.filter((x) => x.role === "ambassadeur");
                const encadrants = slotStaff.filter(
                  (x) => x.role === "enseignant" || x.role === "personnel",
                );
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
                          {formatParisTimeLabel(s.startAt)} · encadrants {encadrants.length}/
                          {PORTES_OUVERTES_MAX_ENCADRANTS} · ambassadeurs {ambassadeurs.length}/
                          {PORTES_OUVERTES_MAX_AMBASSADEURS}
                        </p>
                      </div>
                      <span className="text-xs font-bold text-violet-700">
                        {open ? "Replier" : "Gérer"}
                      </span>
                    </button>
                    {open ? (
                      <div className="p-4 space-y-4">
                        <StaffSearchPicker
                          kind="enseignant"
                          title="Professeur"
                          disabled={busy || encadrants.length >= PORTES_OUVERTES_MAX_ENCADRANTS}
                          onPick={(hit) =>
                            void addStaff({
                              slotId: s.id,
                              role: "enseignant",
                              refId: hit.refId,
                              displayName: hit.displayName,
                              meta: hit.meta,
                            })
                          }
                        />
                        <StaffSearchPicker
                          kind="personnel"
                          title="Personnel OGEC"
                          disabled={busy || encadrants.length >= PORTES_OUVERTES_MAX_ENCADRANTS}
                          onPick={(hit) =>
                            void addStaff({
                              slotId: s.id,
                              role: "personnel",
                              refId: hit.refId,
                              displayName: hit.displayName,
                              meta: hit.meta,
                            })
                          }
                        />
                        <StaffSearchPicker
                          kind="eleve"
                          title="Élève ambassadeur"
                          disabled={busy || ambassadeurs.length >= PORTES_OUVERTES_MAX_AMBASSADEURS}
                          onPick={(hit) =>
                            void addStaff({
                              slotId: s.id,
                              role: "ambassadeur",
                              refId: hit.refId,
                              displayName: hit.displayName,
                              meta: hit.meta,
                            })
                          }
                        />
                        {slotStaff.length === 0 ? (
                          <p className="text-sm text-slate-500">Personne assignée.</p>
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
              })
            )}
          </section>
        </div>
      ) : null}

      {addDraft && board ? (
        <div className="fixed inset-0 z-50 flex items-end justify-center bg-slate-900/40 p-4 sm:items-center">
          <form
            onSubmit={(ev) => void submitAdd(ev)}
            className="w-full max-w-lg rounded-2xl bg-white p-5 shadow-xl space-y-4"
          >
            <div className="flex items-start justify-between gap-3">
              <div>
                <h3 className="text-lg font-bold text-slate-900">Ajouter un visiteur</h3>
                <p className="text-sm text-slate-600">
                  Appel parent / saisie manuelle sur ce créneau.
                </p>
              </div>
              <button
                type="button"
                className="text-sm font-bold text-slate-500"
                onClick={() => setAddDraft(null)}
              >
                Fermer
              </button>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block sm:col-span-2">
                <span className="text-xs font-bold uppercase text-slate-500">Créneau</span>
                <select
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                  value={addDraft.slotId}
                  onChange={(e) => setAddDraft({ ...addDraft, slotId: e.target.value })}
                >
                  {daySlots.map((s) => (
                    <option key={s.id} value={s.id}>
                      {formatParisTimeLabel(s.startAt)} — {s.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Prénom contact</span>
                <input
                  required
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={addDraft.firstName}
                  onChange={(e) => setAddDraft({ ...addDraft, firstName: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Nom contact</span>
                <input
                  required
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={addDraft.lastName}
                  onChange={(e) => setAddDraft({ ...addDraft, lastName: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Téléphone</span>
                <input
                  required
                  type="tel"
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={addDraft.phone}
                  onChange={(e) => setAddDraft({ ...addDraft, phone: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">E-mail</span>
                <input
                  required
                  type="email"
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={addDraft.email}
                  onChange={(e) => setAddDraft({ ...addDraft, email: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Établissement</span>
                <select
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                  value={addDraft.cycle}
                  onChange={(e) =>
                    setAddDraft({
                      ...addDraft,
                      cycle: e.target.value as PortesOuvertesCycle,
                    })
                  }
                >
                  {availableCycles.map((c) => (
                    <option key={c} value={c}>
                      {board.cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Classe souhaitée</span>
                {addClasses.length > 0 ? (
                  <select
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                    value={addDraft.classeSouhaitee}
                    onChange={(e) =>
                      setAddDraft({ ...addDraft, classeSouhaitee: e.target.value })
                    }
                  >
                    {addClasses.map((c) => (
                      <option key={c} value={c}>
                        {c}
                      </option>
                    ))}
                  </select>
                ) : (
                  <input
                    required
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                    value={addDraft.classeSouhaitee}
                    onChange={(e) =>
                      setAddDraft({ ...addDraft, classeSouhaitee: e.target.value })
                    }
                  />
                )}
              </label>
            </div>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700"
                onClick={() => setAddDraft(null)}
              >
                Annuler
              </button>
              <button
                type="submit"
                disabled={busy}
                className="rounded-xl bg-violet-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
              >
                {busy ? "Enregistrement…" : "Enregistrer"}
              </button>
            </div>
          </form>
        </div>
      ) : null}

      {edit && board ? (
        <div className="fixed inset-0 z-50 flex items-end justify-center bg-slate-900/40 p-4 sm:items-center">
          <form
            onSubmit={(ev) => void submitEdit(ev)}
            className="w-full max-w-lg rounded-2xl bg-white p-5 shadow-xl space-y-4"
          >
            <div className="flex items-start justify-between gap-3">
              <h3 className="text-lg font-bold text-slate-900">Modifier l’inscription</h3>
              <button
                type="button"
                className="text-sm font-bold text-slate-500"
                onClick={() => setEdit(null)}
              >
                Fermer
              </button>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Prénom</span>
                <input
                  required
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={edit.firstName}
                  onChange={(e) => setEdit({ ...edit, firstName: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Nom</span>
                <input
                  required
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={edit.lastName}
                  onChange={(e) => setEdit({ ...edit, lastName: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Téléphone</span>
                <input
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={edit.phone}
                  onChange={(e) => setEdit({ ...edit, phone: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">E-mail</span>
                <input
                  required
                  type="email"
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                  value={edit.email}
                  onChange={(e) => setEdit({ ...edit, email: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Établissement</span>
                <select
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                  value={edit.cycle}
                  onChange={(e) =>
                    setEdit({ ...edit, cycle: e.target.value as PortesOuvertesCycle })
                  }
                >
                  {availableCycles.map((c) => (
                    <option key={c} value={c}>
                      {board.cycleLabels[c] || PORTES_OUVERTES_CYCLE_LABELS[c]}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block">
                <span className="text-xs font-bold uppercase text-slate-500">Classe</span>
                {editClasses.length > 0 ? (
                  <select
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
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
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                    value={edit.classeSouhaitee}
                    onChange={(e) => setEdit({ ...edit, classeSouhaitee: e.target.value })}
                  />
                )}
              </label>
              <label className="block sm:col-span-2">
                <span className="text-xs font-bold uppercase text-slate-500">Créneau</span>
                <select
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm font-semibold"
                  value={edit.slotId}
                  onChange={(e) => setEdit({ ...edit, slotId: e.target.value })}
                >
                  {editDaySlots.map((s) => (
                    <option key={s.id} value={s.id}>
                      {formatParisTimeLabel(s.startAt)} — {s.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700"
                onClick={() => setEdit(null)}
              >
                Annuler
              </button>
              <button
                type="submit"
                disabled={busy}
                className="rounded-xl bg-violet-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
              >
                Enregistrer
              </button>
            </div>
          </form>
        </div>
      ) : null}
    </ModulePageShell>
  );
}

function StaffSearchPicker({
  kind,
  title,
  disabled,
  onPick,
}: {
  kind: SearchKind;
  title?: string;
  disabled: boolean;
  onPick: (hit: SearchHit) => void;
}) {
  const [q, setQ] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    const needle = q.trim();
    if (needle.length < 2 || disabled) {
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
  }, [q, kind, disabled]);

  const heading =
    title ||
    (kind === "eleve"
      ? "Ambassadeur (élève)"
      : kind === "enseignant"
        ? "Professeur"
        : "Personnel OGEC");

  return (
    <div className="space-y-1.5">
      <label className="block text-xs font-bold uppercase text-slate-500">{heading}</label>
      <input
        type="search"
        disabled={disabled}
        placeholder={disabled ? "Plafond atteint" : "Rechercher (2 lettres min.)"}
        className="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm disabled:opacity-50"
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />
      {searching ? <p className="text-[11px] text-slate-500">Recherche…</p> : null}
      {results.length > 0 ? (
        <ul className="max-h-40 overflow-auto rounded-xl border border-slate-100 bg-white text-sm">
          {results.map((hit) => (
            <li key={`${hit.refId}-${hit.displayName}`}>
              <button
                type="button"
                disabled={disabled}
                className="w-full px-3 py-2 text-left hover:bg-slate-50 disabled:opacity-50"
                onClick={() => {
                  onPick(hit);
                  setQ("");
                  setResults([]);
                }}
              >
                <span className="font-semibold text-slate-900">{hit.displayName}</span>
                {hit.meta?.classe ? (
                  <span className="ml-2 text-xs text-slate-500">{hit.meta.classe}</span>
                ) : hit.meta?.jobTitle ? (
                  <span className="ml-2 text-xs text-slate-500">{hit.meta.jobTitle}</span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
