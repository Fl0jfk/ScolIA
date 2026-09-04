"use client";

import { useEffect, useMemo, useState } from "react";
import { DEFAULT_PROF_ROOM_SUBJECT_COLORS } from "@/app/lib/prof-room-defaults";
import { PROF_ROOM_COLOR_PRESETS } from "@/app/lib/prof-room-subject-colors";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ProfRoomAdminPicker, { type DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import ProfRoomGlassCard from "@/app/components/prof-room/ProfRoomGlassCard";
import { dash } from "@/app/lib/dashboard-brand";
import SubjectColorEditor from "./SubjectColorEditor";

type Room = {
  id: string;
  name: string;
  building?: string;
  kind?: "facility" | "classroom";
  bookable?: boolean;
};

type ModuleConfig = {
  classesByPole: Record<string, string[]>;
  subjectColors: Record<string, string>;
  bookingHorizonDays: number;
};

function slugifyRoomId(name: string): string {
  const base =
    name
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || `salle-${Date.now()}`;
  return base;
}

function uniqueRoomId(name: string, rooms: Room[], skipIdx?: number): string {
  const base = slugifyRoomId(name);
  const used = new Set(
    rooms.filter((_, i) => i !== skipIdx).map((r) => r.id),
  );
  if (!used.has(base)) return base;
  let n = 2;
  while (used.has(`${base}-${n}`)) n++;
  return `${base}-${n}`;
}

export default function ProfRoomSettingsTab() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [rooms, setRooms] = useState<Room[]>([]);
  const [config, setConfig] = useState<ModuleConfig>({
    classesByPole: {},
    subjectColors: {},
    bookingHorizonDays: 56,
  });
  const [newSubject, setNewSubject] = useState("");
  const [newSubjectColor, setNewSubjectColor] = useState(PROF_ROOM_COLOR_PRESETS[0].value);
  const [newPoleName, setNewPoleName] = useState("");
  const [newClassByPole, setNewClassByPole] = useState<Record<string, string>>({});
  const [adminExternalUserIds, setAdminExternalUserIds] = useState<string[]>([]);
  const [directoryMembers, setDirectoryMembers] = useState<DirectoryMemberOption[]>([]);
  const [membersLoading, setMembersLoading] = useState(false);

  const selectedAdmins = useMemo(
    () =>
      adminExternalUserIds
        .map((id) => directoryMembers.find((m) => m.externalUserId === id))
        .filter((m): m is DirectoryMemberOption => Boolean(m)),
    [adminExternalUserIds, directoryMembers],
  );

  useEffect(() => {
    (async () => {
      try {
        const [roomsRes, configRes, usersRes] = await Promise.all([
          fetch("/api/reservation-rooms/rooms"),
          fetch("/api/reservation-rooms/module-config"),
          fetch("/api/reservation-rooms/directory-users"),
        ]);
        const roomsJson = await roomsRes.json();
        const configJson = await configRes.json();
        const usersJson = await usersRes.json();
        if (!roomsRes.ok) throw new Error(roomsJson.error || "Salles introuvables");
        if (!configRes.ok) throw new Error(configJson.error || "Configuration introuvable");
        setRooms(roomsJson.rooms || []);
        const loaded = configJson.config || { classesByPole: {}, subjectColors: {}, bookingHorizonDays: 56 };
        setConfig({
          ...loaded,
          subjectColors: { ...DEFAULT_PROF_ROOM_SUBJECT_COLORS, ...loaded.subjectColors },
        });
        setAdminExternalUserIds(
          Array.isArray(configJson.adminExternalUserIds) ? configJson.adminExternalUserIds : [],
        );
        if (usersRes.ok) {
          setDirectoryMembers((usersJson.users || []) as DirectoryMemberOption[]);
        } else {
          setError(usersJson.error || "Impossible de charger les utilisateurs du directory.");
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Erreur de chargement");
      } finally {
        setLoading(false);
        setMembersLoading(false);
      }
    })();
  }, []);

  const saveRooms = async () => {
    setSaving(true);
    setError(null);
    try {
      const normalized = rooms
        .filter((room) => room.name.trim())
        .map((room, idx) => {
          const name = room.name.trim();
          const isNew = !room.id || /^salle-\d+$/.test(room.id);
          const kind: "facility" | "classroom" =
            room.kind === "classroom" ? "classroom" : "facility";
          return {
            ...room,
            name,
            id: isNew ? uniqueRoomId(name, rooms, idx) : room.id.trim(),
            kind,
            bookable:
              typeof room.bookable === "boolean" ? room.bookable : kind !== "classroom",
          };
        });
      const res = await fetch("/api/reservation-rooms/rooms", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ rooms: normalized }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec enregistrement salles");
      setRooms(normalized);
      alert("Salles enregistrées.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const saveAdmins = async () => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/reservation-rooms/module-config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ adminExternalUserIds }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec enregistrement administrateurs");
      if (Array.isArray(j.adminExternalUserIds)) setAdminExternalUserIds(j.adminExternalUserIds);
      alert("Administrateurs enregistrés.");
      window.location.reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const removeAdmin = (id: string) => {
    setAdminExternalUserIds((prev) => prev.filter((x) => x !== id));
  };

  const saveModuleConfig = async () => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/reservation-rooms/module-config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec enregistrement");
      if (j.config) setConfig(j.config);
      alert("Paramètres enregistrés.");
      window.location.reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const addSubject = () => {
    const name = newSubject.trim().toUpperCase();
    if (!name) return;
    if (config.subjectColors[name]) {
      alert("Cette matière existe déjà.");
      return;
    }
    setConfig({
      ...config,
      subjectColors: { ...config.subjectColors, [name]: newSubjectColor },
    });
    setNewSubject("");
  };

  const removeSubject = (name: string) => {
    if (!confirm(`Supprimer la matière « ${name} » ?`)) return;
    const next = { ...config.subjectColors };
    delete next[name];
    setConfig({ ...config, subjectColors: next });
  };

  const addPole = () => {
    const name = newPoleName.trim().toUpperCase();
    if (!name) return;
    if (config.classesByPole[name]) {
      alert("Ce pôle existe déjà.");
      return;
    }
    setConfig({
      ...config,
      classesByPole: { ...config.classesByPole, [name]: [] },
    });
    setNewPoleName("");
  };

  const addClassToPole = (pole: string) => {
    const cls = (newClassByPole[pole] || "").trim().toUpperCase();
    if (!cls) return;
    const list = config.classesByPole[pole] || [];
    if (list.includes(cls)) return;
    setConfig({
      ...config,
      classesByPole: { ...config.classesByPole, [pole]: [...list, cls] },
    });
    setNewClassByPole({ ...newClassByPole, [pole]: "" });
  };

  const removeClassFromPole = (pole: string, cls: string) => {
    setConfig({
      ...config,
      classesByPole: {
        ...config.classesByPole,
        [pole]: (config.classesByPole[pole] || []).filter((c) => c !== cls),
      },
    });
  };

  if (loading) {
    return <p className={`p-10 text-center text-sm ${dash.textMid}`}>Chargement du paramétrage…</p>;
  }

  const fieldClass =
    "w-full rounded-xl border border-white/70 bg-white/80 px-4 py-3 text-sm font-semibold text-[var(--dash-ink)] outline-none shadow-sm transition focus:border-[var(--dash-primary)]";

  return (
    <div className="space-y-4">
      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 p-3 text-sm font-semibold text-rose-700">{error}</p>
      ) : null}

      <ProfRoomGlassCard bodyClassName="space-y-4 p-5 sm:p-6">
        <h2 className={`text-lg font-semibold tracking-tight ${dash.ink}`}>Administrateurs du module</h2>
        <p className={`text-sm ${dash.textMid}`}>
          Ajoutez ou retirez des personnes depuis l’annuaire. Elles auront le mode administrateur dans la réservation
          de salles et pourront modifier ce paramétrage.
        </p>

        {selectedAdmins.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {selectedAdmins.map((m) => (
              <span
                key={m.externalUserId}
                className={`inline-flex items-center gap-2 rounded-full border bg-white/80 px-3 py-1.5 text-xs font-semibold ${dash.borderSoft} ${dash.ink}`}
              >
                <span className="max-w-[12rem] truncate">{m.displayName || m.email}</span>
                <button
                  type="button"
                  onClick={() => removeAdmin(m.externalUserId)}
                  className={`cursor-pointer ${dash.textMid} hover:text-rose-600`}
                  title="Retirer cet administrateur"
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        ) : (
          <p className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800">
            Aucun administrateur sélectionné. Les administrateurs org conservent l&apos;accès.
          </p>
        )}

        <ProfRoomAdminPicker
          members={directoryMembers}
          selectedIds={adminExternalUserIds}
          onChange={setAdminExternalUserIds}
          loading={membersLoading}
          footerHint="Cochez ou décochez pour ajouter ou retirer un administrateur. Enregistrez pour appliquer."
        />

        <ModuleButton disabled={saving || membersLoading} onClick={saveAdmins}>
          Enregistrer les administrateurs
        </ModuleButton>
      </ProfRoomGlassCard>

      <ProfRoomGlassCard bodyClassName="space-y-4 p-5 sm:p-6">
        <h2 className={`text-lg font-semibold tracking-tight ${dash.ink}`}>Salles</h2>
        <p className={`text-sm ${dash.textMid}`}>
          Salles spéciales (réservables) et salles de classe (visibles, non réservables pour
          l’instant — alimentées aussi par les EDT).
        </p>
        {rooms.map((room, idx) => (
          <div key={room.id || idx} className="flex flex-col gap-2 sm:flex-row sm:items-center">
            <input
              className={`${fieldClass} flex-1`}
              placeholder="Nom de la salle (ex: Salle informatique ou 1A)"
              value={room.name}
              onChange={(e) => {
                const next = [...rooms];
                next[idx] = { ...room, name: e.target.value };
                setRooms(next);
              }}
            />
            <select
              className={`${fieldClass} sm:w-40`}
              value={room.kind === "classroom" ? "classroom" : "facility"}
              onChange={(e) => {
                const kind = e.target.value === "classroom" ? "classroom" : "facility";
                const next = [...rooms];
                next[idx] = {
                  ...room,
                  kind,
                  bookable: kind === "facility" ? true : room.bookable === true,
                };
                setRooms(next);
              }}
            >
              <option value="facility">Spéciale</option>
              <option value="classroom">Classe</option>
            </select>
            <label className="inline-flex items-center gap-1.5 text-xs font-semibold text-slate-600 shrink-0">
              <input
                type="checkbox"
                checked={room.bookable !== false}
                onChange={(e) => {
                  const next = [...rooms];
                  next[idx] = { ...room, bookable: e.target.checked };
                  setRooms(next);
                }}
              />
              Réservable
            </label>
            <button
              type="button"
              onClick={() => setRooms(rooms.filter((_, i) => i !== idx))}
              className="shrink-0 cursor-pointer px-3 text-sm font-semibold text-rose-700 hover:text-rose-800"
            >
              Supprimer
            </button>
          </div>
        ))}
        <div className="flex flex-wrap items-center gap-4 pt-1">
          <button
            type="button"
            className={`cursor-pointer text-sm font-semibold ${dash.textPrimary}`}
            onClick={() =>
              setRooms([
                ...rooms,
                {
                  id: `salle-${Date.now()}`,
                  name: "",
                  kind: "facility",
                  bookable: true,
                },
              ])
            }
          >
            + Salle spéciale
          </button>
          <button
            type="button"
            className={`cursor-pointer text-sm font-semibold ${dash.textPrimary}`}
            onClick={() =>
              setRooms([
                ...rooms,
                {
                  id: `salle-${Date.now()}`,
                  name: "",
                  kind: "classroom",
                  bookable: false,
                },
              ])
            }
          >
            + Salle de classe
          </button>
          <ModuleButton disabled={saving} onClick={saveRooms}>
            Enregistrer les salles
          </ModuleButton>
        </div>
      </ProfRoomGlassCard>

      <ProfRoomGlassCard bodyClassName="space-y-4 p-5 sm:p-6">
        <h2 className={`text-lg font-semibold tracking-tight ${dash.ink}`}>Matières & couleurs</h2>
        <p className={`text-sm ${dash.textMid}`}>
          Choisissez un preset ou une couleur personnalisée via le sélecteur ({Object.keys(config.subjectColors).length}{" "}
          matières).
        </p>
        <div className="space-y-2">
          {Object.entries(config.subjectColors).map(([name, colorValue]) => (
            <SubjectColorEditor
              key={name}
              label={name}
              value={colorValue}
              onChange={(next) =>
                setConfig({
                  ...config,
                  subjectColors: { ...config.subjectColors, [name]: next },
                })
              }
              onRemove={() => removeSubject(name)}
            />
          ))}
        </div>
        <div className={`flex flex-col gap-3 border-t pt-3 ${dash.divider}`}>
          <input
            className={`${fieldClass} uppercase`}
            placeholder="Nouvelle matière"
            value={newSubject}
            onChange={(e) => setNewSubject(e.target.value)}
          />
          <SubjectColorEditor label="Aperçu" value={newSubjectColor} onChange={setNewSubjectColor} />
          <ModuleButton variant="secondary" onClick={addSubject} className="self-start">
            Ajouter la matière
          </ModuleButton>
        </div>
        <ModuleButton disabled={saving} onClick={saveModuleConfig}>
          Enregistrer matières & classes
        </ModuleButton>
      </ProfRoomGlassCard>

      <ProfRoomGlassCard bodyClassName="space-y-4 p-5 sm:p-6">
        <h2 className={`text-lg font-semibold tracking-tight ${dash.ink}`}>Classes par pôle</h2>
        <p className={`text-sm ${dash.textMid}`}>
          Organisez les classes proposées selon le niveau (École, Collège, Lycée…).
        </p>
        {Object.entries(config.classesByPole).map(([pole, classes]) => (
          <div key={pole} className={`space-y-3 rounded-2xl border bg-white/55 p-4 ${dash.borderSoft}`}>
            <p className={`text-sm font-semibold uppercase tracking-wide ${dash.ink}`}>{pole}</p>
            <div className="flex flex-wrap gap-2">
              {classes.map((cls) => (
                <span
                  key={cls}
                  className={`inline-flex items-center gap-1 rounded-lg px-2 py-1 text-xs font-semibold ${dash.bgSoft} ${dash.ink}`}
                >
                  {cls}
                  <button
                    type="button"
                    onClick={() => removeClassFromPole(pole, cls)}
                    className="cursor-pointer text-rose-500 hover:text-rose-700"
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
            <div className="flex gap-2">
              <input
                className={`${fieldClass} flex-1 uppercase`}
                placeholder="Nouvelle classe"
                value={newClassByPole[pole] || ""}
                onChange={(e) => setNewClassByPole({ ...newClassByPole, [pole]: e.target.value })}
                onKeyDown={(e) => e.key === "Enter" && addClassToPole(pole)}
              />
              <button
                type="button"
                onClick={() => addClassToPole(pole)}
                className={`cursor-pointer px-3 text-sm font-semibold ${dash.textPrimary}`}
              >
                + Classe
              </button>
            </div>
          </div>
        ))}
        <div className={`flex gap-2 border-t pt-3 ${dash.divider}`}>
          <input
            className={`${fieldClass} flex-1 uppercase`}
            placeholder="Nouveau pôle (ex: COLLÈGE)"
            value={newPoleName}
            onChange={(e) => setNewPoleName(e.target.value)}
          />
          <ModuleButton variant="secondary" onClick={addPole}>
            Ajouter pôle
          </ModuleButton>
        </div>
        <div className="pt-2">
          <label className={`mb-1 block text-[11px] font-semibold uppercase tracking-wide ${dash.textMid}`}>
            Horizon de réservation (jours, professeurs)
          </label>
          <input
            type="number"
            min={7}
            max={365}
            className={`${fieldClass} w-32`}
            value={config.bookingHorizonDays}
            onChange={(e) => setConfig({ ...config, bookingHorizonDays: parseInt(e.target.value, 10) || 56 })}
          />
        </div>
      </ProfRoomGlassCard>
    </div>
  );
}
