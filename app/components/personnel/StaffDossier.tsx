"use client";

import { useState } from "react";
import Link from "next/link";
import StaffRhSummary from "@/app/components/personnel/StaffRhSummary";
import { RhField, RhInput, RhModalShell, RhTextarea } from "@/app/components/personnel/RhFormModals";
import RhStaffProfileFields from "@/app/components/personnel/RhStaffProfileFields";
import { profileFromFormData } from "@/app/lib/personnel-profile";
import { PERSONNEL_TEMPLATE_VAR_DEFS } from "@/app/lib/personnel-template-vars";
import {
  PERSONNEL_CATEGORY_LABELS,
  type OnboardingSignature,
  type PersonnelRecord,
  type SharedPersonnelDocument,
} from "@/app/lib/personnel-types";
import {
  attachDocumentToStaff,
  PERSONNEL_DROP_ACCEPT,
} from "@/app/lib/personnel-upload-client";
import PersonnelDropZone from "@/app/components/personnel/PersonnelDropZone";
import { StaffDossierTabBar, staffDossierTabs, type StaffDossierTabId } from "@/app/components/personnel/StaffDossierTabBar";

type Props = {
  record: PersonnelRecord;
  canManage: boolean;
  sharedDocs?: SharedPersonnelDocument[];
  onRefresh: () => void;
  backHref?: string;
};

type ModalKind = "medecine" | "entretien-plan" | "entretien-realise" | null;

