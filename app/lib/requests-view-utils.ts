const ROLE_SERVICE_LABELS: Record<string, string> = {
  administratif: "Administratif",
  comptabilite: "Comptabilité",
  maintenance: "Maintenance",
  education: "Vie scolaire",
  direction_ecole: "Direction — école",
  direction_college: "Direction — collège",
  direction_lycee: "Direction — lycée",
  infirmerie: "Infirmerie",
  professeur: "Enseignant",
};

function normalizeRole(value: string) {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[\s-]+/g, "_");
}

export function getViewerServiceLabel(roles: string[]): string {
  const normalized = roles.map(normalizeRole);
  for (const [role, label] of Object.entries(ROLE_SERVICE_LABELS)) {
    if (normalized.includes(role)) return label;
  }
  return "mon service";
}

function isCorbeilleStackKey(key: string): boolean {
  return key === "corbeille" || key === "tri.inconnu";
}

function stackGroupLabel(key: string, roleLabel: string, isPersonal: boolean): string {
  if (isPersonal) return "Mes demandes en cours";
  if (isCorbeilleStackKey(key)) return "Corbeille — visible par tout le personnel";
  const cleaned = roleLabel.replace(/\s*—\s*[^—]+$/, "").trim();
  return cleaned ? `À traiter — ${cleaned}` : `À traiter — ${key}`;
}
