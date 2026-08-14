import type { ClassLevel } from "@/app/lib/class-allocation-types";

const CLASS_LEVEL_LABELS: Record<ClassLevel, string> = {
  ecole: "École",
  college: "Collège",
  lycee: "Lycée",
};

export function classLevelLabel(level: ClassLevel | string | null | undefined): string {
  if (!level) return "—";
  return CLASS_LEVEL_LABELS[level as ClassLevel] ?? String(level);
}
