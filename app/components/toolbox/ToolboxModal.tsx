"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import type { ToolboxToolId } from "@/app/lib/toolbox-types";
import { TOOLBOX_ADMIN_LINKS, TOOLBOX_HUB_LINKS, type ToolboxAdminLinkMeta, type ToolboxHubLinkMeta } from "@/app/lib/toolbox-tools";
import { renderToolboxAdminIcon, renderToolboxHubIcon, renderToolboxIcon } from "@/app/components/toolbox/ToolboxIcons";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";

type PublicTool = {
  id: ToolboxToolId;
  label: string;
  description: string;
  adminPath: string;
  publicPath?: string;
  color: string;
  bg: string;
  season?: string;
};

type Props = {
  open: boolean;
  onClose: () => void;
};

export default function ToolboxModal({ open, onClose }: Props) {
  const router = useRouter();
  const isOrgAdmin = useIsOrgAdmin();
  const [tools, setTools] = useState<PublicTool[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    fetch("/api/toolbox/public", { cache: "no-store" })
      .then((r) => r.json())
      .then((j) => setTools(j.tools || []))
      .catch(() => setTools([]))
      .finally(() => setLoading(false));
  }, [open]);

  if (!open) return null;

  function openTool(tool: PublicTool) {
    onClose();
    if (tool.id === "qrcreator") {
      router.push("/qrcreator");
      return;
    }
    if (tool.id === "repartition-classes") {
      router.push("/toolbox/repartition-classes");
      return;
    }
    if (tool.publicPath) {
      window.open(tool.publicPath, "_blank", "noopener,noreferrer");
      return;
    }
    router.push("/toolbox");
  }

  function openAdminLink(link: ToolboxAdminLinkMeta) {
    onClose();
    router.push(link.adminPath);
  }

  function openHubLink(link: ToolboxHubLinkMeta) {
    onClose();
    router.push(link.adminPath);
  }

  const adminLinks = isOrgAdmin ? TOOLBOX_ADMIN_LINKS : [];
  const hubLinks = TOOLBOX_HUB_LINKS;
  const hasTiles = tools.length > 0 || hubLinks.length > 0 || adminLinks.length > 0;

  return (
    <div
      className="fixed inset-0 z-[200] flex items-end sm:items-center justify-center p-4 sm:p-6"
      role="dialog"
      aria-modal="true"
      aria-label="Boîte à outils"
    >
      <button
        type="button"
        className="absolute inset-0 bg-slate-900/50 backdrop-blur-sm"
        onClick={onClose}
        aria-label="Fermer"
      />
      <div className="relative w-full max-w-lg rounded-[2rem] bg-white/95 shadow-2xl border border-white/60 overflow-hidden animate-in fade-in zoom-in-95 duration-200">
        <div className="px-6 pt-6 pb-4 border-b border-slate-100 bg-gradient-to-b from-slate-50 to-white">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-[10px] font-black uppercase tracking-widest text-slate-400">Scola</p>
              <h2 className="text-xl font-black text-slate-900">Boîte à outils</h2>
              <p className="text-xs text-slate-500 mt-0.5">QR code · Photocopies couleur</p>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="rounded-full w-9 h-9 flex items-center justify-center bg-slate-100 text-slate-600 hover:bg-slate-200 font-bold"
            >
              ✕
            </button>
          </div>
        </div>

        <div className="p-6">
          {loading ? (
            <p className="text-sm text-slate-500 text-center py-8">Chargement…</p>
          ) : !hasTiles ? (
            <p className="text-sm text-slate-500 text-center py-8">
              Aucun outil activé. {isOrgAdmin ? "Configurez la boîte à outils ci-dessous." : ""}
            </p>
          ) : (
            <div className="grid grid-cols-3 gap-4 sm:gap-5">
              {tools.map((tool) => (
                <button
                  key={tool.id}
                  type="button"
                  onClick={() => openTool(tool)}
                  className="group flex flex-col items-center gap-2 rounded-2xl p-3 hover:bg-slate-50 transition-colors"
                >
                  <span
                    className={`flex h-16 w-16 items-center justify-center rounded-[1.35rem] shadow-md ring-1 ring-black/5 transition-transform group-hover:scale-105 group-active:scale-95 ${tool.bg} ${tool.color}`}
                  >
                    {renderToolboxIcon(tool.id, "w-9 h-9")}
                  </span>
                  <span className="text-[11px] font-bold text-slate-800 text-center leading-tight">
                    {tool.label}
                  </span>
                  {tool.season ? (
                    <span className="text-[9px] font-semibold text-slate-400">{tool.season}</span>
                  ) : null}
                </button>
              ))}
              {hubLinks.map((link) => (
                <button
                  key={link.id}
                  type="button"
                  onClick={() => openHubLink(link)}
                  className="group flex flex-col items-center gap-2 rounded-2xl p-3 hover:bg-slate-50 transition-colors"
                >
                  <span
                    className={`flex h-16 w-16 items-center justify-center rounded-[1.35rem] shadow-md ring-1 ring-black/5 transition-transform group-hover:scale-105 group-active:scale-95 ${link.bg} ${link.color}`}
                  >
                    {renderToolboxHubIcon(link.id, "w-9 h-9")}
                  </span>
                  <span className="text-[11px] font-bold text-slate-800 text-center leading-tight">
                    {link.label}
                  </span>
                </button>
              ))}
              {adminLinks.map((link) => (
                <button
                  key={link.id}
                  type="button"
                  onClick={() => openAdminLink(link)}
                  className="group flex flex-col items-center gap-2 rounded-2xl p-3 hover:bg-slate-50 transition-colors"
                >
                  <span
                    className={`flex h-16 w-16 items-center justify-center rounded-[1.35rem] shadow-md ring-1 ring-black/5 transition-transform group-hover:scale-105 group-active:scale-95 ${link.bg} ${link.color}`}
                  >
                    {renderToolboxAdminIcon(link.id, "w-9 h-9")}
                  </span>
                  <span className="text-[11px] font-bold text-slate-800 text-center leading-tight">
                    {link.label}
                  </span>
                </button>
              ))}
            </div>
          )}

          {isOrgAdmin && (
            <div className="mt-6 pt-4 border-t border-slate-100 flex flex-wrap gap-2 justify-center">
              <Link
                href="/toolbox"
                onClick={onClose}
                className="rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white hover:bg-slate-800"
              >
                Configurer les outils
              </Link>
              <Link
                href="/etablissement/evenements"
                onClick={onClose}
                className="rounded-xl border border-slate-200 px-4 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Événements
              </Link>
              <Link
                href="/etablissement/communication"
                onClick={onClose}
                className="rounded-xl border border-slate-200 px-4 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
              >
                Communication
              </Link>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
