"use client";

import { useEffect, useMemo, useState } from "react";
import { dash } from "@/app/lib/dashboard-brand";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

export type ProfRoomBeneficiary = {
  userId?: string;
  firstName: string;
  lastName: string;
  email?: string;
  source: "directory" | "manual";
};

function memberLabel(m: DirectoryMemberOption): string {
  const name = m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim();
  return name || m.email;
}

function norm(s: string) {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

function splitManualName(raw: string): { firstName: string; lastName: string } {
  const parts = raw.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return { firstName: "", lastName: "" };
  if (parts.length === 1) return { firstName: parts[0]!, lastName: parts[0]! };
  return {
    firstName: parts[0]!,
    lastName: parts.slice(1).join(" ").toUpperCase(),
  };
}

type Props = {
  value: ProfRoomBeneficiary | null;
  onChange: (next: ProfRoomBeneficiary | null) => void;
  /** Préremplir la sélection depuis une réservation existante (prénom/nom). */
  matchFirstName?: string;
  matchLastName?: string;
};

export default function ProfRoomBeneficiarySelect({
  value,
  onChange,
  matchFirstName,
  matchLastName,
}: Props) {
  const [members, setMembers] = useState<DirectoryMemberOption[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [mode, setMode] = useState<"directory" | "manual">(
    value?.source === "manual" ? "manual" : "directory",
  );
  const [manualDraft, setManualDraft] = useState(
    value?.source === "manual"
      ? `${value.firstName} ${value.lastName}`.trim()
      : "",
  );

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      setLoading(true);
      setLoadError(null);
      try {
        const res = await fetch("/api/reservation-rooms/directory-users", { cache: "no-store" });
        const j = (await res.json()) as { users?: DirectoryMemberOption[]; error?: string };
        if (!res.ok) throw new Error(j.error || "Chargement impossible");
        if (!cancelled) {
          setMembers((j.users || []).filter((u) => u.externalUserId && !u.pending));
        }
      } catch (e) {
        if (!cancelled) {
          setLoadError(e instanceof Error ? e.message : "Erreur de chargement");
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Rematch réservation existante → personne de l’annuaire.
  useEffect(() => {
    if (!members.length || value?.userId) return;
    const fn = (matchFirstName || value?.firstName || "").trim();
    const ln = (matchLastName || value?.lastName || "").trim();
    if (!fn && !ln) return;
    const hit = members.find((m) => {
      const mFn = (m.firstName || "").trim().toLowerCase();
      const mLn = (m.lastName || "").trim().toLowerCase();
      return (
        mFn === fn.toLowerCase() &&
        mLn === ln.toLowerCase()
      );
    });
    if (hit) {
      onChange({
        userId: hit.externalUserId,
        firstName: (hit.firstName || "").trim() || memberLabel(hit).split(/\s+/)[0] || "",
        lastName: ((hit.lastName || "").trim() || memberLabel(hit)).toUpperCase(),
        email: hit.email,
        source: "directory",
      });
      setMode("directory");
    } else if (fn || ln) {
      setMode("manual");
      setManualDraft(`${fn} ${ln}`.trim());
      if (!value || value.source !== "manual") {
        onChange({
          firstName: fn,
          lastName: ln.toUpperCase(),
          source: "manual",
        });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- rematch une fois l’annuaire prêt
  }, [members]);

  const filtered = useMemo(() => {
    const q = norm(search.trim());
    const list = [...members].sort((a, b) => {
      const la = (a.lastName || memberLabel(a)).localeCompare(b.lastName || memberLabel(b), "fr");
      if (la !== 0) return la;
      return (a.firstName || "").localeCompare(b.firstName || "", "fr");
    });
    if (!q) return list;
    return list.filter((m) => {
      const blob = norm(`${memberLabel(m)} ${m.email} ${m.firstName ?? ""} ${m.lastName ?? ""}`);
      return blob.includes(q);
    });
  }, [members, search]);

  const selectMember = (m: DirectoryMemberOption) => {
    const firstName =
      (m.firstName || "").trim() || memberLabel(m).split(/\s+/)[0] || "";
    const lastName = ((m.lastName || "").trim() || memberLabel(m)).toUpperCase();
    onChange({
      userId: m.externalUserId,
      firstName,
      lastName,
      email: m.email,
      source: "directory",
    });
    setMode("directory");
  };

  const applyManual = (raw: string) => {
    setManualDraft(raw);
    const { firstName, lastName } = splitManualName(raw);
    if (!firstName.trim() || !lastName.trim()) {
      onChange(null);
      return;
    }
    onChange({
      firstName: firstName.trim(),
      lastName: lastName.trim().toUpperCase(),
      source: "manual",
    });
  };

  if (loading) {
    return <p className={`text-sm ${dash.textMid}`}>Chargement du personnel…</p>;
  }

  if (loadError) {
    return (
      <div className="space-y-3">
        <p className={`text-sm text-rose-700`}>Annuaire indisponible : {loadError}</p>
        <label className="block min-w-0">
          <span className={`mb-1.5 block ${dash.fieldLabel}`}>Saisie manuelle (prénom NOM)</span>
          <input
            type="text"
            value={manualDraft}
            onChange={(e) => applyManual(e.target.value)}
            placeholder="ex. Marie DUPONT"
            className={`w-full rounded-xl border bg-white/80 px-3 py-2.5 text-sm font-semibold outline-none ${dash.borderSoft} ${dash.ink} ${dash.focusBorder}`}
          />
        </label>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => setMode("directory")}
          className={`rounded-lg border px-3 py-1.5 text-xs font-semibold transition ${
            mode === "directory"
              ? "border-[color:var(--dash-primary)] bg-[color:var(--dash-primary)]/10 text-[var(--dash-ink)]"
              : `${dash.borderSoft} bg-white/70 ${dash.textMid}`
          }`}
        >
          Personnel
        </button>
        <button
          type="button"
          onClick={() => setMode("manual")}
          className={`rounded-lg border px-3 py-1.5 text-xs font-semibold transition ${
            mode === "manual"
              ? "border-amber-300 bg-amber-50 text-amber-900"
              : `${dash.borderSoft} bg-white/70 ${dash.textMid}`
          }`}
        >
          Autre (saisie libre)
        </button>
      </div>

      {value ? (
        <div
          className={`flex flex-wrap items-center gap-2 rounded-xl border px-3 py-2 text-sm ${
            value.source === "manual"
              ? "border-amber-200 bg-amber-50 text-amber-950"
              : "border-[color:var(--dash-primary)]/30 bg-[color:var(--dash-primary)]/5"
          } ${dash.ink}`}
        >
          <span className="font-semibold">
            {value.firstName} {value.lastName}
          </span>
          {value.email ? <span className={`text-xs ${dash.textMid}`}>{value.email}</span> : null}
          <button
            type="button"
            onClick={() => {
              onChange(null);
              setManualDraft("");
            }}
            className={`ml-auto text-xs font-semibold underline ${dash.textMid}`}
          >
            Changer
          </button>
        </div>
      ) : null}

      {mode === "directory" ? (
        <>
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Rechercher un collègue…"
            className={`w-full rounded-xl border bg-white/80 px-3 py-2.5 text-sm font-semibold outline-none ${dash.borderSoft} ${dash.ink} ${dash.focusBorder}`}
          />
          <div
            className={`max-h-56 divide-y overflow-y-auto rounded-xl border bg-white/70 ${dash.borderSoft} ${dash.divider}`}
          >
            {filtered.length === 0 ? (
              <p className={`p-4 text-sm italic ${dash.textMid}`}>Aucune personne trouvée.</p>
            ) : (
              filtered.map((m) => {
                const selected = value?.userId === m.externalUserId;
                return (
                  <button
                    key={m.externalUserId}
                    type="button"
                    onClick={() => selectMember(m)}
                    className={`flex w-full items-center gap-3 px-4 py-2.5 text-left text-sm transition ${dash.hoverBgSoft} ${
                      selected ? "bg-[color:var(--dash-primary)]/10" : ""
                    }`}
                  >
                    <span
                      className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-full border ${
                        selected
                          ? "border-[color:var(--dash-primary)] bg-[color:var(--dash-primary)] text-white"
                          : dash.borderSoft
                      }`}
                      aria-hidden
                    >
                      {selected ? "✓" : null}
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className={`block truncate font-semibold ${dash.ink}`}>
                        {memberLabel(m)}
                      </span>
                      <span className={`block truncate text-xs ${dash.textMid}`}>{m.email}</span>
                    </span>
                  </button>
                );
              })
            )}
          </div>
        </>
      ) : (
        <label className="block min-w-0">
          <span className={`mb-1.5 block ${dash.fieldLabel}`}>Prénom et nom</span>
          <input
            type="text"
            value={manualDraft}
            onChange={(e) => applyManual(e.target.value)}
            placeholder="ex. Marie DUPONT"
            className={`w-full rounded-xl border bg-white/80 px-3 py-2.5 text-sm font-semibold outline-none ${dash.borderSoft} ${dash.ink} ${dash.focusBorder}`}
          />
          <span className={`mt-1 block text-xs ${dash.textMid}`}>
            À utiliser uniquement si la personne n’est pas dans l’annuaire.
          </span>
        </label>
      )}
    </div>
  );
}
