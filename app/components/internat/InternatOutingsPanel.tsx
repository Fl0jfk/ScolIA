"use client";

import { useCallback, useEffect, useState } from "react";
import { OUTING_STATUS_LABELS } from "@/app/lib/internat-outing";
import type { InternatOuting, InternatStudent } from "@/app/lib/internat-types";
import { studentDisplayName } from "@/app/lib/internat-types";

const STATUS_TONE: Record<string, string> = {
  pending_direction: "bg-amber-100 text-amber-900",
  pending_parents: "bg-blue-100 text-blue-900",
  authorized: "bg-emerald-100 text-emerald-900",
  refused: "bg-red-100 text-red-900",
  cancelled: "bg-slate-100 text-slate-600",
};

export default function InternatOutingsPanel({
  students,
  canManage,
}: {
  students: InternatStudent[];
  canManage: boolean;
}) {
  const [outings, setOutings] = useState<InternatOuting[]>([]);
  const [authorizedToday, setAuthorizedToday] = useState<
    Array<{
      studentName: string;
      classe: string;
      outingTitle: string;
      outingDate: string;
      departureTime?: string;
    }>
  >([]);
  const [authorizedWeek, setAuthorizedWeek] = useState<typeof authorizedToday>([]);
  const [busy, setBusy] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [form, setForm] = useState({
    title: "",
    activity: "",
    destination: "",
    accompanists: "",
    outingDate: "",
    departureTime: "",
    returnTime: "",
  });

  const activeStudents = students.filter((s) => s.actif);

  const load = useCallback(async () => {
    const [outingsRes, todayRes, weekRes] = await Promise.all([
      fetch("/api/internat/outings", { cache: "no-store" }),
      fetch("/api/internat/outings?view=authorized&scope=today", { cache: "no-store" }),
      fetch("/api/internat/outings?view=authorized&scope=week", { cache: "no-store" }),
    ]);
    const data = await outingsRes.json();
    const todayData = await todayRes.json();
    const weekData = await weekRes.json();
    if (outingsRes.ok) setOutings(data.outings || []);
    if (todayRes.ok) setAuthorizedToday(todayData.authorized || []);
    if (weekRes.ok) setAuthorizedWeek(weekData.authorized || []);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const createOuting = async () => {
    const studentIds = activeStudents.filter((s) => selected[s.id]).map((s) => s.id);
    if (!studentIds.length) return alert("Sélectionnez au moins un interne.");
    setBusy(true);
    try {
      const res = await fetch("/api/internat/outings", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...form, studentIds }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Création impossible");
      if (data.emailWarnings?.length) {
        alert(`Sortie créée avec avertissements :\n${data.emailWarnings.join("\n")}`);
      } else {
        alert("Sortie créée. Les e-mails de validation ont été envoyés à la direction.");
      }
      setShowForm(false);
      setSelected({});
      setForm({
        title: "",
        activity: "",
        destination: "",
        accompanists: "",
        outingDate: "",
        departureTime: "",
        returnTime: "",
      });
      await load();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const directionDecision = async (
    outingId: string,
    etablissement: string,
    decision: "approve" | "refuse",
  ) => {
    if (decision === "refuse" && !confirm(`Refuser la validation ${etablissement} ?`)) return;
    setBusy(true);
    try {
      const res = await fetch("/api/internat/outings", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: outingId, action: "direction_decision", etablissement, decision }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Action impossible");
      await load();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const resendParents = async (id: string) => {
    setBusy(true);
    try {
      const res = await fetch("/api/internat/outings", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, action: "resend_parents" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Envoi impossible");
      alert("E-mails parents relancés si applicable.");
      await load();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const cancelOuting = async (id: string) => {
    if (!confirm("Annuler cette demande de sortie ?")) return;
    setBusy(true);
    try {
      const res = await fetch("/api/internat/outings", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, action: "cancel" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Annulation impossible");
      await load();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-indigo-100 bg-indigo-50/50 p-5 text-sm text-indigo-950 space-y-3">
        <div>
          <p className="font-bold mb-1">Autorisations de sortie</p>
          <p>
            La direction valide d&apos;abord la sortie par e-mail, puis les parents reçoivent un lien sécurisé pour
            autoriser ou refuser. Un e-mail parent suffit par interne (parent 1 ou parent 2).
          </p>
        </div>
        {canManage && (
          <div className="flex flex-wrap items-center gap-3 pt-1 border-t border-indigo-100">
            <button
              type="button"
              disabled={busy}
              onClick={async () => {
                if (!confirm("Envoyer le récap hebdomadaire aux parents (sorties de la semaine prochaine) ?")) return;
                setBusy(true);
                try {
                  const res = await fetch("/api/internat/weekly-parent-digest", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ force: true }),
                  });
                  const data = await res.json();
                  if (!res.ok) throw new Error(data?.error || "Envoi impossible");
                  alert(
                    data.count
                      ? `${data.count} e-mail(s) parent envoyé(s) pour la semaine du ${data.weekLabel}.`
                      : "Aucune sortie à annoncer la semaine prochaine (ou aucun mail parent renseigné).",
                  );
                } catch (e: unknown) {
                  alert(e instanceof Error ? e.message : "Erreur");
                } finally {
                  setBusy(false);
                }
              }}
              className="bg-white border border-indigo-200 text-indigo-800 px-4 py-2 rounded-xl font-bold text-sm"
            >
              Récap hebdo parents
            </button>
            <span className="text-xs text-indigo-800/70">Envoi auto le dimanche si activé (cron).</span>
          </div>
        )}
      </div>

      <div className="grid sm:grid-cols-2 gap-4">
        <section className="bg-white border border-emerald-100 rounded-2xl p-4">
          <h3 className="font-black text-emerald-900 text-sm mb-2">Autorisés aujourd&apos;hui</h3>
          {authorizedToday.length === 0 ? (
            <p className="text-xs text-slate-500">Aucun interne autorisé pour aujourd&apos;hui.</p>
          ) : (
            <ul className="text-sm space-y-1">
              {authorizedToday.map((a, i) => (
                <li key={`${a.studentName}-${i}`}>
                  <span className="font-semibold">{a.studentName}</span>
                  <span className="text-slate-500 text-xs"> — {a.outingTitle}</span>
                </li>
              ))}
            </ul>
          )}
        </section>
        <section className="bg-white border border-sky-100 rounded-2xl p-4">
          <h3 className="font-black text-sky-900 text-sm mb-2">Autorisés cette semaine</h3>
          {authorizedWeek.length === 0 ? (
            <p className="text-xs text-slate-500">Aucune autorisation sur la semaine en cours.</p>
          ) : (
            <ul className="text-sm space-y-1 max-h-40 overflow-y-auto">
              {authorizedWeek.map((a, i) => (
                <li key={`${a.studentName}-${a.outingDate}-${i}`}>
                  <span className="font-semibold">{a.studentName}</span>
                  <span className="text-slate-500 text-xs">
                    {" "}
                    — {a.outingDate} · {a.outingTitle}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      {canManage && (
        <button
          type="button"
          disabled={busy}
          onClick={() => setShowForm((v) => !v)}
          className="bg-indigo-600 text-white px-4 py-2 rounded-xl font-bold text-sm"
        >
          {showForm ? "Fermer le formulaire" : "Nouvelle sortie"}
        </button>
      )}

      {showForm && canManage && (
        <div className="bg-white border border-slate-200 rounded-2xl p-5 space-y-4">
          <h3 className="font-black text-slate-900">Créer une sortie</h3>
          <div className="grid sm:grid-cols-2 gap-3">
            <input
              className="border rounded-xl px-3 py-2 text-sm sm:col-span-2"
              placeholder="Titre (ex. Sortie cinéma mercredi)"
              value={form.title}
              onChange={(e) => setForm({ ...form, title: e.target.value })}
            />
            <input
              className="border rounded-xl px-3 py-2 text-sm sm:col-span-2"
              placeholder="Activité *"
              value={form.activity}
              onChange={(e) => setForm({ ...form, activity: e.target.value })}
            />
            <input
              className="border rounded-xl px-3 py-2 text-sm"
              placeholder="Lieu / destination"
              value={form.destination}
              onChange={(e) => setForm({ ...form, destination: e.target.value })}
            />
            <input
              className="border rounded-xl px-3 py-2 text-sm"
              placeholder="Accompagnateur(s) *"
              value={form.accompanists}
              onChange={(e) => setForm({ ...form, accompanists: e.target.value })}
            />
            <input
              type="date"
              className="border rounded-xl px-3 py-2 text-sm"
              value={form.outingDate}
              onChange={(e) => setForm({ ...form, outingDate: e.target.value })}
            />
            <div className="flex gap-2">
              <input
                type="time"
                className="border rounded-xl px-3 py-2 text-sm flex-1"
                value={form.departureTime}
                onChange={(e) => setForm({ ...form, departureTime: e.target.value })}
              />
              <input
                type="time"
                className="border rounded-xl px-3 py-2 text-sm flex-1"
                placeholder="Retour"
                value={form.returnTime}
                onChange={(e) => setForm({ ...form, returnTime: e.target.value })}
              />
            </div>
          </div>

          <div>
            <p className="text-xs font-bold uppercase text-slate-500 mb-2">Internes concernés</p>
            <div className="max-h-48 overflow-y-auto border border-slate-100 rounded-xl p-3 space-y-2 text-sm">
              {activeStudents.map((s) => {
                const hasEmail = Boolean(s.parent1?.email || s.parent2?.email);
                return (
                  <label key={s.id} className={`flex items-center gap-2 ${!hasEmail ? "text-amber-800" : ""}`}>
                    <input
                      type="checkbox"
                      checked={!!selected[s.id]}
                      onChange={(e) => setSelected((prev) => ({ ...prev, [s.id]: e.target.checked }))}
                    />
                    <span>
                      {studentDisplayName(s)} — {s.classe}
                      {!hasEmail && " (e-mail parent manquant)"}
                    </span>
                  </label>
                );
              })}
            </div>
          </div>

          <button
            type="button"
            disabled={busy}
            onClick={createOuting}
            className="bg-emerald-600 text-white px-4 py-2 rounded-xl font-bold text-sm"
          >
            Envoyer à la direction
          </button>
        </div>
      )}

      <div className="bg-white border border-slate-200 rounded-2xl overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-slate-50 text-left">
            <tr>
              <th className="p-3 font-bold">Sortie</th>
              <th className="p-3 font-bold">Date</th>
              <th className="p-3 font-bold">Internes</th>
              <th className="p-3 font-bold">Statut</th>
              <th className="p-3 font-bold">Suivi</th>
              {canManage && <th className="p-3 font-bold">Actions</th>}
            </tr>
          </thead>
          <tbody>
            {outings.length === 0 && (
              <tr>
                <td colSpan={canManage ? 6 : 5} className="p-6 text-center text-slate-500">
                  Aucune sortie enregistrée.
                </td>
              </tr>
            )}
            {outings.map((o) => (
              <tr key={o.id} className="border-t border-slate-100 align-top">
                <td className="p-3">
                  <p className="font-semibold">{o.title}</p>
                  <p className="text-xs text-slate-500 mt-0.5">{o.activity}</p>
                </td>
                <td className="p-3 whitespace-nowrap">{o.outingDate}</td>
                <td className="p-3">
                  <ul className="text-xs space-y-1">
                    {o.participants.map((p) => (
                      <li key={p.studentId}>
                        {p.studentName}{" "}
                        <span
                          className={
                            p.parentStatus === "authorized"
                              ? "text-emerald-700"
                              : p.parentStatus === "refused"
                                ? "text-red-700"
                                : "text-slate-400"
                          }
                        >
                          ({p.parentStatus === "authorized" ? "OK parent" : p.parentStatus === "refused" ? "refus parent" : "attente parent"})
                        </span>
                      </li>
                    ))}
                  </ul>
                </td>
                <td className="p-3">
                  <span className={`text-xs font-bold px-2 py-1 rounded-lg ${STATUS_TONE[o.status] || ""}`}>
                    {OUTING_STATUS_LABELS[o.status]}
                  </span>
                </td>
                <td className="p-3 text-xs text-slate-500">
                  {o.directionDecisions.map((d) => (
                    <div key={d.etablissement} className="mb-1">
                      {d.etablissement} :{" "}
                      {d.status === "approved" ? "validé" : d.status === "refused" ? "refusé" : "en attente"}
                      {canManage && d.status === "pending" && o.status !== "cancelled" && (
                        <span className="ml-1 inline-flex gap-1">
                          <button
                            type="button"
                            disabled={busy}
                            className="text-emerald-700 font-bold"
                            onClick={() => directionDecision(o.id, d.etablissement, "approve")}
                          >
                            OK
                          </button>
                          <button
                            type="button"
                            disabled={busy}
                            className="text-red-600 font-bold"
                            onClick={() => directionDecision(o.id, d.etablissement, "refuse")}
                          >
                            Refus
                          </button>
                        </span>
                      )}
                    </div>
                  ))}
                </td>
                {canManage && (
                  <td className="p-3 space-y-1">
                    {!["cancelled", "authorized"].includes(o.status) && (
                      <button
                        type="button"
                        className="block text-xs text-red-600 font-bold"
                        disabled={busy}
                        onClick={() => cancelOuting(o.id)}
                      >
                        Annuler
                      </button>
                    )}
                    {o.status === "pending_parents" && (
                      <button
                        type="button"
                        className="block text-xs text-indigo-600 font-bold"
                        disabled={busy}
                        onClick={() => resendParents(o.id)}
                      >
                        Relancer parents
                      </button>
                    )}
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
