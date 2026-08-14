"use client";

import type { HTMLAttributes, ReactNode } from "react";
import GlassLayer from "@/app/components/GlassLayer";

export default function ModuleCard({
  children,
  className = "",
  bodyClassName = "",
  ...rest
}: {
  children: ReactNode;
  bodyClassName?: string;
} & HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={`relative rounded-[1.5rem] border border-white/55 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] ${className}`}
      {...rest}
    >
      <GlassLayer className="bg-white/50 backdrop-blur-2xl" />
      <div className={`relative z-[1] ${bodyClassName}`}>{children}</div>
    </div>
  );
}
