"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  SettingsLoading,
  SettingsNotice,
  SettingsSection,
  settingsInputClass,
  settingsSelectClass,
} from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

type RoleOption = { slug: string; label: string };

type RegistryUserRow = {
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  roles: string[];
  pending?: boolean;
  createdAt: string;
  updatedAt: string;
};

type SortKey = "name" | "email" | "role";

function norm(s: string) {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function RoleCheckboxes({
  options,
  selected,
  onChange,
}: {
  options: RoleOption[];
  selected: string[];
  onChange: (roles: string[]) => void;
}) {
  const toggle = (slug: string) => {
    if (selected.includes(slug)) onChange(selected.filter((r) => r !== slug));
    else onChange([...selected, slug]);
  };
  return (
    <div className="flex flex-wrap gap-2">
      {options.map((opt) => {
        const on = selected.includes(opt.slug);
        return (
          <button
            key={opt.slug}
            type="button"
            onClick={() => toggle(opt.slug)}
            className={`px-3 py-1.5 rounded-full text-xs font-semibold border transition ${
              on
                ? "bg-[var(--dash-primary)] text-white border-[var(--dash-primary)]"
                : "bg-white/70 text-slate-600 border-white/80 hover:border-[color:var(--dash-primary)]/35"
            }`}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

function labelFor(u: RegistryUserRow) {
  return u.displayName || `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim() || u.email;
}

/** Gestion utilisateurs du directory — onglet Paramètres généraux. */
export default function MembresPanel() {
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [users, setUsers] = useState<RegistryUserRow[]>([]);
  const [roleOptions, setRoleOptions] = useState<RoleOption[]>([]);
  const [email, setEmail] = useState("");
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [inviteRoles, setInviteRoles] = useState<string[]>([]);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [editRoles, setEditRoles] = useState<string[]>([]);
  const [savingEdit, setSavingEdit] = useState(false);
  const [invitingEmail, setInvitingEmail] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [roleFilter, setRoleFilter] = useState("");
  const [sortBy, setSortBy] = useState<SortKey>("name");
  const [showAddForm, setShowAddForm] = useState(false);

  const load = useCallback(async () => {
    setError(null);
    try {
      const res = await fetch("/api/members");
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Chargement impossible");
      setUsers(j.users ?? []);
      setRoleOptions(j.roleOptions ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const roleLabels = useCallback(
    (slugs: string[]) => slugs.map((s) => roleOptions.find((o) => o.slug === s)?.label ?? s).join(", ") || "—",
    [roleOptions],
  );

  const filteredUsers = useMemo(() => {
    const q = norm(search.trim());
    let list = [...users];
    if (q) {
      list = list.filter((u) => {
        const blob = norm(
          [labelFor(u), u.email, u.firstName, u.lastName, u.externalUserId, ...u.roles, roleLabels(u.roles)].join(" "),
        );
        return blob.includes(q);
      });
    }
    if (roleFilter) {
      list = list.filter((u) => u.roles.includes(roleFilter));
    }
    list.sort((a, b) => {
      if (sortBy === "email") return a.email.localeCompare(b.email, "fr");
      if (sortBy === "role") {
        const ra = a.roles[0] ?? "";
        const rb = b.roles[0] ?? "";
        return ra.localeCompare(rb, "fr") || labelFor(a).localeCompare(labelFor(b), "fr");
      }
      return labelFor(a).localeCompare(labelFor(b), "fr", { sensitivity: "base" });
    });
    return list;
  }, [users, search, roleFilter, sortBy, roleLabels]);

  const sendInvite = async (e: React.FormEvent) => {
    e.preventDefault();
    if (inviteRoles.length === 0) {
      setError("Choisissez au moins un rôle.");
      return;
    }
    setSending(true);
    setError(null);
    try {
      const res = await fetch("/api/members", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, firstName, lastName, intranetRoles: inviteRoles }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec");
      setEmail("");
      setFirstName("");
      setLastName("");
      setInviteRoles([]);
      setShowAddForm(false);
      alert(j.message || "Enregistré.");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setSending(false);
    }
  };

  const startEdit = (u: RegistryUserRow) => {
    setEditingKey(u.externalUserId || u.email);
    setEditRoles([...u.roles]);
  };

  const saveEdit = async (u: RegistryUserRow) => {
    if (editRoles.length === 0) {
      setError("Sélectionnez au moins un rôle.");
      return;
    }
    setSavingEdit(true);
    setError(null);
    try {
      const res = await fetch("/api/members", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          externalUserId: u.externalUserId || undefined,
          email: u.email,
          intranetRoles: editRoles,
        }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec");
      setEditingKey(null);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setSavingEdit(false);
    }
  };

  const removeUser = async (u: RegistryUserRow) => {
    if (!confirm(`Supprimer ${labelFor(u)} (${u.email}) ? Le compte ne pourra plus se connecter.`)) return;
    try {
      const q = new URLSearchParams();
      if (u.externalUserId) q.set("externalUserId", u.externalUserId);
      else q.set("email", u.email);
      const res = await fetch(`/api/members?${q}`, { method: "DELETE" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    }
  };

  const sendInvitationLink = async (u: RegistryUserRow) => {
    const name = labelFor(u);
    if (
      !confirm(
        `Envoyer un lien d’invitation à ${name} (${u.email}) ?\n\n` +
          `La personne reçoit un e-mail pour créer son mot de passe (lien valable 1 h), puis active la double authentification à la première connexion.\n` +
          `Un éventuel ancien mot de passe ne fonctionnera plus.`,
      )
    ) {
      return;
    }
    setInvitingEmail(u.email);
    setError(null);
    setSuccess(null);
    try {
      const res = await fetch("/api/admin/auth/send-activation", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: u.email }),
      });
      const j = (await res.json()) as { ok?: boolean; error?: string; message?: string };
      if (!res.ok || !j.ok) throw new Error(j.error || "Envoi impossible");
      setSuccess(j.message || `Lien d’invitation envoyé à ${u.email}.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setInvitingEmail(null);
    }
  };

  if (loading) return <SettingsLoading label="Chargement des utilisateurs…" />;

  return (
    <div className="space-y-6">
      <p className={`text-sm ${dash.textMid}`}>
        Gestion directe via Better-Auth (invitations, rôles, suppression). Une instance = une instance auth.
      </p>

      <SettingsSection>
        <h3 className={`text-sm font-semibold ${dash.ink}`}>Lien d’invitation</h3>
        <p className={`mt-1 text-sm ${dash.textMid}`}>
          Sur chaque personne, utilisez <strong>Lien d’invitation</strong> pour lui envoyer un e-mail
          d’accès. Elle crée son mot de passe (lien valable 1&nbsp;heure), se connecte, puis active la
          double authentification (MFA). Si le compte est déjà activé (MFA en place), le bouton
          n’envoie pas de nouveau lien — la personne se connecte normalement.
        </p>
      </SettingsSection>

      {error ? <SettingsNotice tone="error">{error}</SettingsNotice> : null}
      {success ? <SettingsNotice tone="ok">{success}</SettingsNotice> : null}

      <SettingsSection className="sticky top-2 z-10">
        <div className="flex flex-wrap gap-3 items-end">
          <label className="flex-1 min-w-[200px]">
            <span className="text-xs font-bold text-slate-500 uppercase tracking-wide">Rechercher</span>
            <input
              type="search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Nom, prénom, e-mail…"
              className={`${settingsInputClass} mt-1`}
            />
          </label>
          <label className="min-w-[160px]">
            <span className="text-xs font-bold text-slate-500 uppercase tracking-wide">Rôle</span>
            <select
              value={roleFilter}
              onChange={(e) => setRoleFilter(e.target.value)}
              className={`${settingsSelectClass} mt-1`}
            >
              <option value="">Tous les rôles</option>
              {roleOptions.map((o) => (
                <option key={o.slug} value={o.slug}>
                  {o.label}
                </option>
              ))}
            </select>
          </label>
          <label className="min-w-[140px]">
            <span className="text-xs font-bold text-slate-500 uppercase tracking-wide">Trier</span>
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as SortKey)}
              className={`${settingsSelectClass} mt-1`}
            >
              <option value="name">Nom</option>
              <option value="email">E-mail</option>
              <option value="role">Rôle principal</option>
            </select>
          </label>
        </div>
        <p className={`text-xs ${dash.textMid}`}>
          {filteredUsers.length} affiché{filteredUsers.length > 1 ? "s" : ""} sur {users.length}
        </p>
      </SettingsSection>

      <div className="flex justify-between items-center">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>Liste</h2>
        <button
          type="button"
          onClick={() => setShowAddForm((v) => !v)}
          className={`text-sm font-semibold ${dash.textPrimary} hover:underline`}
        >
          {showAddForm ? "Masquer le formulaire" : "+ Ajouter"}
        </button>
      </div>

      {showAddForm && (
        <form onSubmit={sendInvite} className="relative overflow-hidden rounded-[1.5rem] border border-white/55 bg-white/55 p-6 space-y-4 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-2xl">
          <h3 className="font-bold text-slate-800">Nouvel utilisateur</h3>
          <div className="grid gap-3 sm:grid-cols-2">
            <label>
              <span className="text-sm font-bold text-slate-600">Prénom</span>
              <input
                value={firstName}
                onChange={(e) => setFirstName(e.target.value)}
                className={settingsInputClass}
              />
            </label>
            <label>
              <span className="text-sm font-bold text-slate-600">Nom</span>
              <input
                value={lastName}
                onChange={(e) => setLastName(e.target.value)}
                className={settingsInputClass}
              />
            </label>
          </div>
          <label className="block">
            <span className="text-sm font-bold text-slate-600">E-mail *</span>
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className={settingsInputClass}
            />
          </label>
          <div>
            <span className="text-sm font-bold text-slate-600 block mb-2">Rôles *</span>
            <RoleCheckboxes options={roleOptions} selected={inviteRoles} onChange={setInviteRoles} />
          </div>
          <button
            type="submit"
            disabled={sending}
            className="px-6 py-3 rounded-2xl bg-[var(--dash-primary)] text-white font-semibold disabled:opacity-50"
          >
            {sending ? "En cours…" : "Enregistrer"}
          </button>
        </form>
      )}

      {filteredUsers.length === 0 ? (
        <p className="text-sm text-slate-500 text-center py-8">Aucun résultat pour cette recherche.</p>
      ) : (
        <ul className="space-y-2 max-h-[60vh] overflow-y-auto pr-1">
          {filteredUsers.map((u) => {
            const key = u.externalUserId || u.email;
            const name = labelFor(u);
            return (
              <li key={key} className="rounded-2xl border border-white/55 bg-white/55 p-4 backdrop-blur-xl hover:bg-white/75">
                <div className="flex flex-wrap justify-between gap-2">
                  <div className="min-w-0">
                    <p className="font-bold text-slate-900 truncate">{name}</p>
                    <p className="text-sm text-slate-500 truncate">{u.email}</p>
                    {editingKey !== key && (
                      <p className={`mt-1 text-xs font-medium ${dash.textPrimary}`}>{roleLabels(u.roles)}</p>
                    )}
                    {u.pending && <span className="text-xs text-amber-600 font-bold">Invitation en attente</span>}
                  </div>
                  <div className="flex flex-wrap gap-3 shrink-0">
                    {editingKey !== key && (
                      <>
                        <button
                          type="button"
                          onClick={() => void sendInvitationLink(u)}
                          disabled={invitingEmail === u.email}
                          className={`text-sm font-semibold ${dash.textPrimary} disabled:opacity-50`}
                        >
                          {invitingEmail === u.email ? "Envoi…" : "Lien d’invitation"}
                        </button>
                        <button type="button" onClick={() => startEdit(u)} className={`text-sm font-semibold ${dash.textPrimary}`}>
                          Modifier
                        </button>
                        <button type="button" onClick={() => removeUser(u)} className="text-sm font-bold text-red-600">
                          Supprimer
                        </button>
                      </>
                    )}
                  </div>
                </div>
                {editingKey === key && (
                  <div className="mt-4 pt-4 border-t space-y-3">
                    <RoleCheckboxes options={roleOptions} selected={editRoles} onChange={setEditRoles} />
                    <div className="flex gap-2">
                      <button
                        type="button"
                        onClick={() => saveEdit(u)}
                        disabled={savingEdit}
                        className="px-4 py-2 rounded-xl bg-[var(--dash-primary)] text-white text-sm font-semibold"
                      >
                        Enregistrer
                      </button>
                      <button
                        type="button"
                        onClick={() => setEditingKey(null)}
                        className="px-4 py-2 rounded-xl bg-slate-100 text-sm font-bold"
                      >
                        Annuler
                      </button>
                    </div>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
