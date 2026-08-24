"use client";

import { useEffect, useMemo, useState } from "react";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import DirectoryPersonSelect, {
  directoryMemberLabel,
} from "@/app/components/settings/DirectoryPersonSelect";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";
import { useAppContext } from "@/app/hooks/useAppContext";
import { formatAbsencePeriod, type AbsencePeriodType } from "@/app/lib/absence-period";
import {
  canChooseDeclarationScope,
  resolveSelfDeclarationScope,
  type AbsenceScope,
} from "@/app/lib/absences-types";

type DirectoryMemberWithRoles = DirectoryMemberOption & { roles?: string[] };

type Props = {
  onSuccess?: () => void;
};

export default function AbsencesDeclareOnBehalf({ onSuccess }: Props) {
  const { data: appCtx } = useAppContext();
  const establishments = appCtx?.establishments ?? [];
  const [members, setMembers] = useState<DirectoryMemberWithRoles[]>([]);
  const [loadingMembers, setLoadingMembers] = useState(true);
  const [colleague, setColleague] = useState<DirectoryMemberWithRoles | null>(null);
  const [declareScope, setDeclareScope] = useState<AbsenceScope>("ogec");
  const [etablissement, setEtablissement] = useState("");
  const [periodType, setPeriodType] = useState<AbsencePeriodType>("multi_day");
  const [singleDayDate, setSingleDayDate] = useState("");
  const [startTime, setStartTime] = useState("");
  const [endTime, setEndTime] = useState("");
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");
  const [reason, setReason] = useState("");
  const [details, setDetails] = useState("");
  const [justificationFile, setJustificationFile] = useState<File | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        setLoadingMembers(true);
        const res = await fetch("/api/absences/directory-users", { cache: "no-store" });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) throw new Error(data?.error || "Annuaire indisponible.");
        if (!cancelled) setMembers(Array.isArray(data.users) ? data.users : []);
      } catch (e: unknown) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Erreur annuaire.");
      } finally {
        if (!cancelled) setLoadingMembers(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const colleagueRoles = useMemo(
    () => (Array.isArray(colleague?.roles) ? colleague!.roles.map(String) : []),
    [colleague],
  );
  const canChooseScope = canChooseDeclarationScope(colleagueRoles);
  const effectiveScope = canChooseScope
    ? declareScope
    : resolveSelfDeclarationScope(colleagueRoles);

  useEffect(() => {
    if (!colleague) return;
    setDeclareScope(resolveSelfDeclarationScope(colleagueRoles));
  }, [colleague, colleagueRoles]);

  const submit = async () => {
    setError(null);
    setSuccess(null);
    if (!colleague?.externalUserId) {
      setError("Choisissez le collègue concerné.");
      return;
    }
    if (!reason.trim()) {
      setError("Merci de remplir le motif.");
      return;
    }
    if (periodType === "single_day") {
      if (!singleDayDate || !startTime || !endTime) {
        setError("Pour une journée, indiquez la date et les heures.");
        return;
      }
      if (endTime <= startTime) {
        setError("L'heure de fin doit être après l'heure de début.");
        return;
      }
    } else if (!startDate || !endDate) {
      setError("Indiquez la date de début et la date de fin.");
      return;
    } else if (endDate < startDate) {
      setError("La date de fin doit être après la date de début.");
      return;
    }
    if (effectiveScope === "professeur" && !etablissement) {
      setError("Merci de choisir un établissement.");
      return;
    }

    try {
      setSaving(true);
      let justification: { fileName: string; fileUrl: string } | null = null;
      if (justificationFile) {
        const presignRes = await fetch("/api/travels/upload", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            fileName: justificationFile.name,
            fileType: justificationFile.type || "application/octet-stream",
          }),
        });
        const presignPayload = await presignRes.json();
        if (!presignRes.ok || !presignPayload?.uploadUrl || !presignPayload?.fileUrl) {
          throw new Error("Impossible de préparer l'upload du justificatif.");
        }
        await fetch(presignPayload.uploadUrl, {
          method: "PUT",
          body: justificationFile,
          headers: { "Content-Type": justificationFile.type || "application/octet-stream" },
        });
        justification = {
          fileName: justificationFile.name,
          fileUrl: presignPayload.fileUrl,
        };
      }

      const res = await fetch("/api/absences", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          onBehalfOf: { userId: colleague.externalUserId },
          data: {
            scope: effectiveScope,
            etablissement: effectiveScope === "professeur" ? etablissement : null,
            periodType,
            startDate: periodType === "single_day" ? singleDayDate : startDate,
            endDate: periodType === "single_day" ? singleDayDate : endDate,
            startTime: periodType === "single_day" ? startTime : null,
            endTime: periodType === "single_day" ? endTime : null,
            reason: reason.trim(),
            details: details.trim(),
            justification,
          },
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.error || "Échec création");

      setSuccess(
        `Demande enregistrée pour ${directoryMemberLabel(colleague)} — en attente de validation direction (comme une auto-déclaration).`,
      );
      setColleague(null);
      setPeriodType("multi_day");
      setSingleDayDate("");
      setStartTime("");
      setEndTime("");
      setStartDate("");
      setEndDate("");
      setReason("");
      setDetails("");
      setJustificationFile(null);
      onSuccess?.();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur de création.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-6 max-w-3xl">
      <section className="bg-white border border-emerald-200 rounded-3xl p-6 shadow-sm">
        <h2 className="text-xl font-black text-slate-900 mb-1">Demande pour un collègue</h2>
        <p className="text-sm text-slate-600 mb-4">
          Même circuit qu&apos;une auto-déclaration : la demande part en{" "}
          <strong>attente de validation</strong> auprès de la direction. Utile si un collègue est en
          difficulté pour saisir lui-même.
        </p>

        <div className="space-y-4">
          <div>
            <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
              Collègue concerné
            </label>
            <DirectoryPersonSelect
              members={members}
              selectedId={colleague?.externalUserId}
              selectedEmail={colleague?.email}
              onChange={(m) => {
                if (!m) {
                  setColleague(null);
                  return;
                }
                const full = members.find((x) => x.externalUserId === m.externalUserId) || m;
                setColleague(full);
              }}
              loading={loadingMembers}
            />
          </div>

          {colleague ? (
            <>
              {canChooseScope ? (
                <div>
                  <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                    Type d&apos;absence
                  </label>
                  <select
                    value={declareScope}
                    onChange={(e) => setDeclareScope(e.target.value as AbsenceScope)}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
                    disabled={saving}
                  >
                    <option value="ogec">Personnel OGEC</option>
                    <option value="professeur">Professeur</option>
                  </select>
                </div>
              ) : (
                <div className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-sm">
                  <span className="font-black text-slate-700">Type :</span>{" "}
                  <span className="font-semibold text-slate-800">
                    {effectiveScope === "professeur" ? "Professeur" : "Personnel OGEC"}
                  </span>
                </div>
              )}

              {effectiveScope === "professeur" && (
                <div>
                  <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                    Établissement
                  </label>
                  <EstablishmentSelect
                    value={etablissement}
                    onChange={setEtablissement}
                    establishments={establishments}
                    includeGroupe={false}
                    className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
                    disabled={saving}
                  />
                </div>
              )}

              <div>
                <label className="text-[11px] font-black uppercase tracking-wider text-slate-500 block mb-2">
                  Durée
                </label>
                <select
                  value={periodType}
                  onChange={(e) => setPeriodType(e.target.value as AbsencePeriodType)}
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 bg-white"
                  disabled={saving}
                >
                  <option value="single_day">Une journée (créneau horaire)</option>
                  <option value="multi_day">Plusieurs jours</option>
                </select>
              </div>

              {periodType === "single_day" ? (
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                  <label className="block sm:col-span-1">
                    <span className="text-xs font-bold text-slate-600">Jour</span>
                    <input
                      type="date"
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                      value={singleDayDate}
                      onChange={(e) => setSingleDayDate(e.target.value)}
                      disabled={saving}
                    />
                  </label>
                  <label className="block">
                    <span className="text-xs font-bold text-slate-600">Début</span>
                    <input
                      type="time"
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                      value={startTime}
                      onChange={(e) => setStartTime(e.target.value)}
                      disabled={saving}
                    />
                  </label>
                  <label className="block">
                    <span className="text-xs font-bold text-slate-600">Fin</span>
                    <input
                      type="time"
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                      value={endTime}
                      onChange={(e) => setEndTime(e.target.value)}
                      disabled={saving}
                    />
                  </label>
                </div>
              ) : (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <label className="block">
                    <span className="text-xs font-bold text-slate-600">Du</span>
                    <input
                      type="date"
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                      value={startDate}
                      onChange={(e) => setStartDate(e.target.value)}
                      disabled={saving}
                    />
                  </label>
                  <label className="block">
                    <span className="text-xs font-bold text-slate-600">Au</span>
                    <input
                      type="date"
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                      value={endDate}
                      onChange={(e) => setEndDate(e.target.value)}
                      disabled={saving}
                    />
                  </label>
                </div>
              )}

              <label className="block">
                <span className="text-xs font-bold text-slate-600">Motif</span>
                <input
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  disabled={saving}
                  placeholder="Ex. arrêt maladie, convocation…"
                />
              </label>

              <label className="block">
                <span className="text-xs font-bold text-slate-600">Détails (optionnel)</span>
                <textarea
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 min-h-[80px]"
                  value={details}
                  onChange={(e) => setDetails(e.target.value)}
                  disabled={saving}
                />
              </label>

              <label className="block">
                <span className="text-xs font-bold text-slate-600">Justificatif (optionnel)</span>
                <input
                  type="file"
                  className="mt-1 block w-full text-sm"
                  onChange={(e) => setJustificationFile(e.target.files?.[0] || null)}
                  disabled={saving}
                />
              </label>

              {(startDate || singleDayDate) && reason.trim() ? (
                <p className="text-xs text-slate-500">
                  Aperçu :{" "}
                  {formatAbsencePeriod({
                    periodType,
                    startDate: periodType === "single_day" ? singleDayDate : startDate,
                    endDate: periodType === "single_day" ? singleDayDate : endDate,
                    startTime: periodType === "single_day" ? startTime : null,
                    endTime: periodType === "single_day" ? endTime : null,
                  })}
                </p>
              ) : null}

              <button
                type="button"
                onClick={() => void submit()}
                disabled={saving}
                className="rounded-xl bg-emerald-700 px-4 py-2.5 text-sm font-bold text-white hover:bg-emerald-800 disabled:opacity-60"
              >
                {saving ? "Envoi…" : "Envoyer la demande (validation direction)"}
              </button>
            </>
          ) : null}
        </div>
      </section>

      {success ? (
        <p className="text-sm text-emerald-700 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">
          {success}
        </p>
      ) : null}
      {error ? (
        <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">{error}</p>
      ) : null}
    </div>
  );
}
