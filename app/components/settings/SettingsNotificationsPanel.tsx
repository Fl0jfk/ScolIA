"use client";

import type { Dispatch, SetStateAction } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ClerkPersonSelect, {
  ClerkPeopleSelect,
  clerkMemberLabel,
} from "@/app/components/settings/ClerkPersonSelect";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";
import { SettingsField, SettingsSection, settingsInputClass } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

type NotifyPerson = { label?: string; email: string };

function asNotify(value: unknown): NotifyPerson | undefined {
  if (!value || typeof value !== "object") return undefined;
  const email = String((value as { email?: string }).email || "").trim();
  if (!email) return undefined;
  return {
    label: String((value as { label?: string }).label || "").trim() || undefined,
    email,
  };
}

function emailsOf(value: unknown): string[] {
  return Array.isArray(value) ? value.map((s) => String(s).trim()).filter(Boolean) : [];
}

export default function SettingsNotificationsPanel({
  notifications,
  setNotifications,
  activeEstablishmentKinds,
  clerkMembers,
  membersLoading,
  saving,
  saveSection,
}: {
  notifications: Record<string, unknown>;
  setNotifications: Dispatch<SetStateAction<Record<string, unknown>>>;
  activeEstablishmentKinds: Set<string>;
  clerkMembers: ClerkMemberOption[];
  membersLoading: boolean;
  saving: boolean;
  saveSection: (section: string, body: unknown) => Promise<void>;
}) {
  const patch = (next: Record<string, unknown>) => setNotifications({ ...notifications, ...next });

  const setPersonEmail = (key: string, member: ClerkMemberOption | null) => {
    patch({ [key]: member?.email.trim() || undefined });
  };

  const setNotifyPerson = (key: string, member: ClerkMemberOption | null) => {
    if (!member) {
      patch({ [key]: undefined });
      return;
    }
    patch({
      [key]: { label: clerkMemberLabel(member), email: member.email.trim() },
    });
  };

  const internat = (notifications.internatRollCallRecipients as Record<string, string> | undefined) || {};
  const patchInternat = (field: string, member: ClerkMemberOption | null) => {
    patch({
      internatRollCallRecipients: {
        ...internat,
        [field]: member?.email.trim() || undefined,
      },
    });
  };

  const profEcole =
    asNotify(notifications.absencesNotifyProfEcole);
  const profCollege =
    asNotify(notifications.absencesNotifyProfCollege) || asNotify(notifications.absencesNotifyProfCollegeLycee);
  const profLycee =
    asNotify(notifications.absencesNotifyProfLycee) || asNotify(notifications.absencesNotifyProfCollegeLycee);

  const showInternat = activeEstablishmentKinds.has("college") || activeEstablishmentKinds.has("lycee");

  return (
    <div className="space-y-4">
      <SettingsSection
        icon="✉️"
        title="Destinataires"
        description="Choisissez les personnes dans le personnel : l’e-mail principal Clerk est utilisé pour les notifications."
      >
        <p className={`text-xs ${dash.textMid}`}>
          Plus besoin de saisir un nom ou une adresse à la main.
        </p>
      </SettingsSection>

      <SettingsSection icon="🚌" title="Voyages">
        <SettingsField label="Comptabilité" hint="Une ou plusieurs personnes." as="div">
          <ClerkPeopleSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmails={emailsOf(notifications.travelsCompta)}
            onChange={(emails) => patch({ travelsCompta: emails })}
          />
        </SettingsField>
        <SettingsField label="Cuisine / restauration" as="div">
          <ClerkPersonSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmail={String(notifications.travelsCuisine || "")}
            onChange={(member) => setPersonEmail("travelsCuisine", member)}
          />
        </SettingsField>
        <SettingsField label="Zeendoc / envoi PDF" as="div">
          <ClerkPersonSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmail={String(notifications.travelsZeendoc || "")}
            onChange={(member) => setPersonEmail("travelsZeendoc", member)}
          />
        </SettingsField>
      </SettingsSection>

      <SettingsSection icon="🛡️" title="HSE et photocopies">
        <SettingsField label="Gestionnaire HSE" as="div">
          <ClerkPersonSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmail={String(notifications.hseOps || "")}
            onChange={(member) => setPersonEmail("hseOps", member)}
          />
        </SettingsField>
        <SettingsField label="Gestionnaire photocopies couleur" as="div">
          <ClerkPersonSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmail={String(notifications.photocopiesOps || "")}
            onChange={(member) => setPersonEmail("photocopiesOps", member)}
          />
        </SettingsField>
      </SettingsSection>

      <SettingsSection
        icon="🗓️"
        title="Absences"
        description="Notifications après validation direction."
      >
        {activeEstablishmentKinds.has("ecole") ? (
          <SettingsField label="Professeurs — école" as="div">
            <ClerkPersonSelect
              members={clerkMembers}
              loading={membersLoading}
              selectedEmail={profEcole?.email}
              onChange={(member) => setNotifyPerson("absencesNotifyProfEcole", member)}
            />
          </SettingsField>
        ) : null}
        {activeEstablishmentKinds.has("college") ? (
          <SettingsField label="Professeurs — collège" as="div">
            <ClerkPersonSelect
              members={clerkMembers}
              loading={membersLoading}
              selectedEmail={profCollege?.email}
              onChange={(member) => setNotifyPerson("absencesNotifyProfCollege", member)}
            />
          </SettingsField>
        ) : null}
        {activeEstablishmentKinds.has("lycee") ? (
          <SettingsField label="Professeurs — lycée" as="div">
            <ClerkPersonSelect
              members={clerkMembers}
              loading={membersLoading}
              selectedEmail={profLycee?.email}
              onChange={(member) => setNotifyPerson("absencesNotifyProfLycee", member)}
            />
          </SettingsField>
        ) : null}
        <SettingsField label="Personnel OGEC, administratif & RH" as="div">
          <ClerkPeopleSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmails={emailsOf(notifications.absencesNotifyOgecCompta)}
            onChange={(emails) => patch({ absencesNotifyOgecCompta: emails })}
          />
        </SettingsField>
      </SettingsSection>

      {showInternat ? (
        <SettingsSection icon="🌙" title="Internat" description="Appel du soir et alertes urgence.">
          <SettingsField label="Qui reçoit l’appel ?" as="div">
            <ClerkPersonSelect
              members={clerkMembers}
              loading={membersLoading}
              selectedEmail={internat.appelContact || internat.directionLycee || ""}
              onChange={(member) => patchInternat("appelContact", member)}
            />
          </SettingsField>
          {activeEstablishmentKinds.has("lycee") ? (
            <SettingsField label="CPE lycée (optionnel)" as="div">
              <ClerkPersonSelect
                members={clerkMembers}
                loading={membersLoading}
                selectedEmail={internat.cpeLycee || ""}
                onChange={(member) => patchInternat("cpeLycee", member)}
              />
            </SettingsField>
          ) : null}
          {activeEstablishmentKinds.has("college") ? (
            <SettingsField label="CPE collège (optionnel)" as="div">
              <ClerkPersonSelect
                members={clerkMembers}
                loading={membersLoading}
                selectedEmail={internat.cpeCollege || ""}
                onChange={(member) => patchInternat("cpeCollege", member)}
              />
            </SettingsField>
          ) : null}
          <SettingsField label="Alertes urgence" as="div">
            <ClerkPeopleSelect
              members={clerkMembers}
              loading={membersLoading}
              selectedEmails={emailsOf(notifications.internatEmergencyRecipients)}
              onChange={(emails) => patch({ internatEmergencyRecipients: emails })}
            />
          </SettingsField>
        </SettingsSection>
      ) : null}

      <SettingsSection icon="📄" title="Stages">
        <SettingsField label="Administratif" as="div">
          <ClerkPeopleSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmails={emailsOf(notifications.stagesAdminEmails)}
            onChange={(emails) => patch({ stagesAdminEmails: emails })}
          />
        </SettingsField>
        <SettingsField
          label="Direction (signature)"
          hint="Sinon, e-mail du directeur de l’établissement de l’élève."
          as="div"
        >
          <ClerkPersonSelect
            members={clerkMembers}
            loading={membersLoading}
            selectedEmail={String(notifications.stagesDirectionEmail || "")}
            onChange={(member) => setPersonEmail("stagesDirectionEmail", member)}
          />
        </SettingsField>
        <SettingsField label="Modèle de convention vierge (URL PDF)">
          <input
            className={settingsInputClass}
            type="url"
            value={String(notifications.stagesConventionTemplateUrl || "")}
            onChange={(e) => patch({ stagesConventionTemplateUrl: e.target.value.trim() })}
            placeholder="https://…/convention-stage-vierge.pdf"
          />
        </SettingsField>
        <p className={`text-xs ${dash.textMid}`}>
          PDF remplissable (Adobe) hébergé sur S3 ou autre — lien affiché sur /stages/deposer.
        </p>
      </SettingsSection>

      <div className="flex justify-end">
        <ModuleButton
          variant="primary"
          disabled={saving}
          className="rounded-2xl px-5 shadow-[0_12px_28px_-16px_rgba(15,23,42,0.55)]"
          onClick={() => saveSection("notifications", notifications)}
        >
          {saving ? "Enregistrement…" : "Enregistrer les notifications"}
        </ModuleButton>
      </div>
    </div>
  );
}
