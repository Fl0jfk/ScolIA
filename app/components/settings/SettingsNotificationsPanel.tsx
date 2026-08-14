"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { SettingsSection, settingsInputClass } from "@/app/components/settings/SettingsChrome";

export default function SettingsNotificationsPanel({
  notifications,
  setNotifications,
  activeEstablishmentKinds,
  saving,
  saveSection,
}: {
  notifications: Record<string, unknown>;
  setNotifications: Dispatch<SetStateAction<Record<string, unknown>>>;
  activeEstablishmentKinds: Set<string>;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  return (
    <SettingsSection icon="✉️" title="Notifications" className="space-y-4">
      <label className="block text-sm font-bold">Emails compta voyages (séparés par virgule)</label>
      <input
        className={settingsInputClass}
        value={Array.isArray(notifications.travelsCompta) ? (notifications.travelsCompta as string[]).join(", ") : ""}
        onChange={(e) =>
          setNotifications({
            ...notifications,
            travelsCompta: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
          })
        }
      />
      <label className="block text-sm font-bold">Email cuisine / restauration</label>
      <input
        className={settingsInputClass}
        value={String(notifications.travelsCuisine || "")}
        onChange={(e) => setNotifications({ ...notifications, travelsCuisine: e.target.value })}
      />
      <label className="block text-sm font-bold">Gestionnaire HSE (e-mail)</label>
      <input
        className={settingsInputClass}
        value={String(notifications.hseOps || "")}
        onChange={(e) => setNotifications({ ...notifications, hseOps: e.target.value })}
      />
      <label className="block text-sm font-bold">Gestionnaire photocopies couleur (e-mail)</label>
      <input
        className={settingsInputClass}
        value={String(notifications.photocopiesOps || "")}
        onChange={(e) => setNotifications({ ...notifications, photocopiesOps: e.target.value })}
      />
      <label className="block text-sm font-bold">Email Zeendoc / envoi PDF voyages</label>
      <input
        className={settingsInputClass}
        value={String(notifications.travelsZeendoc || "")}
        onChange={(e) => setNotifications({ ...notifications, travelsZeendoc: e.target.value })}
      />
      <hr className="border-slate-200" />
      <p className="text-sm font-black text-slate-800">Absences — notifications après validation direction</p>
      {activeEstablishmentKinds.has("ecole") && (
        <>
          <label className="block text-sm font-bold">Professeurs — école (nom)</label>
          <input
            className={`${settingsInputClass} mb-2`}
            value={String((notifications.absencesNotifyProfEcole as { label?: string })?.label || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyProfEcole: {
                  ...((notifications.absencesNotifyProfEcole as object) || {}),
                  label: e.target.value,
                  email: String((notifications.absencesNotifyProfEcole as { email?: string })?.email || ""),
                },
              })
            }
          />
          <label className="block text-sm font-bold">Professeurs — école (e-mail)</label>
          <input
            className={settingsInputClass}
            type="email"
            value={String((notifications.absencesNotifyProfEcole as { email?: string })?.email || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyProfEcole: {
                  label: String((notifications.absencesNotifyProfEcole as { label?: string })?.label || ""),
                  email: e.target.value,
                },
              })
            }
          />
        </>
      )}
      {activeEstablishmentKinds.has("college") && (
        <>
          <label className="block text-sm font-bold">Professeurs — collège (nom)</label>
          <input
            className={`${settingsInputClass} mb-2`}
            value={String((notifications.absencesNotifyProfCollege as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyProfCollege: {
                  label: e.target.value,
                  email: String((notifications.absencesNotifyProfCollege as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || ""),
                },
              })
            }
          />
          <label className="block text-sm font-bold">Professeurs — collège (e-mail)</label>
          <input
            className={settingsInputClass}
            type="email"
            value={String((notifications.absencesNotifyProfCollege as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyProfCollege: {
                  label: String((notifications.absencesNotifyProfCollege as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || ""),
                  email: e.target.value,
                },
              })
            }
          />
        </>
      )}
      {activeEstablishmentKinds.has("lycee") && (
        <>
          <label className="block text-sm font-bold">Professeurs — lycée (nom)</label>
          <input
            className={`${settingsInputClass} mb-2`}
            value={String((notifications.absencesNotifyProfLycee as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyProfLycee: {
                  label: e.target.value,
                  email: String((notifications.absencesNotifyProfLycee as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || ""),
                },
              })
            }
          />
          <label className="block text-sm font-bold">Professeurs — lycée (e-mail)</label>
          <input
            className={settingsInputClass}
            type="email"
            value={String((notifications.absencesNotifyProfLycee as { email?: string })?.email || (notifications.absencesNotifyProfCollegeLycee as { email?: string })?.email || "")}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyProfLycee: {
                  label: String((notifications.absencesNotifyProfLycee as { label?: string })?.label || (notifications.absencesNotifyProfCollegeLycee as { label?: string })?.label || ""),
                  email: e.target.value,
                },
              })
            }
          />
        </>
      )}
      <label className="block text-sm font-bold">Personnel OGEC, administratif & RH (e-mails séparés par virgule)</label>
      <input
        className={settingsInputClass}
        value={
          Array.isArray(notifications.absencesNotifyOgecCompta)
            ? (notifications.absencesNotifyOgecCompta as string[]).join(", ")
            : ""
        }
        onChange={(e) =>
          setNotifications({
            ...notifications,
            absencesNotifyOgecCompta: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
          })
        }
      />
      <hr className="border-slate-200" />
      <p className="text-sm font-black text-slate-800">Internat — appel du soir (validation)</p>
      <label className="block text-sm font-bold">Qui reçoit l&apos;appel ? (e-mail)</label>
      <input
        className={settingsInputClass}
        value={String(
          (notifications.internatRollCallRecipients as { appelContact?: string; directionLycee?: string })?.appelContact ||
            (notifications.internatRollCallRecipients as { directionLycee?: string })?.directionLycee ||
            "",
        )}
        onChange={(e) =>
          setNotifications({
            ...notifications,
            internatRollCallRecipients: {
              ...((notifications.internatRollCallRecipients as object) || {}),
              appelContact: e.target.value,
            },
          })
        }
      />
      <label className="block text-sm font-bold">CPE lycée (optionnel)</label>
      <input
        className={settingsInputClass}
        value={String((notifications.internatRollCallRecipients as { cpeLycee?: string })?.cpeLycee || "")}
        onChange={(e) =>
          setNotifications({
            ...notifications,
            internatRollCallRecipients: {
              ...((notifications.internatRollCallRecipients as object) || {}),
              cpeLycee: e.target.value,
            },
          })
        }
      />
      <label className="block text-sm font-bold">CPE collège (optionnel)</label>
      <input
        className={settingsInputClass}
        value={String((notifications.internatRollCallRecipients as { cpeCollege?: string })?.cpeCollege || "")}
        onChange={(e) =>
          setNotifications({
            ...notifications,
            internatRollCallRecipients: {
              ...((notifications.internatRollCallRecipients as object) || {}),
              cpeCollege: e.target.value,
            },
          })
        }
      />
      <label className="block text-sm font-bold">Internat — alertes urgence (emails séparés par virgule)</label>
      <input
        className={settingsInputClass}
        value={
          Array.isArray(notifications.internatEmergencyRecipients)
            ? (notifications.internatEmergencyRecipients as string[]).join(", ")
            : ""
        }
        onChange={(e) =>
          setNotifications({
            ...notifications,
            internatEmergencyRecipients: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
          })
        }
      />
      <label className="block text-sm font-bold">Stages — e-mails administratif (séparés par virgule)</label>
      <input
        className={settingsInputClass}
        value={
          Array.isArray(notifications.stagesAdminEmails)
            ? (notifications.stagesAdminEmails as string[]).join(", ")
            : ""
        }
        onChange={(e) =>
          setNotifications({
            ...notifications,
            stagesAdminEmails: e.target.value.split(",").map((s) => s.trim()).filter(Boolean),
          })
        }
      />
      <label className="block text-sm font-bold">Stages — e-mail direction (signature)</label>
      <input
        className={settingsInputClass}
        type="email"
        value={String(notifications.stagesDirectionEmail || "")}
        onChange={(e) =>
          setNotifications({ ...notifications, stagesDirectionEmail: e.target.value.trim() })
        }
        placeholder="directeur@… (sinon e-mail directeur par établissement)"
      />
      <label className="block text-sm font-bold">Stages — modèle convention vierge (URL PDF)</label>
      <input
        className={settingsInputClass}
        type="url"
        value={String(notifications.stagesConventionTemplateUrl || "")}
        onChange={(e) =>
          setNotifications({ ...notifications, stagesConventionTemplateUrl: e.target.value.trim() })
        }
        placeholder="https://…/convention-stage-vierge.pdf"
      />
      <p className="text-xs text-slate-500">
        PDF remplissable (Adobe) hébergé sur S3 ou autre — lien affiché sur /stages/deposer.
      </p>
      <ModuleButton
        variant="primary"
        disabled={saving}
        onClick={() => saveSection("notifications", notifications)}
      >
        Enregistrer les notifications
      </ModuleButton>
    </SettingsSection>
  );
}
