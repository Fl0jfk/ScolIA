"use client";

import { useState, type ReactNode } from "react";
import type {
  EstablishmentKind,
  ExternalQuickLinkConfig,
  IntegrationsConfig,
  NotificationsConfig,
  TravelsModuleConfig,
} from "@/app/lib/app-config-schemas";
import { OnboardingField, onboardingInputClass } from "@/app/components/onboarding/OnboardingShell";
import { newQuickLinkSlot } from "@/app/lib/dashboard-quick-links";
import { dash } from "@/app/lib/dashboard-brand";

function Accordion({
  title,
  description,
  open,
  onToggle,
  children,
}: {
  title: string;
  description: string;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  return (
    <div className="mb-3 overflow-hidden rounded-2xl border border-white/70 bg-white/50">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-start justify-between gap-3 px-4 py-3.5 text-left"
      >
        <div>
          <span className={`block text-sm font-semibold ${dash.ink}`}>{title}</span>
          <span className={`mt-0.5 block text-xs ${dash.textMid}`}>{description}</span>
        </div>
        <span className={`mt-0.5 text-lg font-light ${dash.textMid}`}>{open ? "−" : "+"}</span>
      </button>
      {open ? <div className="border-t border-white/60 px-4 pb-4 pt-3">{children}</div> : null}
    </div>
  );
}

type Props = {
  notifications: Partial<NotificationsConfig>;
  setNotifications: (n: Partial<NotificationsConfig>) => void;
  travels: Partial<TravelsModuleConfig>;
  setTravels: (t: Partial<TravelsModuleConfig>) => void;
  integrations: IntegrationsConfig;
  setIntegrations: (i: IntegrationsConfig) => void;
  externalLinks: ExternalQuickLinkConfig[];
  setExternalLinks: (l: ExternalQuickLinkConfig[]) => void;
  wantQuickLinks: boolean;
  setWantQuickLinks: (v: boolean) => void;
  hasInternat: boolean;
  setHasInternat: (v: boolean) => void;
  activeEstablishmentKinds: Set<EstablishmentKind>;
};

export default function ChapterContacts(props: Props) {
  const {
    notifications,
    setNotifications,
    travels,
    setTravels,
    integrations,
    setIntegrations,
    externalLinks,
    setExternalLinks,
    wantQuickLinks,
    setWantQuickLinks,
    hasInternat,
    setHasInternat,
    activeEstablishmentKinds,
  } = props;

  const [open, setOpen] = useState<Record<string, boolean>>({
    hse: true,
    travels: false,
    zeendoc: false,
    absences: false,
    internat: false,
    links: false,
    onedrive: false,
  });

  const toggle = (key: string) => setOpen((o) => ({ ...o, [key]: !o[key] }));

  return (
    <div>
      <p className={`mb-4 text-sm leading-relaxed ${dash.textMid}`}>
        Configurez les destinataires et modules utiles. Chaque section est optionnelle — vous pourrez
        tout affiner dans Paramètres.
      </p>

      <Accordion
        title="HSE & photocopies"
        description="E-mails après validation direction"
        open={open.hse}
        onToggle={() => toggle("hse")}
      >
        <OnboardingField label="Gestionnaire HSE">
          <input
            className={onboardingInputClass}
            type="email"
            value={notifications.hseOps || ""}
            onChange={(e) => setNotifications({ ...notifications, hseOps: e.target.value })}
          />
        </OnboardingField>
        <OnboardingField label="Réceptionnaires photocopies (e-mails, virgules)">
          <input
            className={onboardingInputClass}
            type="text"
            placeholder="personne1@etab.fr, personne2@etab.fr"
            value={
              Array.isArray(notifications.photocopiesOpsEmails)
                ? notifications.photocopiesOpsEmails.join(", ")
                : notifications.photocopiesOps || ""
            }
            onChange={(e) => {
              const emails = e.target.value
                .split(",")
                .map((s) => s.trim())
                .filter(Boolean);
              setNotifications({
                ...notifications,
                photocopiesOpsEmails: emails,
                photocopiesOps: emails[0] || "",
              });
            }}
          />
        </OnboardingField>
      </Accordion>

      <Accordion
        title="Sorties scolaires"
        description="Compta, cuisine, transporteurs"
        open={open.travels}
        onToggle={() => toggle("travels")}
      >
        <OnboardingField label="E-mails comptabilité (virgules)">
          <input
            className={onboardingInputClass}
            value={Array.isArray(notifications.travelsCompta) ? notifications.travelsCompta.join(", ") : ""}
            onChange={(e) =>
              setNotifications({
                ...notifications,
                travelsCompta: e.target.value
                  .split(",")
                  .map((s) => s.trim())
                  .filter(Boolean),
              })
            }
          />
        </OnboardingField>
        <OnboardingField label="E-mail cuisine / restauration">
          <input
            className={onboardingInputClass}
            type="email"
            value={notifications.travelsCuisine || ""}
            onChange={(e) => setNotifications({ ...notifications, travelsCuisine: e.target.value })}
          />
        </OnboardingField>
        <p className={`mb-2 text-sm font-medium ${dash.ink}`}>Transporteurs habituels</p>
        {(travels.transportProviders || []).map((p, idx) => (
          <div key={idx} className="mb-2 grid grid-cols-2 gap-2">
            <input
              className={onboardingInputClass}
              placeholder="Nom"
              value={p.name}
              onChange={(e) => {
                const copy = [...(travels.transportProviders || [])];
                copy[idx] = { ...copy[idx], name: e.target.value };
                setTravels({ ...travels, transportProviders: copy });
              }}
            />
            <input
              className={onboardingInputClass}
              placeholder="E-mail"
              type="email"
              value={p.email}
              onChange={(e) => {
                const copy = [...(travels.transportProviders || [])];
                copy[idx] = { ...copy[idx], email: e.target.value };
                setTravels({ ...travels, transportProviders: copy });
              }}
            />
          </div>
        ))}
        <button
          type="button"
          className={`text-sm font-semibold ${dash.linkBold}`}
          onClick={() =>
            setTravels({
              ...travels,
              transportProviders: [...(travels.transportProviders || []), { name: "", email: "" }],
            })
          }
        >
          + Ajouter un transporteur
        </button>
      </Accordion>

      <Accordion
        title="Envoi documents (Zeendoc)"
        description="Destination des PDF voyages"
        open={open.zeendoc}
        onToggle={() => toggle("zeendoc")}
      >
        <OnboardingField label="Utilisez-vous Zeendoc ?">
          <select
            className={onboardingInputClass}
            value={integrations.zeendoc?.enabled ? "yes" : "no"}
            onChange={(e) =>
              setIntegrations({
                ...integrations,
                zeendoc: {
                  enabled: e.target.value === "yes",
                  buttonLabel:
                    e.target.value === "yes" ? "Envoyer sur Zeendoc" : "Envoyer par mail",
                  destinationEmail: integrations.zeendoc?.destinationEmail,
                },
              })
            }
          >
            <option value="no">Non — envoi par mail</option>
            <option value="yes">Oui — Zeendoc</option>
          </select>
        </OnboardingField>
        <OnboardingField label="Libellé du bouton">
          <input
            className={onboardingInputClass}
            value={integrations.zeendoc?.buttonLabel || "Envoyer par mail"}
            onChange={(e) =>
              setIntegrations({
                ...integrations,
                zeendoc: {
                  ...integrations.zeendoc,
                  enabled: integrations.zeendoc?.enabled ?? false,
                  buttonLabel: e.target.value,
                },
              })
            }
          />
        </OnboardingField>
        <OnboardingField label="E-mail de destination des PDF">
          <input
            className={onboardingInputClass}
            type="email"
            value={
              integrations.zeendoc?.destinationEmail || notifications.travelsZeendoc || ""
            }
            onChange={(e) =>
              setIntegrations({
                ...integrations,
                zeendoc: {
                  ...integrations.zeendoc,
                  enabled: integrations.zeendoc?.enabled ?? false,
                  destinationEmail: e.target.value,
                },
              })
            }
          />
        </OnboardingField>
      </Accordion>

      <Accordion
        title="Absences"
        description="Notifications professeurs & OGEC"
        open={open.absences}
        onToggle={() => toggle("absences")}
      >
        {activeEstablishmentKinds.has("ecole") && (
          <OnboardingField label="Professeurs — école">
            <input
              className={`${onboardingInputClass} mb-2`}
              placeholder="Nom"
              value={notifications.absencesNotifyProfEcole?.label || ""}
              onChange={(e) =>
                setNotifications({
                  ...notifications,
                  absencesNotifyProfEcole: {
                    label: e.target.value,
                    email: notifications.absencesNotifyProfEcole?.email || "",
                  },
                })
              }
            />
            <input
              className={onboardingInputClass}
              placeholder="E-mail"
              type="email"
              value={notifications.absencesNotifyProfEcole?.email || ""}
              onChange={(e) =>
                setNotifications({
                  ...notifications,
                  absencesNotifyProfEcole: {
                    label: notifications.absencesNotifyProfEcole?.label,
                    email: e.target.value,
                  },
                })
              }
            />
          </OnboardingField>
        )}
        {activeEstablishmentKinds.has("college") && (
          <OnboardingField label="Professeurs — collège">
            <input
              className={`${onboardingInputClass} mb-2`}
              placeholder="Nom"
              value={
                notifications.absencesNotifyProfCollege?.label ||
                notifications.absencesNotifyProfCollegeLycee?.label ||
                ""
              }
              onChange={(e) =>
                setNotifications({
                  ...notifications,
                  absencesNotifyProfCollege: {
                    label: e.target.value,
                    email:
                      notifications.absencesNotifyProfCollege?.email ||
                      notifications.absencesNotifyProfCollegeLycee?.email ||
                      "",
                  },
                })
              }
            />
            <input
              className={onboardingInputClass}
              placeholder="E-mail"
              type="email"
              value={
                notifications.absencesNotifyProfCollege?.email ||
                notifications.absencesNotifyProfCollegeLycee?.email ||
                ""
              }
              onChange={(e) =>
                setNotifications({
                  ...notifications,
                  absencesNotifyProfCollege: {
                    label:
                      notifications.absencesNotifyProfCollege?.label ||
                      notifications.absencesNotifyProfCollegeLycee?.label,
                    email: e.target.value,
                  },
                })
              }
            />
          </OnboardingField>
        )}
        {activeEstablishmentKinds.has("lycee") && (
          <OnboardingField label="Professeurs — lycée">
            <input
              className={`${onboardingInputClass} mb-2`}
              placeholder="Nom"
              value={
                notifications.absencesNotifyProfLycee?.label ||
                notifications.absencesNotifyProfCollegeLycee?.label ||
                ""
              }
              onChange={(e) =>
                setNotifications({
                  ...notifications,
                  absencesNotifyProfLycee: {
                    label: e.target.value,
                    email:
                      notifications.absencesNotifyProfLycee?.email ||
                      notifications.absencesNotifyProfCollegeLycee?.email ||
                      "",
                  },
                })
              }
            />
            <input
              className={onboardingInputClass}
              placeholder="E-mail"
              type="email"
              value={
                notifications.absencesNotifyProfLycee?.email ||
                notifications.absencesNotifyProfCollegeLycee?.email ||
                ""
              }
              onChange={(e) =>
                setNotifications({
                  ...notifications,
                  absencesNotifyProfLycee: {
                    label:
                      notifications.absencesNotifyProfLycee?.label ||
                      notifications.absencesNotifyProfCollegeLycee?.label,
                    email: e.target.value,
                  },
                })
              }
            />
          </OnboardingField>
        )}
        {!activeEstablishmentKinds.has("ecole") &&
          !activeEstablishmentKinds.has("college") &&
          !activeEstablishmentKinds.has("lycee") && (
            <p className="mb-3 rounded-xl border border-amber-200 bg-amber-50/90 px-3 py-2 text-sm text-amber-900">
              Ajoutez un établissement au chapitre Structure pour configurer les notifications
              professeurs.
            </p>
          )}
        <OnboardingField label="OGEC / administratif / RH (virgules)">
          <input
            className={onboardingInputClass}
            value={
              Array.isArray(notifications.absencesNotifyOgecCompta)
                ? notifications.absencesNotifyOgecCompta.join(", ")
                : ""
            }
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifyOgecCompta: e.target.value
                  .split(",")
                  .map((s) => s.trim())
                  .filter(Boolean),
              })
            }
          />
        </OnboardingField>
        <OnboardingField label="Responsables des surveillants (virgules)">
          <input
            className={onboardingInputClass}
            placeholder="Notifiés si absence d'un personnel Surveillant"
            value={
              Array.isArray(notifications.absencesNotifySurveillanceResponsables)
                ? notifications.absencesNotifySurveillanceResponsables.join(", ")
                : ""
            }
            onChange={(e) =>
              setNotifications({
                ...notifications,
                absencesNotifySurveillanceResponsables: e.target.value
                  .split(",")
                  .map((s) => s.trim())
                  .filter(Boolean),
              })
            }
          />
        </OnboardingField>
      </Accordion>

      <Accordion
        title="Internat"
        description="Appel et urgences"
        open={open.internat}
        onToggle={() => toggle("internat")}
      >
        <label className={`mb-4 flex items-center gap-2 text-sm ${dash.ink}`}>
          <input
            type="checkbox"
            checked={hasInternat}
            onChange={(e) => setHasInternat(e.target.checked)}
          />
          Nous avons un internat
        </label>
        {hasInternat ? (
          <>
            <OnboardingField label="Qui reçoit l'appel ?">
              <input
                className={onboardingInputClass}
                type="email"
                value={
                  notifications.internatRollCallRecipients?.appelContact ||
                  notifications.internatRollCallRecipients?.directionLycee ||
                  ""
                }
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    internatRollCallRecipients: {
                      ...notifications.internatRollCallRecipients,
                      appelContact: e.target.value,
                    },
                  })
                }
              />
            </OnboardingField>
            <OnboardingField label="Urgences internat (virgules)">
              <input
                className={onboardingInputClass}
                value={
                  Array.isArray(notifications.internatEmergencyRecipients)
                    ? notifications.internatEmergencyRecipients.join(", ")
                    : ""
                }
                onChange={(e) =>
                  setNotifications({
                    ...notifications,
                    internatEmergencyRecipients: e.target.value
                      .split(",")
                      .map((s) => s.trim())
                      .filter(Boolean),
                  })
                }
              />
            </OnboardingField>
          </>
        ) : null}
      </Accordion>

      <Accordion
        title="Raccourcis tableau de bord"
        description="ÉcoleDirecte, Zeendoc, Arena…"
        open={open.links}
        onToggle={() => toggle("links")}
      >
        <label className={`mb-4 flex items-center gap-2 text-sm ${dash.ink}`}>
          <input
            type="checkbox"
            checked={wantQuickLinks}
            onChange={(e) => {
              setWantQuickLinks(e.target.checked);
              if (!e.target.checked) setExternalLinks([]);
            }}
          />
          Ajouter des raccourcis
        </label>
        {wantQuickLinks ? (
          <div className="space-y-3">
            {externalLinks.map((link, idx) => (
              <div
                key={link.id}
                className="space-y-2 rounded-xl border border-white/70 bg-white/60 p-3"
              >
                <div className="flex justify-between">
                  <span className={`text-xs font-bold ${dash.textMid}`}>Raccourci {idx + 1}</span>
                  <button
                    type="button"
                    className="text-xs text-red-600"
                    onClick={() => setExternalLinks(externalLinks.filter((_, i) => i !== idx))}
                  >
                    Retirer
                  </button>
                </div>
                <input
                  className={onboardingInputClass}
                  placeholder="Nom"
                  value={link.name}
                  onChange={(e) => {
                    const copy = [...externalLinks];
                    copy[idx] = { ...copy[idx], name: e.target.value };
                    setExternalLinks(copy);
                  }}
                />
                <input
                  className={onboardingInputClass}
                  placeholder="URL"
                  type="url"
                  value={link.link}
                  onChange={(e) => {
                    const copy = [...externalLinks];
                    copy[idx] = { ...copy[idx], link: e.target.value };
                    setExternalLinks(copy);
                  }}
                />
                <input
                  className={onboardingInputClass}
                  placeholder="URL image (optionnel)"
                  type="url"
                  value={link.img || ""}
                  onChange={(e) => {
                    const copy = [...externalLinks];
                    copy[idx] = { ...copy[idx], img: e.target.value };
                    setExternalLinks(copy);
                  }}
                />
              </div>
            ))}
            <button
              type="button"
              className={`text-sm font-semibold ${dash.linkBold}`}
              onClick={() =>
                setExternalLinks([...externalLinks, newQuickLinkSlot(externalLinks.length)])
              }
            >
              + Ajouter un raccourci
            </button>
          </div>
        ) : null}
      </Accordion>

      <Accordion
        title="OneDrive / OCR"
        description="Dossiers élèves & agent IA"
        open={open.onedrive}
        onToggle={() => toggle("onedrive")}
      >
        <label className={`flex items-center gap-2 text-sm ${dash.ink}`}>
          <input
            type="checkbox"
            checked={integrations.microsoftOneDrive?.enabled ?? false}
            onChange={(e) =>
              setIntegrations({
                ...integrations,
                microsoftOneDrive: { enabled: e.target.checked },
              })
            }
          />
          Nous utilisons OneDrive pour les dossiers élèves
        </label>
      </Accordion>
    </div>
  );
}