export default function StaffDossier({ record, canManage, sharedDocs = [], onRefresh, backHref }: Props) {
  const [tab, setTab] = useState<StaffDossierTabId>("identite");
  const [signEmails, setSignEmails] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [depositBusy, setDepositBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalKind>(null);

  const patch = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMsg(null);
    try {
      const res = await fetch(`/api/personnel/${record.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      onRefresh();
      setModal(null);
    } catch (e) {
      setMsg(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const handleIaDeposit = async (file: File) => {
    setDepositBusy(true);
    setMsg(null);
    try {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("staffId", record.id);
      const res = await fetch("/api/personnel/deposit", { method: "POST", body: fd });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Dépôt impossible");
      onRefresh();
      if (j.autoCreated?.length) {
        setMsg(`Document rangé · ${j.autoCreated.join(" · ")}`);
      }
    } catch (e) {
      setMsg(e instanceof Error ? e.message : "Erreur dépôt IA");
    } finally {
      setDepositBusy(false);
    }
  };

  const handleDocUpload = async (
    file: File,
    visibility: "personnel" | "establishment" | "restricted",
  ) => {
    setBusy(true);
    try {
      await attachDocumentToStaff(record.id, file, { visibility });
      onRefresh();
    } catch (e) {
      setMsg(e instanceof Error ? e.message : "Erreur upload");
    } finally {
      setBusy(false);
    }
  };

  const visits = [...(record.medecineTravail.visits || [])].sort(
    (a, b) => new Date(b.visitedAt).getTime() - new Date(a.visitedAt).getTime(),
  );

  const p = record.profile;

  const tabs = staffDossierTabs({
    hasOnboarding: Boolean(record.onboarding),
    showOffboarding: Boolean(record.offboarding || canManage),
  });

  const renderSignatures = (kind: "onboarding" | "offboarding", signatures: OnboardingSignature[]) => {
    if (!signatures) return null;
    return (
      <ul className="space-y-3">
        {signatures.map((s) => (
          <li key={s.id} className="rounded-xl border p-3 space-y-2">
            <div className="flex items-center justify-between gap-2">
              <span className="font-bold text-sm">{s.label}</span>
              {s.status === "signe" ? (
                <span className="text-xs font-bold text-emerald-600">
                  Signé · {s.signedAt ? new Date(s.signedAt).toLocaleDateString("fr-FR") : ""}
                </span>
              ) : (
                <span className="text-xs font-bold text-amber-600">En attente</span>
              )}
            </div>
            {s.status !== "signe" && canManage && (
              <div className="flex flex-wrap gap-2 items-center">
                <input
                  type="email"
                  className="border rounded-lg px-2 py-1 text-xs flex-1 min-w-[10rem]"
                  placeholder="E-mail du signataire"
                  value={signEmails[`${kind}-${s.id}`] || s.signEmail || ""}
                  onChange={(e) =>
                    setSignEmails((prev) => ({ ...prev, [`${kind}-${s.id}`]: e.target.value }))
                  }
                />
                <button
                  type="button"
                  disabled={busy}
                  onClick={() =>
                    patch({
                      action: "send-signature-link",
                      kind,
                      sigId: s.id,
                      email: signEmails[`${kind}-${s.id}`] || s.signEmail,
                    })
                  }
                  className="text-xs font-bold text-indigo-600 underline"
                >
                  Envoyer le lien
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() =>
                    patch({
                      action: kind === "onboarding" ? "sign-onboarding" : "sign-offboarding",
                      sigId: s.id,
                    })
                  }
                  className="text-xs font-bold text-slate-500"
                >
                  Marquer signé
                </button>
              </div>
            )}
            {s.signSentAt && s.status !== "signe" && (
              <p className="text-[10px] text-slate-400">
                Lien envoyé le {new Date(s.signSentAt).toLocaleString("fr-FR")}
              </p>
            )}
          </li>
        ))}
      </ul>
    );
  };

  return (
    <div className="space-y-6">
      {backHref && (
        <Link href={backHref} className="text-sm font-bold text-indigo-600 hover:underline">
          ← Retour
        </Link>
      )}

      <div className="bg-gradient-to-r from-slate-800 to-slate-700 rounded-2xl p-6 text-white shadow-lg">
        <p className="text-xs font-black uppercase tracking-widest text-slate-300">
          {PERSONNEL_CATEGORY_LABELS[record.category]}
        </p>
        <h1 className="text-2xl font-black mt-1">{record.displayName}</h1>
        <p className="text-slate-300 text-sm mt-1">{record.jobTitle || "—"} · {record.email}</p>
        {record.hireDate && (
          <p className="text-xs text-slate-400 mt-2">Entrée : {new Date(record.hireDate).toLocaleDateString("fr-FR")}</p>
        )}
      </div>

      <StaffRhSummary
        record={record}
        canManage={canManage}
        onDeposit={handleIaDeposit}
        depositBusy={depositBusy}
        onAddMedecineVisit={() => setModal("medecine")}
        onPlanEntretien={() => setModal("entretien-plan")}
        onAddEntretienRealise={() => setModal("entretien-realise")}
      />

      {msg && (
        <p className={`text-sm font-medium ${msg.includes("rangé") ? "text-emerald-600" : "text-rose-600"}`}>{msg}</p>
      )}

      <StaffDossierTabBar tabs={tabs} tab={tab} onChange={setTab} />

      {tab === "identite" && (
        <div className="bg-white rounded-2xl border p-5 space-y-6">
          <div>
            <h3 className="font-black text-slate-800">Données administratives</h3>
            <p className="text-xs text-slate-500 mt-1">
              Utilisées pour les contrats, déclarations Sécu, prévoyance et autres documents.
            </p>
          </div>

          {canManage ? (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const fd = new FormData(e.currentTarget);
                void patch({
                  action: "update-profile",
                  firstName: fd.get("firstName"),
                  lastName: fd.get("lastName"),
                  email: fd.get("email"),
                  jobTitle: fd.get("jobTitle"),
                  hireDate: fd.get("hireDate") || null,
                  profile: profileFromFormData(fd),
                });
              }}
              className="space-y-6"
            >
              <section>
                <h4 className="text-xs font-black uppercase tracking-widest text-slate-500 mb-3">Contact pro</h4>
                <div className="grid sm:grid-cols-2 gap-3">
                  <RhField label="Prénom">
                    <RhInput name="firstName" defaultValue={record.firstName} required />
                  </RhField>
                  <RhField label="Nom">
                    <RhInput name="lastName" defaultValue={record.lastName} required />
                  </RhField>
                  <RhField label="Email">
                    <RhInput name="email" type="email" defaultValue={record.email} required />
                  </RhField>
                  <RhField label="Poste">
                    <RhInput name="jobTitle" defaultValue={record.jobTitle || ""} />
                  </RhField>
                  <RhField label="Date d'entrée">
                    <RhInput name="hireDate" type="date" defaultValue={record.hireDate || ""} />
                  </RhField>
                </div>
              </section>

              <RhStaffProfileFields key={record.updatedAt} profile={record.profile} showContract showBank />

              <div className="rounded-xl border border-dashed border-slate-200 p-4 bg-slate-50">
                <p className="text-xs font-bold text-slate-600 mb-2">Variables document (aperçu)</p>
                <div className="flex flex-wrap gap-1.5 max-h-32 overflow-y-auto">
                  {PERSONNEL_TEMPLATE_VAR_DEFS.map((v) => (
                    <code key={v.key} className="text-[10px] bg-white border rounded px-1.5 py-0.5 text-indigo-700">
                      {`{{${v.key}}}`}
                    </code>
                  ))}
                </div>
              </div>

              <button
                type="submit"
                disabled={busy}
                className="px-5 py-2.5 rounded-xl bg-indigo-600 text-white text-sm font-bold disabled:opacity-50"
              >
                {busy ? "Enregistrement…" : "Enregistrer les données"}
              </button>
            </form>
          ) : (
            <div className="grid sm:grid-cols-2 gap-4 text-sm">
              <div>
                <p className="text-xs font-bold text-slate-500">N° sécurité sociale</p>
                <p className="font-medium">{p?.socialSecurityNumber || "—"}</p>
              </div>
              <div>
                <p className="text-xs font-bold text-slate-500">Date de naissance</p>
                <p className="font-medium">
                  {p?.birthDate ? new Date(p.birthDate).toLocaleDateString("fr-FR") : "—"}
                </p>
              </div>
              <div className="sm:col-span-2">
                <p className="text-xs font-bold text-slate-500">Adresse</p>
                <p className="font-medium">
                  {[p?.addressLine1, p?.postalCode, p?.city].filter(Boolean).join(", ") || "—"}
                </p>
              </div>
            </div>
          )}
        </div>
      )}

      {tab === "docs" && (
        <div className="space-y-4">
          {sharedDocs.length > 0 && (
            <div className="bg-indigo-50 rounded-2xl border border-indigo-100 p-5">
              <h3 className="font-black text-indigo-900 text-sm mb-3">Documents utiles (tous)</h3>
              <ul className="space-y-2">
                {sharedDocs.map((d) => (
                  <li key={d.id}>
                    <a href={d.fileUrl} target="_blank" rel="noopener noreferrer" className="text-sm font-bold text-indigo-700 underline">
                      {d.name}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div className="bg-white rounded-2xl border p-5 space-y-3">
            {canManage && (
              <PersonnelDropZone
                title="Glisser-déposer un document"
                hint="Excel (.xlsx, .xls), PDF ou Word — déposé sur ce dossier"
                disabled={busy}
                accept={PERSONNEL_DROP_ACCEPT}
                onFile={async (file) => {
                  const visible = confirm(
                    `Rendre « ${file.name} » visible par ${record.displayName} ?\n\nOK = oui · Annuler = réservé établissement`,
                  );
                  await handleDocUpload(file, visible ? "personnel" : "establishment");
                }}
              />
            )}

            <h3 className="font-black text-slate-800">Documents du dossier</h3>
            {record.documents.length === 0 ? (
              <p className="text-sm text-slate-400 italic">Aucun document.</p>
            ) : (
              <ul className="divide-y">
                {record.documents.map((d) => (
                  <li key={d.id} className="py-3 flex items-center justify-between gap-3">
                    <div>
                      <a href={d.fileUrl} target="_blank" rel="noopener noreferrer" className="font-bold text-sm text-indigo-600 underline">
                        {d.name}
                      </a>
                      <p className="text-[10px] text-slate-400 mt-0.5">
                        {d.category !== "autre" && `${d.category} · `}
                        {d.visibility === "personnel" ? "Visible personnel" : d.visibility === "restricted" ? "Restreint RH" : "Établissement"}
                        {d.expiresAt ? ` · expire ${new Date(d.expiresAt).toLocaleDateString("fr-FR")}` : ""}
                      </p>
                    </div>
                    {canManage && (
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => patch({ action: "remove-document", docId: d.id })}
                        className="text-xs text-rose-600 font-bold underline disabled:opacity-50"
                      >
                        Supprimer
                      </button>
                    )}
                  </li>
                ))}
              </ul>
            )}

            {canManage && (
              <div className="pt-4 border-t space-y-3">
                <p className="text-xs font-bold text-slate-600">Ajouter un document</p>
                <div className="flex flex-wrap gap-2">
                  {(
                    [
                      ["personnel", "Visible par la personne"],
                      ["establishment", "Établissement seulement"],
                      ["restricted", "Restreint RH"],
                    ] as const
                  ).map(([vis, label]) => (
                    <label key={vis} className="cursor-pointer">
                      <input
                        type="file"
                        className="hidden"
                        disabled={busy}
                        onChange={(e) => {
                          const f = e.target.files?.[0];
                          if (f) void handleDocUpload(f, vis);
                          e.target.value = "";
                        }}
                      />
                      <span className="inline-block px-3 py-2 rounded-xl bg-slate-100 text-xs font-bold text-slate-700 hover:bg-indigo-100 hover:text-indigo-800 transition">
                        + {label}
                      </span>
                    </label>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {tab === "formations" && (
        <div className="bg-white rounded-2xl border p-5 space-y-4">
          <h3 className="font-black text-slate-800">Formations</h3>
          {record.formations.length === 0 ? (
            <p className="text-sm text-slate-400 italic">Aucune formation enregistrée.</p>
          ) : (
            <ul className="space-y-2">
              {record.formations.map((f) => (
                <li key={f.id} className="rounded-xl border p-3 flex justify-between items-center gap-2">
                  <div>
                    <p className="font-bold text-sm">{f.title}</p>
                    <p className="text-xs text-slate-500">
                      {f.status}
                      {f.plannedDate ? ` · ${new Date(f.plannedDate).toLocaleDateString("fr-FR")}` : ""}
                    </p>
                  </div>
                  {canManage && f.status !== "realisee" && (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => patch({ action: "update-formation", formId: f.id, patch: { status: "realisee", completedDate: new Date().toISOString().slice(0, 10) } })}
                      className="text-xs font-bold text-emerald-600 underline"
                    >
                      Marquer réalisée
                    </button>
                  )}
                </li>
              ))}
            </ul>
          )}
          {canManage && (
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                const title = prompt("Intitulé de la formation :");
                if (!title) return;
                const planned = prompt("Date prévue (AAAA-MM-JJ) :") || null;
                void patch({ action: "add-formation", title, plannedDate: planned, status: planned ? "planifiee" : "demandee" });
              }}
              className="text-sm font-bold text-indigo-600 underline"
            >
              + Ajouter une formation
            </button>
          )}
        </div>
      )}

      {tab === "habilitations" && (
        <div className="bg-white rounded-2xl border p-5 space-y-4">
          <h3 className="font-black text-slate-800">Habilitations</h3>
          {record.habilitations.length === 0 ? (
            <p className="text-sm text-slate-400 italic">Aucune habilitation.</p>
          ) : (
            <ul className="space-y-2">
              {record.habilitations.map((h) => (
                <li key={h.id} className="rounded-xl border p-3">
                  <p className="font-bold text-sm">{h.label}</p>
                  <p className="text-xs text-slate-500">
                    Expire le {new Date(h.expiresAt).toLocaleDateString("fr-FR")}
                  </p>
                </li>
              ))}
            </ul>
          )}
          {canManage && (
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                const label = prompt("Type (CACES, SST, etc.) :");
                const expires = prompt("Date d'expiration (AAAA-MM-JJ) :");
                if (!label || !expires) return;
                void patch({ action: "add-habilitation", label, expiresAt: expires });
              }}
              className="text-sm font-bold text-indigo-600 underline"
            >
              + Ajouter une habilitation
            </button>
          )}
        </div>
      )}

      {tab === "medecine" && (
        <div className="bg-white rounded-2xl border p-5 space-y-4">
          <div className="flex items-center justify-between gap-2">
            <h3 className="font-black text-slate-800">Médecine du travail</h3>
            {canManage && (
              <button
                type="button"
                disabled={busy}
                onClick={() => setModal("medecine")}
                className="text-xs font-bold text-emerald-600 underline"
              >
                + Visite
              </button>
            )}
          </div>
          <div className="grid sm:grid-cols-2 gap-4 text-sm">
            <div>
              <p className="text-xs font-bold text-slate-500">Dernière visite</p>
              <p className="font-bold">
                {record.medecineTravail.lastVisitAt
                  ? new Date(record.medecineTravail.lastVisitAt).toLocaleDateString("fr-FR")
                  : "—"}
              </p>
            </div>
            <div>
              <p className="text-xs font-bold text-slate-500">Prochaine visite (cycle 3 ans)</p>
              <p className="font-bold">
                {record.medecineTravail.nextVisitAt
                  ? new Date(record.medecineTravail.nextVisitAt).toLocaleDateString("fr-FR")
                  : "—"}
              </p>
            </div>
          </div>

          <div>
            <p className="text-xs font-bold text-slate-500 mb-2">Historique des visites</p>
            {visits.length === 0 ? (
              <p className="text-sm text-slate-400 italic">Aucune visite enregistrée.</p>
            ) : (
              <ul className="space-y-2">
                {visits.map((v) => (
                  <li key={v.id} className="rounded-xl border p-3">
                    <p className="font-bold text-sm">{new Date(v.visitedAt).toLocaleDateString("fr-FR")}</p>
                    {v.visitType && <p className="text-xs text-slate-600">{v.visitType}</p>}
                    {v.notes && canManage && <p className="text-xs text-slate-400 mt-1">{v.notes}</p>}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      )}

      {tab === "entretiens" && (
        <div className="bg-white rounded-2xl border p-5 space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="font-black text-slate-800">Entretiens professionnels</h3>
            {canManage && (
              <div className="flex gap-2">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => setModal("entretien-realise")}
                  className="text-xs font-bold text-violet-600 underline"
                >
                  + Réalisé
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => setModal("entretien-plan")}
                  className="text-xs font-bold text-indigo-600 underline"
                >
                  Planifier
                </button>
              </div>
            )}
          </div>
          {record.entretiens.length === 0 ? (
            <p className="text-sm text-slate-400 italic">Aucun entretien.</p>
          ) : (
            <ul className="space-y-2">
              {[...record.entretiens]
                .sort((a, b) => {
                  const da = a.completedAt || a.scheduledAt || "";
                  const db = b.completedAt || b.scheduledAt || "";
                  return new Date(db).getTime() - new Date(da).getTime();
                })
                .map((e) => (
                  <li key={e.id} className="rounded-xl border p-3 flex justify-between gap-2">
                    <div>
                      <p className="font-bold text-sm capitalize">{e.status.replace(/_/g, " ")}</p>
                      {e.completedAt && (
                        <p className="text-xs text-slate-500">
                          Réalisé le {new Date(e.completedAt).toLocaleDateString("fr-FR")}
                        </p>
                      )}
                      {e.scheduledAt && e.status !== "realise" && (
                        <p className="text-xs text-slate-500">
                          Prévu le {new Date(e.scheduledAt).toLocaleDateString("fr-FR")}
                        </p>
                      )}
                      {e.nextDueAt && (
                        <p className="text-xs text-violet-600 font-medium mt-0.5">
                          Prochain cycle : {new Date(e.nextDueAt).toLocaleDateString("fr-FR")}
                        </p>
                      )}
                    </div>
                    {canManage && e.status !== "realise" && (
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() =>
                          patch({
                            action: "update-entretien",
                            entId: e.id,
                            patch: { status: "realise", completedAt: new Date().toISOString().slice(0, 10) },
                          })
                        }
                        className="text-xs font-bold text-emerald-600 underline shrink-0"
                      >
                        Marquer réalisé
                      </button>
                    )}
                  </li>
                ))}
            </ul>
          )}
        </div>
      )}

      {tab === "onboarding" && record.onboarding && (
        <div className="bg-white rounded-2xl border p-5 space-y-6">
          <div className="flex items-center justify-between">
            <h3 className="font-black text-slate-800">Onboarding</h3>
            <span className="text-xs font-bold px-3 py-1 rounded-full bg-indigo-100 text-indigo-800 capitalize">
              {record.onboarding.status.replace(/_/g, " ")}
            </span>
          </div>

          <div>
            <p className="text-xs font-bold text-slate-500 mb-2">Checklist</p>
            <ul className="space-y-2">
              {record.onboarding.checklist.map((c) => (
                <li key={c.id} className="flex items-center gap-2">
                  {canManage ? (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => patch({ action: "toggle-checklist", itemId: c.id })}
                      className={`w-5 h-5 rounded border flex items-center justify-center text-xs ${c.done ? "bg-emerald-500 border-emerald-500 text-white" : "border-slate-300"}`}
                    >
                      {c.done ? "✓" : ""}
                    </button>
                  ) : (
                    <span className={`w-5 h-5 rounded flex items-center justify-center text-xs ${c.done ? "bg-emerald-100 text-emerald-700" : "bg-slate-100"}`}>
                      {c.done ? "✓" : "·"}
                    </span>
                  )}
                  <span className={`text-sm ${c.done ? "text-slate-400 line-through" : "text-slate-800"}`}>{c.label}</span>
                </li>
              ))}
            </ul>
          </div>

          <div>
            <p className="text-xs font-bold text-slate-500 mb-2">Signatures (lien e-mail)</p>
            {renderSignatures("onboarding", record.onboarding.signatures)}
          </div>
        </div>
      )}

      {tab === "offboarding" && (
        <div className="bg-white rounded-2xl border p-5 space-y-6">
          {!record.offboarding ? (
            canManage ? (
              <div className="space-y-3">
                <p className="text-sm text-slate-600">Aucun parcours de sortie en cours.</p>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => {
                    const endDate = prompt("Date de fin de contrat (AAAA-MM-JJ) :");
                    if (endDate) patch({ action: "start-offboarding", endDate });
                  }}
                  className="bg-slate-800 text-white px-4 py-2 rounded-xl font-bold text-sm"
                >
                  Démarrer l&apos;offboarding
                </button>
              </div>
            ) : (
              <p className="text-sm text-slate-500">Pas d&apos;offboarding.</p>
            )
          ) : (
            <>
              <div className="flex items-center justify-between">
                <h3 className="font-black text-slate-800">Offboarding</h3>
                <span className="text-xs font-bold px-3 py-1 rounded-full bg-slate-100 capitalize">
                  {record.offboarding.status.replace(/_/g, " ")} · fin {record.offboarding.endDate}
                </span>
              </div>
              <div>
                <p className="text-xs font-bold text-slate-500 mb-2">Checklist</p>
                <ul className="space-y-2">
                  {record.offboarding.checklist.map((c) => (
                    <li key={c.id} className="flex items-center gap-2">
                      {canManage ? (
                        <button
                          type="button"
                          disabled={busy}
                          onClick={() => patch({ action: "toggle-offboarding-checklist", itemId: c.id })}
                          className={`w-5 h-5 rounded border flex items-center justify-center text-xs ${c.done ? "bg-emerald-500 border-emerald-500 text-white" : "border-slate-300"}`}
                        >
                          {c.done ? "✓" : ""}
                        </button>
                      ) : (
                        <span className={`w-5 h-5 rounded flex items-center justify-center text-xs ${c.done ? "bg-emerald-100 text-emerald-700" : "bg-slate-100"}`}>
                          {c.done ? "✓" : "·"}
                        </span>
                      )}
                      <span className={`text-sm ${c.done ? "text-slate-400 line-through" : "text-slate-800"}`}>{c.label}</span>
                    </li>
                  ))}
                </ul>
              </div>
              <div>
                <p className="text-xs font-bold text-slate-500 mb-2">Signatures (lien e-mail)</p>
                {renderSignatures("offboarding", record.offboarding.signatures)}
              </div>
            </>
          )}
        </div>
      )}

      {modal === "medecine" && (
        <RhModalShell title="Enregistrer une visite médicale" onClose={() => setModal(null)}>
          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              const fd = new FormData(e.currentTarget);
              void patch({
                action: "add-medecine-visit",
                visitedAt: fd.get("visitedAt"),
                visitType: fd.get("visitType"),
                notes: fd.get("notes"),
              });
            }}
          >
            <RhField label="Date de visite">
              <RhInput name="visitedAt" type="date" required defaultValue={new Date().toISOString().slice(0, 10)} />
            </RhField>
            <RhField label="Type de visite">
              <RhInput name="visitType" placeholder="Visite périodique, reprise…" />
            </RhField>
            <RhField label="Notes (RH)">
              <RhTextarea name="notes" placeholder="Optionnel" />
            </RhField>
            <p className="text-xs text-slate-500">La prochaine échéance sera calculée automatiquement (+3 ans).</p>
            <div className="flex gap-2 pt-2">
              <button type="button" onClick={() => setModal(null)} className="flex-1 py-2.5 rounded-xl border font-bold text-sm text-slate-600">
                Annuler
              </button>
              <button type="submit" disabled={busy} className="flex-1 py-2.5 rounded-xl bg-emerald-600 text-white font-bold text-sm disabled:opacity-50">
                Enregistrer
              </button>
            </div>
          </form>
        </RhModalShell>
      )}

      {modal === "entretien-plan" && (
        <RhModalShell title="Planifier un entretien" onClose={() => setModal(null)}>
          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              const fd = new FormData(e.currentTarget);
              const date = String(fd.get("scheduledAt") || "");
              void patch({
                action: "add-entretien",
                status: date ? "planifie" : "a_planifier",
                scheduledAt: date || null,
                notes: fd.get("notes"),
              });
            }}
          >
            <RhField label="Date prévue">
              <RhInput name="scheduledAt" type="date" />
            </RhField>
            <RhField label="Notes">
              <RhTextarea name="notes" placeholder="Optionnel" />
            </RhField>
            <div className="flex gap-2 pt-2">
              <button type="button" onClick={() => setModal(null)} className="flex-1 py-2.5 rounded-xl border font-bold text-sm text-slate-600">
                Annuler
              </button>
              <button type="submit" disabled={busy} className="flex-1 py-2.5 rounded-xl bg-indigo-600 text-white font-bold text-sm disabled:opacity-50">
                Planifier
              </button>
            </div>
          </form>
        </RhModalShell>
      )}

      {modal === "entretien-realise" && (
        <RhModalShell title="Enregistrer un entretien réalisé" onClose={() => setModal(null)}>
          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              const fd = new FormData(e.currentTarget);
              void patch({
                action: "add-entretien-realise",
                completedAt: fd.get("completedAt"),
                notes: fd.get("notes"),
              });
            }}
          >
            <RhField label="Date de réalisation">
              <RhInput name="completedAt" type="date" required defaultValue={new Date().toISOString().slice(0, 10)} />
            </RhField>
            <RhField label="Notes">
              <RhTextarea name="notes" placeholder="Optionnel" />
            </RhField>
            <p className="text-xs text-slate-500">Le prochain cycle sera fixé à +3 ans.</p>
            <div className="flex gap-2 pt-2">
              <button type="button" onClick={() => setModal(null)} className="flex-1 py-2.5 rounded-xl border font-bold text-sm text-slate-600">
                Annuler
              </button>
              <button type="submit" disabled={busy} className="flex-1 py-2.5 rounded-xl bg-violet-600 text-white font-bold text-sm disabled:opacity-50">
                Enregistrer
              </button>
            </div>
          </form>
        </RhModalShell>
      )}
    </div>
  );
}
