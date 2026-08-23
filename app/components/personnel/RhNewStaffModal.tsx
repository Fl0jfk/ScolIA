"use client";

import { useEffect, useMemo, useState } from "react";
import RhStaffProfileFields from "@/app/components/personnel/RhStaffProfileFields";
import { profileFromFormData } from "@/app/lib/personnel-profile";
import { PERSONNEL_CATEGORY_OPTIONS, type PersonnelCategory } from "@/app/lib/personnel-types";

type DirectoryCandidate = {
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  roles: string[];
  roleLabel: string;
  suggestedCategory: PersonnelCategory;
  pending: boolean;
};

type Props = {
  open: boolean;
  onClose: () => void;
  onCreated: (recordId: string) => void;
};

export default function RhNewStaffModal({ open, onClose, onCreated }: Props) {
  const [candidates, setCandidates] = useState<DirectoryCandidate[]>([]);
  const [loadingCandidates, setLoadingCandidates] = useState(false);
  const [directorySearch, setDirectorySearch] = useState("");
  const [showDirectoryPicker, setShowDirectoryPicker] = useState(false);
  const [linkedDirectoryUser, setLinkedDirectoryUser] = useState<DirectoryCandidate | null>(null);

  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [jobTitle, setJobTitle] = useState("");
  const [hireDate, setHireDate] = useState("");
  const [category, setCategory] = useState<PersonnelCategory>("administratif");
  const [createDirectoryUser, setCreateDirectoryUser] = useState(true);

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const resetForm = () => {
    setDirectorySearch("");
    setShowDirectoryPicker(false);
    setLinkedDirectoryUser(null);
    setFirstName("");
    setLastName("");
    setEmail("");
    setJobTitle("");
    setHireDate("");
    setCategory("administratif");
    setCreateDirectoryUser(true);
    setError(null);
  };

  useEffect(() => {
    if (!open) return;
    resetForm();
    setLoadingCandidates(true);
    void fetch("/api/personnel/directory-candidates", { cache: "no-store" })
      .then(async (res) => {
        const j = await res.json();
        if (!res.ok) throw new Error(j.error || "Chargement impossible");
        setCandidates(j.candidates || []);
      })
      .catch((e) => setError(e instanceof Error ? e.message : "Erreur"))
      .finally(() => setLoadingCandidates(false));
  }, [open]);

  const filteredDirectory = useMemo(() => {
    const q = directorySearch.trim().toLowerCase();
    if (!q) return candidates.slice(0, 12);
    return candidates
      .filter((c) => {
        const hay = `${c.displayName || ""} ${c.email} ${c.roleLabel}`.toLowerCase();
        return hay.includes(q);
      })
      .slice(0, 12);
  }, [candidates, directorySearch]);

  const applyDirectoryLink = (c: DirectoryCandidate) => {
    setLinkedDirectoryUser(c);
    setFirstName(c.firstName || "");
    setLastName(c.lastName || "");
    setEmail(c.email);
    setCategory(c.suggestedCategory);
    setCreateDirectoryUser(false);
    setShowDirectoryPicker(false);
    setDirectorySearch("");
  };

  const clearDirectoryLink = () => {
    setLinkedDirectoryUser(null);
    setCreateDirectoryUser(true);
  };

  const onEmailBlur = () => {
    const normalized = email.trim().toLowerCase();
    if (!normalized || linkedDirectoryUser) return;
    const match = candidates.find((c) => c.email.toLowerCase() === normalized);
    if (match) applyDirectoryLink(match);
  };

  const submit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    const fd = new FormData(e.currentTarget);

    const payload: Record<string, unknown> = {
      firstName: fd.get("firstName"),
      lastName: fd.get("lastName"),
      email: fd.get("email"),
      category: fd.get("category"),
      jobTitle: fd.get("jobTitle") || undefined,
      hireDate: fd.get("hireDate") || null,
      withOnboarding: true,
      profile: profileFromFormData(fd),
    };

    if (linkedDirectoryUser) {
      payload.mode = "link-directory";
      payload.externalUserId = linkedDirectoryUser.externalUserId || undefined;
      payload.email = linkedDirectoryUser.email;
    } else {
      payload.mode = "create";
      payload.createDirectoryUserUser = createDirectoryUser;
    }

    try {
      const res = await fetch("/api/personnel", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Création impossible");
      onCreated(j.record.id);
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-sm">
      <div className="bg-white rounded-2xl shadow-2xl max-w-3xl w-full max-h-[92vh] overflow-hidden flex flex-col">
        <div className="p-6 border-b border-slate-100 shrink-0">
          <h2 className="text-lg font-black text-slate-900">Nouveau collaborateur</h2>
          <p className="text-xs text-slate-500 mt-1">
            Un seul formulaire : associez un compte existant si possible, sinon création d&apos;une invitation.
          </p>
        </div>

        <form id="rh-new-staff" onSubmit={submit} className="p-6 overflow-y-auto flex-1 space-y-6">
          {error && <p className="text-sm text-rose-600 font-medium">{error}</p>}

          <section className="rounded-xl border border-indigo-100 bg-indigo-50/40 p-4 space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="text-xs font-black uppercase tracking-widest text-indigo-800">Compte</p>
              {!linkedDirectoryUser && (
                <button
                  type="button"
                  onClick={() => setShowDirectoryPicker((v) => !v)}
                  className="text-xs font-bold text-indigo-600 underline"
                >
                  {showDirectoryPicker ? "Masquer la liste" : `Associer depuis l’annuaire (${candidates.length})`}
                </button>
              )}
            </div>

            {linkedDirectoryUser ? (
              <div className="flex items-start justify-between gap-3 rounded-xl bg-white border border-emerald-200 p-3">
                <div>
                  <p className="text-sm font-bold text-emerald-800">Compte associé</p>
                  <p className="text-sm text-slate-800 mt-0.5">{linkedDirectoryUser.displayName || linkedDirectoryUser.email}</p>
                  <p className="text-xs text-slate-500">{linkedDirectoryUser.email}</p>
                  <p className="text-[10px] font-bold text-indigo-600 mt-1 uppercase">
                    {linkedDirectoryUser.roleLabel}
                    {linkedDirectoryUser.pending && " · invitation en attente"}
                  </p>
                  <p className="text-xs text-emerald-700 mt-2">Aucun doublon ne sera créé.</p>
                </div>
                <button
                  type="button"
                  onClick={clearDirectoryLink}
                  className="text-xs font-bold text-slate-500 underline shrink-0"
                >
                  Dissocier
                </button>
              </div>
            ) : showDirectoryPicker ? (
              <div className="space-y-2">
                <input
                  type="search"
                  value={directorySearch}
                  onChange={(e) => setDirectorySearch(e.target.value)}
                  placeholder="Rechercher dans le directory…"
                  className="w-full border rounded-xl p-2.5 text-sm bg-white"
                />
                {loadingCandidates ? (
                  <p className="text-xs text-slate-400 text-center py-3">Chargement…</p>
                ) : filteredDirectory.length === 0 ? (
                  <p className="text-xs text-slate-400 italic text-center py-3">
                    Aucun compte disponible sans dossier RH.
                  </p>
                ) : (
                  <ul className="space-y-1.5 max-h-40 overflow-y-auto">
                    {filteredDirectory.map((c) => (
                      <li key={c.externalUserId || c.email}>
                        <button
                          type="button"
                          onClick={() => applyDirectoryLink(c)}
                          className="w-full text-left rounded-lg border border-slate-200 bg-white px-3 py-2 hover:border-indigo-300 transition"
                        >
                          <p className="font-bold text-sm">{c.displayName || c.email}</p>
                          <p className="text-[10px] text-slate-500">{c.email} · {c.roleLabel}</p>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            ) : (
              <p className="text-xs text-slate-600">
                Saisissez l&apos;email ci-dessous : s&apos;il existe dans le directory, il sera associé automatiquement.
              </p>
            )}
          </section>

          <section>
            <h4 className="text-xs font-black uppercase tracking-widest text-slate-500 mb-3">Identité & contact pro</h4>
            <div className="grid sm:grid-cols-2 gap-3">
              <input
                name="firstName"
                required
                value={firstName}
                onChange={(e) => setFirstName(e.target.value)}
                placeholder="Prénom *"
                className="w-full border rounded-xl p-3 text-sm"
              />
              <input
                name="lastName"
                required
                value={lastName}
                onChange={(e) => setLastName(e.target.value)}
                placeholder="Nom *"
                className="w-full border rounded-xl p-3 text-sm"
              />
              <input
                name="email"
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                onBlur={onEmailBlur}
                placeholder="Email professionnel *"
                className="w-full border rounded-xl p-3 text-sm sm:col-span-2"
              />
              <input
                name="jobTitle"
                value={jobTitle}
                onChange={(e) => setJobTitle(e.target.value)}
                placeholder="Poste / fonction"
                className="w-full border rounded-xl p-3 text-sm"
              />
              <input
                name="hireDate"
                type="date"
                value={hireDate}
                onChange={(e) => setHireDate(e.target.value)}
                className="w-full border rounded-xl p-3 text-sm"
              />
              <select
                name="category"
                value={category}
                onChange={(e) => setCategory(e.target.value as PersonnelCategory)}
                className="w-full border rounded-xl p-3 text-sm sm:col-span-2"
              >
                {PERSONNEL_CATEGORY_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </div>

            {!linkedDirectoryUser && (
              <label className="flex items-start gap-2 text-sm text-slate-600 mt-3">
                <input
                  type="checkbox"
                  checked={createDirectoryUser}
                  onChange={(e) => setCreateDirectoryUser(e.target.checked)}
                  className="mt-1"
                />
                <span>
                  Créer un compte si l&apos;email n&apos;existe pas encore
                  <span className="block text-xs text-slate-400">
                    Une invitation sera envoyée uniquement pour un nouvel email.
                  </span>
                </span>
              </label>
            )}
          </section>

          <RhStaffProfileFields />
        </form>

        <div className="p-6 border-t border-slate-100 flex gap-2 shrink-0">
          <button type="button" onClick={onClose} className="flex-1 py-2.5 rounded-xl border font-bold text-sm text-slate-600">
            Annuler
          </button>
          <button
            type="submit"
            form="rh-new-staff"
            disabled={busy}
            className="flex-1 py-2.5 rounded-xl bg-indigo-600 text-white font-bold text-sm disabled:opacity-50"
          >
            {busy ? "Création…" : linkedDirectoryUser ? "Créer le dossier (compte associé)" : "Créer le dossier"}
          </button>
        </div>
      </div>
    </div>
  );
}
