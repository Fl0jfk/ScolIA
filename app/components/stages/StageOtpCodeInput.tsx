"use client";

type Props = {
  value: string;
  onChange: (digits: string) => void;
  id?: string;
  disabled?: boolean;
  autoFocus?: boolean;
};

/** Champ OTP 6 chiffres — style net, sans placeholder italique. */
export default function StageOtpCodeInput({
  value,
  onChange,
  id = "stage-otp-code",
  disabled,
  autoFocus,
}: Props) {
  return (
    <label className="block space-y-1.5" htmlFor={id}>
      <span className="text-xs font-semibold text-stone-700">Code à 6 chiffres</span>
      <input
        id={id}
        type="text"
        inputMode="numeric"
        pattern="[0-9]*"
        autoComplete="one-time-code"
        autoFocus={autoFocus}
        disabled={disabled}
        maxLength={6}
        value={value}
        onChange={(e) => onChange(e.target.value.replace(/\D/g, "").slice(0, 6))}
        aria-label="Code à 6 chiffres reçu par e-mail"
        placeholder="------"
        className="w-full rounded-xl border border-stone-300 bg-white px-3 py-3 text-center font-mono text-2xl font-semibold tabular-nums tracking-[0.45em] text-[#1F3D2B] shadow-sm outline-none placeholder:font-mono placeholder:text-xl placeholder:font-normal placeholder:not-italic placeholder:tracking-[0.45em] placeholder:text-stone-300 focus:border-[#2F6B4A] focus:ring-2 focus:ring-[#2F6B4A]/25 disabled:bg-stone-100"
      />
      <span className="block text-[11px] text-stone-500">
        Saisissez le code reçu par e-mail (chiffres uniquement).
      </span>
    </label>
  );
}
