import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import Dropdown from "./Dropdown";

export type LoaderName = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";

const LOADERS: { id: LoaderName; label: string }[] = [
  { id: "vanilla", label: "Vanilla" },
  { id: "fabric", label: "Fabric" },
  { id: "quilt", label: "Quilt" },
  { id: "forge", label: "Forge" },
  { id: "neoforge", label: "NeoForge" },
];

interface Props {
  loader: LoaderName;
  loaderVersion: string | null;
  gameVersion: string;
  onLoaderChange: (loader: LoaderName) => void;
  onLoaderVersionChange: (version: string | null) => void;
}

/** Mod loader buttons and, for Fabric or Quilt, a build picker (newest stable by default). */
export default function LoaderPicker({
  loader,
  loaderVersion,
  gameVersion,
  onLoaderChange,
  onLoaderVersionChange,
}: Props) {
  const [builds, setBuilds] = useState<string[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    if (loader === "vanilla" || !isTauri()) {
      setBuilds([]);
      return;
    }
    let cancelled = false;
    setLoadError(null);
    invoke<string[]>("list_loader_versions", { loader, gameVersion })
      .then((list) => !cancelled && setBuilds(list))
      .catch((err) => !cancelled && setLoadError(String(err)));
    return () => {
      cancelled = true;
    };
  }, [loader, gameVersion]);

  return (
    <div className="space-y-2">
      <div>
        <label className="text-xs font-semibold text-[#8E96A8]">Mod Loader</label>
        <div className="grid grid-cols-3 sm:grid-cols-5 gap-2 mt-1">
          {LOADERS.map((l) => (
            <button
              key={l.id}
              type="button"
              onClick={() => {
                onLoaderChange(l.id);
                onLoaderVersionChange(null);
              }}
              className={`py-2 px-3 rounded-xl text-xs font-bold border transition-colors cursor-pointer ${
                loader === l.id
                  ? "bg-[#E86A17]/20 text-[#E86A17] border-[#E86A17]/60 shadow-sm"
                  : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:bg-[#1A2030]"
              }`}
            >
              {l.label}
            </button>
          ))}
        </div>
      </div>

      {loader !== "vanilla" && (
        <div>
          <label className="text-xs font-semibold text-[#8E96A8]">{LOADERS.find((l) => l.id === loader)?.label} Version</label>
          <Dropdown
            ariaLabel="Loader version"
            className="mt-1"
            value={loaderVersion ?? ""}
            onChange={(v) => onLoaderVersionChange(v || null)}
            options={[
              { value: "", label: "Newest stable" },
              ...(loaderVersion && !builds.includes(loaderVersion) ? [{ value: loaderVersion, label: loaderVersion }] : []),
              ...builds.map((b) => ({ value: b, label: b })),
            ]}
          />
          {loadError && <p className="text-[11px] text-rose-400 mt-1">Could not load versions: {loadError}</p>}
        </div>
      )}
    </div>
  );
}
