"use client";

import { useState } from "react";
import PhotoEditor from "@/app/components/account/PhotoEditor";

type Member = { userId: string; displayName: string; photoUrl: string | null };

const norm = (s: string) => s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

/** Photos de profil du personnel (jamais des élèves), gérées par un administrateur. */
export default function StaffPhotosPanel() {
  const [open, setOpen] = useState(false);
  const [staff, setStaff] = useState<Member[] | null>(null);
  const [search, setSearch] = useState("");
  const [onlyMissing, setOnlyMissing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function toggle() {
    const next = !open;
    setOpen(next);
    if (!next) return;
    setError(null);
    try {
      const res = await fetch("/api/avatars", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible.");
      setStaff(data.staff);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement impossible.");
    }
  }

  const missing = staff?.filter((m) => !m.photoUrl).length ?? 0;
  const visible = (staff ?? []).filter(
    (m) => (!onlyMissing || !m.photoUrl) && norm(m.displayName).includes(norm(search.trim())),
  );

  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="font-bold text-slate-900">Photos du personnel</h2>
          <p className="text-sm text-slate-600">
            Professeurs et personnel uniquement. La photo devient leur photo de profil ; chacun peut la changer
            ou la retirer depuis son compte.
          </p>
        </div>
        <button
          type="button"
          onClick={() => void toggle()}
          aria-expanded={open}
          className="rounded-xl border border-slate-300 px-4 py-2 text-sm font-bold"
        >
          {open ? "Fermer" : "Gérer les photos"}
        </button>
      </div>

      {open && (
        <div className="mt-4">
          {error && (
            <p role="alert" className="text-sm text-rose-700">
              {error}
            </p>
          )}
          {!staff ? (
            !error && <p className="text-sm text-slate-400">Chargement…</p>
          ) : (
            <>
              <div className="flex flex-wrap items-center gap-3">
                <input
                  type="search"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Rechercher un nom…"
                  aria-label="Rechercher une personne"
                  className="min-w-0 flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm"
                />
                <label className="flex items-center gap-2 text-sm">
                  <input type="checkbox" checked={onlyMissing} onChange={(e) => setOnlyMissing(e.target.checked)} />
                  Sans photo ({missing})
                </label>
              </div>
              <ul className="mt-4 grid max-h-[28rem] grid-cols-2 gap-3 overflow-y-auto sm:grid-cols-3 lg:grid-cols-4">
                {visible.map((m) => (
                  <li key={m.userId} className="rounded-xl border border-slate-100 bg-slate-50 p-3 text-center">
                    <PhotoEditor
                      target={m.userId}
                      name={m.displayName}
                      photoUrl={m.photoUrl}
                      size={72}
                      onChange={(photoUrl) =>
                        setStaff(
                          (list) => list?.map((x) => (x.userId === m.userId ? { ...x, photoUrl } : x)) ?? null,
                        )
                      }
                    />
                    <p className="mt-2 break-words text-sm font-medium text-slate-800">{m.displayName}</p>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </section>
  );
}
