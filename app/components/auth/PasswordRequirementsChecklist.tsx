"use client";

import {
  getPasswordRequirements,
  PASSWORD_POLICY_EXAMPLE,
} from "@/app/lib/password-policy";

type Props = {
  password: string;
  /** Variante visuelle (page forcée vs modale). */
  tone?: "amber" | "slate";
};

/** Checklist live des règles MDP (symbole bien mis en avant). */
export default function PasswordRequirementsChecklist({
  password,
  tone = "slate",
}: Props) {
  const requirements = getPasswordRequirements(password);
  const muted = tone === "amber" ? "text-amber-900/70" : "text-slate-500";
  const okClass = tone === "amber" ? "text-emerald-800" : "text-emerald-700";
  const koClass = tone === "amber" ? "text-amber-900/55" : "text-slate-400";

  return (
    <div className={`rounded-xl border px-3 py-2.5 text-xs ${tone === "amber" ? "border-amber-100 bg-amber-50/70" : "border-slate-100 bg-slate-50"}`}>
      <p className={`font-semibold ${muted}`}>Le mot de passe doit contenir :</p>
      <ul className="mt-1.5 space-y-1">
        {requirements.map((req) => (
          <li
            key={req.id}
            className={`flex items-start gap-2 ${req.ok ? okClass : koClass}`}
          >
            <span aria-hidden className="mt-0.5 w-3.5 shrink-0 font-bold">
              {req.ok ? "✓" : "○"}
            </span>
            <span>
              {req.label}
              {req.id === "symbol" && !req.ok ? (
                <span className="block font-medium text-amber-800">
                  Obligatoire — un point, un point d’exclamation, etc.
                </span>
              ) : null}
            </span>
          </li>
        ))}
      </ul>
      <p className={`mt-2 ${muted}`}>
        {PASSWORD_POLICY_EXAMPLE}
      </p>
    </div>
  );
}
