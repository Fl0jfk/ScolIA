import "server-only";

import { randomBytes, timingSafeEqual } from "crypto";
import {
  getJson,
  putJson,
} from "@/app/lib/s3-storage";
import { listJsonRecordsInDir, mutateJsonRecordLocked } from "@/app/lib/ent-json-postgres";
import { loadAppConfig } from "@/app/lib/app-config";
import { groupeDirection } from "@/app/lib/sorties-classe-direction";
import { getInternatStudents } from "@/app/lib/internat-storage";
import { withDossierParentContacts } from "@/app/lib/internat-famille-db";
import {
  emailValide,
  sortieIdDuJeton,
  type Resultat,
  type SaisieSortie,
  type SortieClasse,
  type SortieDirection,
  type SortieEleve,
} from "@/app/lib/sorties-classe";

/**
 * Un document par sortie. Toute modification passe par `modifierSortie` (verrou par document) :
 * deux parents qui répondent en même temps ne s'écrasent pas.
 */
const DIR = "internat/sorties-classe";

function chemin(id: string) {
  return `${DIR}/${id}.json`;
}

function nouveauJeton(sortieId: string) {
  return `${sortieId}.${randomBytes(32).toString("base64url")}`;
}

function normaliser(s: SortieClasse): SortieClasse {
  s.directions = Array.isArray(s.directions) ? s.directions : [];
  s.eleves = Array.isArray(s.eleves) ? s.eleves : [];
  for (const e of s.eleves) e.emails = Array.isArray(e.emails) ? e.emails : [];
  return s;
}

export async function lireSortie(id: string): Promise<SortieClasse | null> {
  if (!/^sc_[a-z0-9]{8,40}$/.test(id)) return null;
  const hit = await getJson<SortieClasse>(chemin(id));
  return hit?.data ? normaliser(hit.data) : null;
}

export async function listerSorties(): Promise<SortieClasse[]> {
  const rows = await listJsonRecordsInDir<SortieClasse & Record<string, unknown>>(DIR);
  return rows
    .filter((r) => r && r.date && r.createdBy)
    .map((r) => normaliser(r))
    .sort((a, b) => b.date.localeCompare(a.date) || b.createdAt.localeCompare(a.createdAt));
}

export async function modifierSortie(
  id: string,
  mutate: (sortie: SortieClasse) => Resultat,
): Promise<{ ok: true; sortie: SortieClasse } | { ok: false; error: string; status: number }> {
  if (!/^sc_[a-z0-9]{8,40}$/.test(id)) return { ok: false, error: "Sortie introuvable.", status: 404 };
  const out = await mutateJsonRecordLocked<
    SortieClasse & Record<string, unknown>,
    { res: Resultat; sortie: SortieClasse }
  >(chemin(id), (current) => {
    const sortie = normaliser(current);
    const res = mutate(sortie);
    return { save: res.ok, value: { res, sortie } };
  });
  if (!out) return { ok: false, error: "Sortie introuvable.", status: 404 };
  if (!out.value.res.ok) return { ok: false, error: out.value.res.error, status: 409 };
  return { ok: true, sortie: out.value.sortie };
}

export async function trouverParJeton(token: string): Promise<
  | { sortie: SortieClasse; role: "parent"; eleve: SortieEleve }
  | { sortie: SortieClasse; role: "direction"; direction: SortieDirection }
  | null
> {
  const id = sortieIdDuJeton(token);
  if (!id) return null;
  const sortie = await lireSortie(id);
  if (!sortie) return null;
  const egal = (other: string) => {
    const a = Buffer.from(token);
    const b = Buffer.from(other || "");
    return a.length === b.length && timingSafeEqual(a, b);
  };
  const eleve = sortie.eleves.find((e) => egal(e.token));
  if (eleve) return { sortie, role: "parent", eleve };
  const direction = sortie.directions.find((d) => egal(d.token));
  if (direction) return { sortie, role: "direction", direction };
  return null;
}

/** Liste minimale des internes pour la sélection : ni santé, ni contacts, ni chambre. */
export async function listerInternesPourSelection() {
  const students = await withDossierParentContacts(
    (await getInternatStudents()).filter((s) => s.actif),
  );
  return students
    .map((s) => ({
      id: s.id,
      nom: s.eleveRef.nom,
      prenom: s.eleveRef.prenom,
      classe: s.classe || "Sans classe",
      emailParent: Boolean(emailValide(s.parent1?.email) || emailValide(s.parent2?.email)),
    }))
    .sort(
      (a, b) =>
        a.classe.localeCompare(b.classe, "fr", { numeric: true }) ||
        a.nom.localeCompare(b.nom, "fr") ||
        a.prenom.localeCompare(b.prenom, "fr"),
    );
}

export async function creerSortie(
  saisie: SaisieSortie,
  createur: SortieClasse["createdBy"],
): Promise<{ ok: true; sortie: SortieClasse } | { ok: false; error: string }> {
  const [all, bundle] = await Promise.all([getInternatStudents(), loadAppConfig()]);
  // E-mails parents à jour (foyers du dossier élève) ; en cas d'échec de lecture, rien n'est créé.
  const students = await withDossierParentContacts(
    all.filter((s) => s.actif && saisie.studentIds.includes(s.id)),
  );
  const id = `sc_${Date.now().toString(36)}${randomBytes(6).toString("hex")}`;
  const eleves: SortieEleve[] = [];
  const directions = new Map<string, SortieDirection>();

  for (const studentId of saisie.studentIds) {
    const s = students.find((x) => x.id === studentId && x.actif);
    if (!s) {
      return { ok: false, error: "Un élève sélectionné n'est plus interne. Rechargez la page." };
    }
    const emails = [s.parent1?.email, s.parent2?.email].map(emailValide).filter(Boolean);
    const { groupe: etablissement, niveau, email } = groupeDirection(s, bundle.establishments);
    if (!directions.has(etablissement)) {
      directions.set(etablissement, {
        etablissement,
        niveau,
        token: nouveauJeton(id),
        email,
        status: "pending",
      });
    }
    eleves.push({
      studentId: s.id,
      nom: s.eleveRef.nom,
      prenom: s.eleveRef.prenom,
      classe: s.classe || "Sans classe",
      etablissement,
      token: nouveauJeton(id),
      emails: [...new Set(emails)],
      parentStatus: "pending",
    });
  }

  const now = new Date().toISOString();
  const { studentIds: _studentIds, ...infos } = saisie;
  const sortie: SortieClasse = {
    id,
    ...infos,
    createdBy: createur,
    createdAt: now,
    updatedAt: now,
    directions: [...directions.values()],
    eleves,
  };
  await putJson(chemin(id), sortie);
  return { ok: true, sortie };
}
