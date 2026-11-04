"use client";

import { RhField, RhInput, RhTextarea } from "@/app/components/personnel/RhFormModals";
import type { PersonnelProfile } from "@/app/lib/personnel-profile";

type Props = {
  prefix?: string;
  showContract?: boolean;
  showBank?: boolean;
  profile?: PersonnelProfile;
  requireBirthDate?: boolean;
};

function n(name: string, prefix?: string) {
  return prefix ? `${prefix}${name}` : name;
}

export default function RhStaffProfileFields({
  prefix,
  showContract = true,
  showBank = true,
  profile,
  requireBirthDate = false,
}: Props) {
  const p = profile;
  return (
    <div className="space-y-6">
      <section>
        <h4 className="text-xs font-black uppercase tracking-widest text-slate-500 mb-3">État civil</h4>
        <div className="grid sm:grid-cols-2 gap-3">
          <RhField label={requireBirthDate ? "Date de naissance *" : "Date de naissance"}>
            <RhInput
              name={n("birthDate", prefix)}
              type="date"
              required={requireBirthDate}
              min="1900-01-01"
              defaultValue={p?.birthDate || ""}
            />
          </RhField>
          <RhField label="Lieu de naissance">
            <RhInput name={n("birthPlace", prefix)} placeholder="Ville, pays" defaultValue={p?.birthPlace || ""} />
          </RhField>
          <RhField label="Nom de naissance">
            <RhInput name={n("birthName", prefix)} placeholder="Si différent" defaultValue={p?.birthName || ""} />
          </RhField>
          <RhField label="Nationalité">
            <RhInput name={n("nationality", prefix)} defaultValue={p?.nationality || "Française"} />
          </RhField>
          <RhField label="Sexe">
            <select
              name={n("gender", prefix)}
              className="w-full border border-slate-200 rounded-xl px-3 py-2 text-sm"
              defaultValue={p?.gender || ""}
            >
              <option value="">—</option>
              <option value="M">Masculin</option>
              <option value="F">Féminin</option>
              <option value="autre">Autre</option>
            </select>
          </RhField>
          <RhField label="N° sécurité sociale (NIR)">
            <RhInput
              name={n("socialSecurityNumber", prefix)}
              placeholder="1 23 45 67 890 123 45"
              autoComplete="off"
              defaultValue={p?.socialSecurityNumber || ""}
            />
          </RhField>
          <RhField label="Situation familiale">
            <RhInput
              name={n("maritalStatus", prefix)}
              placeholder="Célibataire, marié(e)…"
              defaultValue={p?.maritalStatus || ""}
            />
          </RhField>
          <RhField label="Nombre d'enfants">
            <RhInput
              name={n("childrenCount", prefix)}
              type="number"
              min={0}
              step={1}
              defaultValue={p?.childrenCount ?? ""}
            />
          </RhField>
        </div>
      </section>

      <section>
        <h4 className="text-xs font-black uppercase tracking-widest text-slate-500 mb-3">Coordonnées</h4>
        <div className="grid sm:grid-cols-2 gap-3">
          <div className="sm:col-span-2">
            <RhField label="Adresse">
              <RhInput name={n("addressLine1", prefix)} placeholder="N°, rue" defaultValue={p?.addressLine1 || ""} />
            </RhField>
          </div>
          <div className="sm:col-span-2">
            <RhField label="Complément d'adresse">
              <RhInput name={n("addressLine2", prefix)} placeholder="Bâtiment, étage…" defaultValue={p?.addressLine2 || ""} />
            </RhField>
          </div>
          <RhField label="Code postal">
            <RhInput name={n("postalCode", prefix)} defaultValue={p?.postalCode || ""} />
          </RhField>
          <RhField label="Ville">
            <RhInput name={n("city", prefix)} defaultValue={p?.city || ""} />
          </RhField>
          <RhField label="Pays">
            <RhInput name={n("country", prefix)} defaultValue={p?.country || "France"} />
          </RhField>
          <RhField label="Téléphone fixe">
            <RhInput name={n("phone", prefix)} type="tel" defaultValue={p?.phone || ""} />
          </RhField>
          <RhField label="Mobile">
            <RhInput name={n("phoneMobile", prefix)} type="tel" defaultValue={p?.phoneMobile || ""} />
          </RhField>
        </div>
      </section>

      {showContract && (
        <section>
          <h4 className="text-xs font-black uppercase tracking-widest text-slate-500 mb-3">Contrat & emploi</h4>
          <div className="grid sm:grid-cols-2 gap-3">
            <RhField label="Établissement">
              <RhInput
                name={n("establishment", prefix)}
                placeholder="École, collège, lycée, siège…"
                defaultValue={p?.establishment || ""}
              />
            </RhField>
            <RhField label="Matricule interne">
              <RhInput name={n("internalId", prefix)} defaultValue={p?.internalId || ""} />
            </RhField>
            <RhField label="Type de contrat">
              <select
                name={n("contractType", prefix)}
                className="w-full border border-slate-200 rounded-xl px-3 py-2 text-sm"
                defaultValue={p?.contractType || ""}
              >
                <option value="">—</option>
                <option value="cdi">CDI</option>
                <option value="cdd">CDD</option>
                <option value="cddu">CDDU</option>
                <option value="interim">Intérim</option>
                <option value="stage">Stage</option>
                <option value="autre">Autre</option>
              </select>
            </RhField>
            <RhField label="Date fin de contrat">
              <RhInput name={n("contractEndDate", prefix)} type="date" defaultValue={p?.contractEndDate || ""} />
            </RhField>
            <RhField label="Temps de travail (%)">
              <RhInput
                name={n("workTimePercent", prefix)}
                type="number"
                min={0}
                max={100}
                defaultValue={p?.workTimePercent ?? 100}
              />
            </RhField>
            <RhField label="Classification">
              <RhInput name={n("classification", prefix)} placeholder="Ex. Employé" defaultValue={p?.classification || ""} />
            </RhField>
            <RhField label="Coefficient">
              <RhInput name={n("coefficient", prefix)} defaultValue={p?.coefficient || ""} />
            </RhField>
            <RhField label="Salaire brut mensuel">
              <RhInput
                name={n("grossMonthlySalary", prefix)}
                placeholder="Ex. 2 100,00"
                defaultValue={p?.grossMonthlySalary || ""}
              />
            </RhField>
          </div>
        </section>
      )}

      {showBank && (
        <section>
          <h4 className="text-xs font-black uppercase tracking-widest text-slate-500 mb-3">Banque & urgence</h4>
          <div className="grid sm:grid-cols-2 gap-3">
            <RhField label="IBAN">
              <RhInput name={n("iban", prefix)} autoComplete="off" defaultValue={p?.iban || ""} />
            </RhField>
            <RhField label="BIC">
              <RhInput name={n("bic", prefix)} autoComplete="off" defaultValue={p?.bic || ""} />
            </RhField>
            <RhField label="Contact urgence — nom">
              <RhInput name={n("emergencyContactName", prefix)} defaultValue={p?.emergencyContactName || ""} />
            </RhField>
            <RhField label="Contact urgence — téléphone">
              <RhInput
                name={n("emergencyContactPhone", prefix)}
                type="tel"
                defaultValue={p?.emergencyContactPhone || ""}
              />
            </RhField>
            <div className="sm:col-span-2">
              <RhField label="Notes RH">
                <RhTextarea name={n("notes", prefix)} placeholder="Informations complémentaires" defaultValue={p?.notes || ""} />
              </RhField>
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
