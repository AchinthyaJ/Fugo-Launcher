import { useState, useEffect } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  Play,
  Square,
  Settings,
  FolderOpen,
  Copy,
  Trash2,
  Puzzle,
  Globe,
  Image as ImageIcon,
  Check,
  Search,
  ShieldCheck,
  Cpu,
  Package,
  Sparkles
} from "lucide-react";
import type { Instance } from "../model";
import LoaderPicker, { LoaderName } from "./LoaderPicker";

const KNOWN_LOADERS: LoaderName[] = ["vanilla", "fabric", "quilt", "forge", "neoforge"];
/** The loader a Java instance starts from in the settings; Horizon Rust is not a Java loader. */
const javaLoaderOf = (loader: string): LoaderName =>
  (KNOWN_LOADERS as string[]).includes(loader) ? (loader as LoaderName) : "vanilla";

interface InstalledMod {
  filename: string;
  name: string;
  enabled: boolean;
  size_bytes: number;
}

interface ContentItem {
  name: string;
  filename: string;
  size_bytes: number;
  enabled?: boolean;
  description?: string | null;
}

interface InstanceDashboardProps {
  instance: Instance;
  isRunning: boolean;
  onBack: () => void;
  onLaunch: (inst: Instance) => void;
  onKill: (id: string) => void;
  onUpdate: (updated: Instance) => void;
  onDelete: (id: string) => void;
  onDuplicate: (id: string) => void;
  onOpenFolder: (id: string) => void;
}

const PRESET_COVERS = [
  { name: "Misty Mountains", file: "mountains_misty.png" },
  { name: "Sunset Peaks", file: "mountains_sunset.png" },
  { name: "Night Sky Aurora", file: "night_sky_aurora.png" },
  { name: "Dark Obsidian", file: "dark_obsidian_mesh.png" },
  { name: "Default Cover 1", file: "instance_default_cover_1.png" },
  { name: "Default Cover 2", file: "instance_default_cover_2.png" },
  { name: "Default Cover 3", file: "instance_default_cover_3.png" },
  { name: "Default Cover 4", file: "instance_default_cover_4.png" },
];

function getCoverSrc(banner: string | null | undefined): string {
  if (!banner) return "/assets/instance_default_cover_1.png";
  if (banner.startsWith("http://") || banner.startsWith("https://") || banner.startsWith("/")) return banner;
  return `/assets/${banner}`;
}

export default function InstanceDashboard({
  instance,
  isRunning,
  onBack,
  onLaunch,
  onKill,
  onUpdate,
  onDelete,
  onDuplicate,
  onOpenFolder
}: InstanceDashboardProps) {
  const [activeTab, setActiveTab] = useState<"content" | "worlds" | "screenshots" | "settings">("content");

  // Mods & content state
  const [mods, setMods] = useState<InstalledMod[]>([]);
  const [resourcePacks, setResourcePacks] = useState<ContentItem[]>([]);
  const [shaderPacks, setShaderPacks] = useState<ContentItem[]>([]);
  const [worlds, setWorlds] = useState<ContentItem[]>([]);
  const [screenshots, setScreenshots] = useState<ContentItem[]>([]);
  const [modSearchFilter, setModSearchFilter] = useState("");
  const [packSearchFilter, setPackSearchFilter] = useState("");
  const [scanResult, setScanResult] = useState<string | null>(null);

  // Editable settings state
  const [editName, setEditName] = useState(instance.name);
  const [editEdition, setEditEdition] = useState<"rust" | "java">(instance.edition);
  const [editLoader, setEditLoader] = useState<LoaderName>(javaLoaderOf(instance.loader));
  const [editLoaderVersion, setEditLoaderVersion] = useState<string | null>(instance.loader_version ?? null);
  const [editRam, setEditRam] = useState(instance.memory_max_mb);
  const [editJavaArgs, setEditJavaArgs] = useState(instance.java_args || "");
  const [editBanner, setEditBanner] = useState(instance.banner || "instance_default_cover_1.png");
  const [isSavedNotice, setIsSavedNotice] = useState(false);

  const isRust = instance.edition === "rust" || instance.loader === "horizon-rust";
  const loaderBadge = isRust ? "HORIZON RUST" : (instance.loader ? instance.loader.toUpperCase() : "VANILLA");

  const loadData = () => {
    if (!isTauri()) return;

    if (!isRust) {
      invoke<InstalledMod[]>("list_instance_mods", { instanceId: instance.id })
        .then(setMods)
        .catch(() => setMods([]));
    }

    invoke<ContentItem[]>("list_instance_content", { instanceId: instance.id, subfolder: "resourcepacks" })
      .then(setResourcePacks)
      .catch(() => setResourcePacks([]));

    invoke<ContentItem[]>("list_instance_content", { instanceId: instance.id, subfolder: "shaderpacks" })
      .then(setShaderPacks)
      .catch(() => setShaderPacks([]));

    invoke<ContentItem[]>("list_instance_content", { instanceId: instance.id, subfolder: "saves" })
      .then(setWorlds)
      .catch(() => setWorlds([]));

    invoke<ContentItem[]>("list_instance_content", { instanceId: instance.id, subfolder: "screenshots" })
      .then(setScreenshots)
      .catch(() => setScreenshots([]));
  };

  useEffect(() => {
    loadData();
    setEditName(instance.name);
    setEditEdition(instance.edition);
    setEditLoader(javaLoaderOf(instance.loader));
    setEditLoaderVersion(instance.loader_version ?? null);
    setEditRam(instance.memory_max_mb);
    setEditJavaArgs(instance.java_args || "");
    setEditBanner(instance.banner || "instance_default_cover_1.png");
  }, [instance.id]);

  const handleToggleMod = async (mod: InstalledMod) => {
    if (!isTauri()) return;
    try {
      await invoke("toggle_instance_mod", {
        instanceId: instance.id,
        filename: mod.filename,
        enabled: !mod.enabled
      });
      loadData();
    } catch (err: any) {
      alert(`Toggle failed: ${err}`);
    }
  };

  const handleDeleteMod = async (mod: InstalledMod) => {
    if (!confirm(`Delete mod ${mod.name}?`)) return;
    if (!isTauri()) return;
    try {
      await invoke("delete_instance_mod", {
        instanceId: instance.id,
        filename: mod.filename
      });
      loadData();
    } catch (err: any) {
      alert(`Delete failed: ${err}`);
    }
  };

  const handleToggleResourcePack = async (pack: ContentItem) => {
    if (!isTauri()) return;
    try {
      await invoke("toggle_instance_content", {
        instanceId: instance.id,
        subfolder: "resourcepacks",
        filename: pack.filename,
        enabled: !pack.enabled
      });
      loadData();
    } catch (err: any) {
      alert(`Toggle failed: ${err}`);
    }
  };

  const handleDeleteResourcePack = async (pack: ContentItem) => {
    if (!confirm(`Delete resource pack ${pack.name}?`)) return;
    if (!isTauri()) return;
    try {
      await invoke("delete_instance_content", {
        instanceId: instance.id,
        subfolder: "resourcepacks",
        filename: pack.filename
      });
      loadData();
    } catch (err: any) {
      alert(`Delete failed: ${err}`);
    }
  };

  const handleSaveSettings = async () => {
    const updated: Instance = {
      ...instance,
      name: editName.trim() || instance.name,
      edition: editEdition,
      loader: editEdition === "rust" ? "horizon-rust" : editLoader,
      loader_version: editEdition === "java" && editLoader !== "vanilla" ? editLoaderVersion : null,
      game_version: editEdition === "rust" ? "26.3" : instance.game_version,
      memory_max_mb: editRam,
      java_args: editJavaArgs,
      banner: editBanner
    };

    if (isTauri()) {
      try {
        await invoke("update_instance", { instance: updated });
        onUpdate(updated);
        setIsSavedNotice(true);
        setTimeout(() => setIsSavedNotice(false), 2500);
      } catch (err: any) {
        alert(`Failed to save settings: ${err}`);
      }
    } else {
      onUpdate(updated);
      setIsSavedNotice(true);
      setTimeout(() => setIsSavedNotice(false), 2500);
    }
  };

  const filteredMods = mods.filter(
    (m) =>
      m.name.toLowerCase().includes(modSearchFilter.toLowerCase()) ||
      m.filename.toLowerCase().includes(modSearchFilter.toLowerCase())
  );

  const filteredPacks = resourcePacks.filter(
    (p) =>
      p.name.toLowerCase().includes(packSearchFilter.toLowerCase()) ||
      p.filename.toLowerCase().includes(packSearchFilter.toLowerCase()) ||
      (p.description && p.description.toLowerCase().includes(packSearchFilter.toLowerCase()))
  );

  return (
    <div className="space-y-6 animate-in fade-in duration-200">
      {/* Top Breadcrumb & Back button */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={onBack}
            className="fugo-secondary-btn flex items-center gap-2 cursor-pointer hover:bg-white/10 active:scale-95 transition-all"
          >
            <ArrowLeft className="w-3.5 h-3.5 text-[#E86A17]" />
            <span className="font-semibold text-xs">← Back to Instances</span>
          </button>
          <span className="text-white/20">/</span>
          <span className="text-xs font-semibold text-[#8E96A8] truncate max-w-sm">{instance.name}</span>
        </div>

        <button
          type="button"
          onClick={() => onOpenFolder(instance.id)}
          className="fugo-secondary-btn flex items-center gap-1.5 text-xs text-[#8E96A8] hover:text-white"
          title="Open Instance Directory"
        >
          <FolderOpen className="w-3.5 h-3.5 text-[#E86A17]" />
          <span>Open Folder</span>
        </button>
      </div>

      {/* Hero Header Glass Card (72px Cover + Name + Badges + Play/Stop + Stats) */}
      <div className="relative rounded-[22px] bg-[#0E121E]/80 border border-white/[0.08] p-6 shadow-2xl backdrop-blur-xl overflow-hidden">
        {/* Subtle background glow from cover */}
        <div
          className="absolute -right-20 -top-20 w-80 h-80 opacity-20 filter blur-3xl pointer-events-none rounded-full"
          style={{ background: isRust ? "radial-gradient(circle, #10B981, transparent)" : "radial-gradient(circle, #E86A17, transparent)" }}
        />

        {/* Top Row: Icon + Name/Badges + Play Button */}
        <div className="flex items-center justify-between gap-4 relative z-10">
          <div className="flex items-center gap-4">
            <div className="w-[72px] h-[72px] rounded-[16px] overflow-hidden bg-black/40 border border-white/10 shadow-lg flex-shrink-0 relative group">
              <img
                src={getCoverSrc(instance.banner)}
                alt=""
                className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                onError={(e) => {
                  (e.target as HTMLImageElement).src = "/assets/instance_default_cover_1.png";
                }}
              />
            </div>

            <div className="space-y-1.5">
              <div className="flex items-center gap-2">
                <span
                  className={`text-[10px] font-bold px-2 py-0.5 rounded-full border tracking-wide uppercase ${
                    isRust
                      ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/40"
                      : "bg-[#E86A17]/20 text-[#FFA048] border-[#E86A17]/40"
                  }`}
                >
                  {loaderBadge}
                </span>
                <span className="text-[10px] font-bold text-white/70 bg-white/[0.06] border border-white/[0.08] px-2 py-0.5 rounded-full">
                  {instance.game_version}
                </span>
                {isRust && (
                  <span className="text-[10px] font-bold text-purple-300 bg-purple-500/20 border border-purple-500/40 px-2 py-0.5 rounded-full">
                    WGPU NATIVE
                  </span>
                )}
              </div>

              <h1 className="text-2xl font-black text-white tracking-tight">{instance.name}</h1>
            </div>
          </div>

          {/* Primary Action Button: Play / Stop */}
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => {
                if (isRunning) {
                  onKill(instance.id);
                } else {
                  onLaunch(instance);
                }
              }}
              className={`h-11 px-7 rounded-[14px] text-white font-bold text-sm flex items-center gap-2.5 transition-all shadow-lg cursor-pointer active:scale-95 ${
                isRunning
                  ? "bg-red-500/80 hover:bg-red-500 text-white shadow-red-500/30"
                  : "bg-gradient-to-r from-[#E86A17] to-[#F97316] hover:brightness-110 shadow-[#E86A17]/30 hover:shadow-[#E86A17]/50"
              }`}
            >
              {isRunning ? (
                <>
                  <Square className="w-4 h-4 fill-white" />
                  <span>Stop Instance</span>
                </>
              ) : (
                <>
                  <Play className="w-4 h-4 fill-white" />
                  <span>Play Now</span>
                </>
              )}
            </button>
          </div>
        </div>

        {/* Divider */}
        <div className="h-[1px] bg-white/[0.06] my-4" />

        {/* Bottom Row: 4-Cell Stats Strip + Action shortcuts */}
        <div className="flex flex-wrap items-center justify-between gap-4 text-xs relative z-10">
          <div className="flex items-center gap-6">
            <div>
              <span className="text-[10px] font-bold text-[#8E96A8] uppercase tracking-wider block">STATUS</span>
              <div className="flex items-center gap-1.5 mt-0.5">
                <span className={`w-2 h-2 rounded-full ${isRunning ? "bg-emerald-400 animate-pulse shadow-sm shadow-emerald-400" : "bg-emerald-400"}`} />
                <span className={`font-semibold ${isRunning ? "text-emerald-300" : "text-white/80"}`}>
                  {isRunning ? "Running in background" : "Ready to play"}
                </span>
              </div>
            </div>

            <div className="h-6 w-[1px] bg-white/10" />

            <div>
              <span className="text-[10px] font-bold text-[#8E96A8] uppercase tracking-wider block">LAST PLAYED</span>
              <span className="font-semibold text-white/80 mt-0.5 block">
                {instance.last_played || "Never"}
              </span>
            </div>

            <div className="h-6 w-[1px] bg-white/10" />

            <div>
              <span className="text-[10px] font-bold text-[#8E96A8] uppercase tracking-wider block">MAX MEMORY</span>
              <span className="font-semibold text-[#FFA048] mt-0.5 block">
                {(instance.memory_max_mb / 1024).toFixed(1)} GB
              </span>
            </div>
          </div>

          {/* Quick Action Buttons */}
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setActiveTab("settings")}
              className="h-8 px-3 rounded-[10px] bg-white/[0.06] hover:bg-white/[0.12] border border-white/[0.08] text-white/90 text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer"
              title="Settings"
            >
              <Settings className="w-3.5 h-3.5 text-purple-400" />
              <span>Settings</span>
            </button>

            <button
              type="button"
              onClick={() => onDuplicate(instance.id)}
              className="h-8 px-3 rounded-[10px] bg-white/[0.06] hover:bg-white/[0.12] border border-white/[0.08] text-white/90 text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer"
              title="Duplicate Instance"
            >
              <Copy className="w-3.5 h-3.5 text-blue-400" />
              <span>Duplicate</span>
            </button>

            <button
              type="button"
              onClick={() => onDelete(instance.id)}
              className="h-8 px-3 rounded-[10px] bg-red-500/10 hover:bg-red-500/20 border border-red-500/20 text-red-300 text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer"
              title="Delete Instance"
            >
              <Trash2 className="w-3.5 h-3.5 text-red-400" />
              <span>Delete</span>
            </button>
          </div>
        </div>
      </div>

      {/* Navigation Sub-Tabs Bar */}
      <div className="flex items-center gap-1.5 border-b border-white/[0.08] pb-2">
        <button
          type="button"
          onClick={() => setActiveTab("content")}
          className={`h-9 px-4 rounded-[10px] text-xs font-bold flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === "content"
              ? "bg-[#E86A17] text-white shadow-md shadow-[#E86A17]/25"
              : "text-[#8E96A8] hover:text-white hover:bg-white/[0.05]"
          }`}
        >
          <Puzzle className="w-3.5 h-3.5" />
          <span>Content & Resource Packs</span>
          <span className="text-[10px] bg-white/10 px-1.5 py-0.2 rounded-full">
            {mods.length + resourcePacks.length}
          </span>
        </button>

        <button
          type="button"
          onClick={() => setActiveTab("worlds")}
          className={`h-9 px-4 rounded-[10px] text-xs font-bold flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === "worlds"
              ? "bg-[#E86A17] text-white shadow-md shadow-[#E86A17]/25"
              : "text-[#8E96A8] hover:text-white hover:bg-white/[0.05]"
          }`}
        >
          <Globe className="w-3.5 h-3.5" />
          <span>Worlds & Saves</span>
          <span className="text-[10px] bg-white/10 px-1.5 py-0.2 rounded-full">{worlds.length}</span>
        </button>

        <button
          type="button"
          onClick={() => setActiveTab("screenshots")}
          className={`h-9 px-4 rounded-[10px] text-xs font-bold flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === "screenshots"
              ? "bg-[#E86A17] text-white shadow-md shadow-[#E86A17]/25"
              : "text-[#8E96A8] hover:text-white hover:bg-white/[0.05]"
          }`}
        >
          <ImageIcon className="w-3.5 h-3.5" />
          <span>Screenshots</span>
          <span className="text-[10px] bg-white/10 px-1.5 py-0.2 rounded-full">{screenshots.length}</span>
        </button>

        <button
          type="button"
          onClick={() => setActiveTab("settings")}
          className={`h-9 px-4 rounded-[10px] text-xs font-bold flex items-center gap-2 transition-all cursor-pointer ${
            activeTab === "settings"
              ? "bg-[#E86A17] text-white shadow-md shadow-[#E86A17]/25"
              : "text-[#8E96A8] hover:text-white hover:bg-white/[0.05]"
          }`}
        >
          <Settings className="w-3.5 h-3.5" />
          <span>Settings & Config</span>
        </button>
      </div>

      {/* =========================================================================
          TAB 1: CONTENT (MODS & UNDER THAT: RESOURCE PACKS)
          ========================================================================= */}
      {activeTab === "content" && (
        <div className="space-y-6">
          {/* SECTION 1: INSTALLED MODS */}
          <div className="space-y-3">
            {isRust ? (
              /* Rust Edition Informational Notice */
              <div className="p-7 rounded-[20px] bg-[#0E121E]/80 border border-emerald-500/20 text-center space-y-3 shadow-xl backdrop-blur-md">
                <div className="w-12 h-12 rounded-2xl bg-emerald-500/20 border border-emerald-500/30 flex items-center justify-center mx-auto text-emerald-400">
                  <Cpu className="w-6 h-6" />
                </div>
                <h2 className="text-base font-bold text-white">Horizon Rust Edition Mods</h2>
                <p className="text-xs text-[#8E96A8] max-w-lg mx-auto leading-relaxed">
                  Mods support for the Rust Edition is currently in development and not yet enabled for gameplay. Horizon runs on a native high-performance Rust engine (wgpu) with integrated chunk meshing, Vulkan rendering, and zero Java GC stutter.
                </p>
                <div className="flex items-center justify-center gap-3 pt-1">
                  <span className="text-[11px] font-semibold text-emerald-300 bg-emerald-500/10 border border-emerald-500/20 px-3 py-1 rounded-full">
                    ✓ Native 26.3 Client
                  </span>
                  <span className="text-[11px] font-semibold text-purple-300 bg-purple-500/10 border border-purple-500/20 px-3 py-1 rounded-full">
                    ✓ Resource Packs Ready
                  </span>
                  <span className="text-[11px] font-semibold text-blue-300 bg-blue-500/10 border border-blue-500/20 px-3 py-1 rounded-full">
                    ✓ Shaderpacks Ready
                  </span>
                </div>
              </div>
            ) : (
              /* Java Edition Full Mods Section */
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <h3 className="text-base font-bold text-white flex items-center gap-2">
                      <Puzzle className="w-4 h-4 text-[#E86A17]" />
                      <span>Installed Mods</span>
                    </h3>
                    <span className="text-[11px] font-bold text-[#8E96A8] bg-white/[0.06] border border-white/[0.08] px-2.5 py-0.5 rounded-full">
                      {mods.length} {mods.length === 1 ? "mod" : "mods"}
                    </span>
                  </div>

                  <div className="flex items-center gap-2">
                    <div className="relative">
                      <Search className="w-3.5 h-3.5 text-[#8E96A8] absolute left-2.5 top-2.5" />
                      <input
                        type="text"
                        placeholder="Filter installed mods..."
                        value={modSearchFilter}
                        onChange={(e) => setModSearchFilter(e.target.value)}
                        className="h-8 pl-8 pr-3 rounded-lg bg-[#141824] border border-[#23293D] text-xs text-white placeholder-[#8E96A8] focus:border-[#E86A17] focus:outline-none w-48"
                      />
                    </div>

                    <button
                      type="button"
                      onClick={() => {
                        const duplicates = mods.filter(
                          (m, i, arr) => arr.findIndex((x) => x.name.toLowerCase() === m.name.toLowerCase()) !== i
                        );
                        if (duplicates.length > 0) {
                          setScanResult(`Found ${duplicates.length} potential duplicate/conflicting mod files.`);
                        } else {
                          setScanResult("✓ No mod conflicts or duplicate files detected.");
                        }
                        setTimeout(() => setScanResult(null), 4000);
                      }}
                      className="h-8 px-3 rounded-lg bg-white/[0.06] hover:bg-white/10 text-xs font-semibold text-white/90 border border-white/10 transition-colors flex items-center gap-1.5 cursor-pointer"
                    >
                      <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
                      <span>Scan Conflicts</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => onOpenFolder(instance.id)}
                      className="h-8 px-3 rounded-lg bg-white/[0.06] hover:bg-white/10 text-xs font-semibold text-white/90 border border-white/10 transition-colors flex items-center gap-1.5 cursor-pointer"
                    >
                      <FolderOpen className="w-3.5 h-3.5 text-[#8E96A8]" />
                      <span>Mods Folder</span>
                    </button>
                  </div>
                </div>

                {scanResult && (
                  <div className="p-3 rounded-xl bg-purple-500/10 border border-purple-500/20 text-xs text-purple-200">
                    {scanResult}
                  </div>
                )}

                {filteredMods.length === 0 ? (
                  <div className="p-7 rounded-[16px] bg-[#0E121E]/60 border border-white/[0.08] text-center space-y-2">
                    <Puzzle className="w-7 h-7 text-[#8E96A8]/50 mx-auto" />
                    <h4 className="text-xs font-bold text-white/80">No Mods Found</h4>
                    <p className="text-[11px] text-[#8E96A8] max-w-sm mx-auto">
                      Drop .jar mods into the instance mods folder or install them from Modrinth.
                    </p>
                  </div>
                ) : (
                  <div className="grid grid-cols-2 gap-2.5 max-h-[340px] overflow-y-auto pr-1">
                    {filteredMods.map((mod) => (
                      <div
                        key={mod.filename}
                        className={`p-3 rounded-[14px] border transition-all flex items-center justify-between gap-3 ${
                          mod.enabled
                            ? "bg-[#0E121E]/80 border-white/[0.08]"
                            : "bg-[#0E121E]/40 border-white/[0.04] opacity-50"
                        }`}
                      >
                        <div className="min-w-0 flex-1">
                          <h4 className="text-xs font-bold text-white truncate">{mod.name}</h4>
                          <p className="text-[10px] text-[#8E96A8] truncate mt-0.5 font-mono">
                            {mod.filename} · {(mod.size_bytes / (1024 * 1024)).toFixed(1)} MB
                          </p>
                        </div>

                        <div className="flex items-center gap-2">
                          <button
                            type="button"
                            onClick={() => handleToggleMod(mod)}
                            className={`w-8 h-4 rounded-full transition-colors relative cursor-pointer ${
                              mod.enabled ? "bg-[#E86A17]" : "bg-white/20"
                            }`}
                            title={mod.enabled ? "Disable mod" : "Enable mod"}
                          >
                            <span
                              className={`block w-3 h-3 rounded-full bg-white transition-transform ${
                                mod.enabled ? "translate-x-4" : "translate-x-0.5"
                              }`}
                            />
                          </button>

                          <button
                            type="button"
                            onClick={() => handleDeleteMod(mod)}
                            className="text-white/40 hover:text-red-400 p-1 transition-colors cursor-pointer"
                            title="Delete Mod"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* SECTION 2: RESOURCE PACKS (DIRECTLY UNDER MODS) */}
          <div className="pt-2 space-y-3 border-t border-white/[0.08]">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <Package className="w-4 h-4 text-[#FFA048]" />
                  <span>Resource Packs</span>
                </h3>
                <span className="text-[11px] font-bold text-[#8E96A8] bg-white/[0.06] border border-white/[0.08] px-2.5 py-0.5 rounded-full">
                  {resourcePacks.length} {resourcePacks.length === 1 ? "pack" : "packs"}
                </span>
                <span className="text-[11px] text-[#8E96A8]">
                  for textures, models, and sound overhauls
                </span>
              </div>

              <div className="flex items-center gap-2">
                <div className="relative">
                  <Search className="w-3.5 h-3.5 text-[#8E96A8] absolute left-2.5 top-2.5" />
                  <input
                    type="text"
                    placeholder="Search resource packs..."
                    value={packSearchFilter}
                    onChange={(e) => setPackSearchFilter(e.target.value)}
                    className="h-8 pl-8 pr-3 rounded-lg bg-[#141824] border border-[#23293D] text-xs text-white placeholder-[#8E96A8] focus:border-[#E86A17] focus:outline-none w-48"
                  />
                </div>

                <button
                  type="button"
                  onClick={() => onOpenFolder(instance.id)}
                  className="h-8 px-3 rounded-lg bg-white/[0.06] hover:bg-white/10 text-xs font-semibold text-white/90 border border-white/10 transition-colors flex items-center gap-1.5 cursor-pointer"
                >
                  <FolderOpen className="w-3.5 h-3.5 text-[#FFA048]" />
                  <span>Resource Packs Folder</span>
                </button>
              </div>
            </div>

            {filteredPacks.length === 0 ? (
              <div className="p-8 rounded-[16px] bg-[#0E121E]/60 border border-white/[0.08] text-center space-y-2">
                <Package className="w-8 h-8 text-[#8E96A8]/50 mx-auto" />
                <h4 className="text-xs font-bold text-white/80">No Resource Packs Installed</h4>
                <p className="text-[11px] text-[#8E96A8] max-w-sm mx-auto">
                  Drop custom texture pack .zip files or folders into the resourcepacks directory to manage them here.
                </p>
                <button
                  type="button"
                  onClick={() => onOpenFolder(instance.id)}
                  className="text-xs text-[#E86A17] font-semibold hover:underline cursor-pointer pt-1"
                >
                  Open resourcepacks folder
                </button>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2.5 max-h-[340px] overflow-y-auto pr-1">
                {filteredPacks.map((pack) => {
                  const isEnabled = pack.enabled !== false;
                  return (
                    <div
                      key={pack.filename}
                      className={`p-3 rounded-[14px] border transition-all flex items-center justify-between gap-3 ${
                        isEnabled
                          ? "bg-[#0E121E]/80 border-white/[0.08]"
                          : "bg-[#0E121E]/40 border-white/[0.04] opacity-50"
                      }`}
                    >
                      <div className="flex items-center gap-3 min-w-0 flex-1">
                        <div className="w-9 h-9 rounded-lg bg-[#141824] border border-white/10 flex items-center justify-center shrink-0">
                          <Package className="w-4 h-4 text-[#FFA048]" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <h4 className="text-xs font-bold text-white truncate">{pack.name}</h4>
                            <span className={`text-[9px] font-bold px-1.5 py-0.2 rounded border ${
                              isEnabled
                                ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/40"
                                : "bg-white/5 text-[#8E96A8] border-white/10"
                            }`}>
                              {isEnabled ? "ACTIVE" : "DISABLED"}
                            </span>
                          </div>
                          {pack.description && (
                            <p className="text-[10px] text-[#8E96A8] truncate mt-0.5">
                              {pack.description}
                            </p>
                          )}
                          <p className="text-[9.5px] text-[#6C7A9C] truncate font-mono mt-0.5">
                            {pack.filename} · {(pack.size_bytes / (1024 * 1024)).toFixed(1)} MB
                          </p>
                        </div>
                      </div>

                      <div className="flex items-center gap-2">
                        <button
                          type="button"
                          onClick={() => handleToggleResourcePack(pack)}
                          className={`w-8 h-4 rounded-full transition-colors relative cursor-pointer ${
                            isEnabled ? "bg-[#FFA048]" : "bg-white/20"
                          }`}
                          title={isEnabled ? "Disable pack" : "Enable pack"}
                        >
                          <span
                            className={`block w-3 h-3 rounded-full bg-white transition-transform ${
                              isEnabled ? "translate-x-4" : "translate-x-0.5"
                            }`}
                          />
                        </button>

                        <button
                          type="button"
                          onClick={() => handleDeleteResourcePack(pack)}
                          className="text-white/40 hover:text-red-400 p-1 transition-colors cursor-pointer"
                          title="Delete Resource Pack"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* SECTION 3: SHADER PACKS */}
          <div className="pt-2 space-y-3 border-t border-white/[0.08]">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <h3 className="text-sm font-bold text-white flex items-center gap-2">
                  <Sparkles className="w-3.5 h-3.5 text-purple-400" />
                  <span>Shaderpacks</span>
                </h3>
                <span className="text-[11px] font-bold text-[#8E96A8] bg-white/[0.06] border border-white/[0.08] px-2 py-0.5 rounded-full">
                  {shaderPacks.length}
                </span>
              </div>

              <button
                type="button"
                onClick={() => onOpenFolder(instance.id)}
                className="fugo-secondary-btn text-xs text-[#8E96A8] hover:text-white"
              >
                <FolderOpen className="w-3 h-3 text-purple-400" />
                <span>Open Shaders Folder</span>
              </button>
            </div>

            {shaderPacks.length === 0 ? (
              <div className="p-4 rounded-[14px] bg-[#0E121E]/40 border border-white/[0.06] text-center text-xs text-[#8E96A8]">
                No custom shaderpacks detected. Drop shaderpack .zip files into the shaderpacks folder.
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2 max-h-[160px] overflow-y-auto pr-1">
                {shaderPacks.map((shader) => (
                  <div key={shader.filename} className="p-2.5 rounded-[12px] bg-[#0E121E]/80 border border-white/[0.08] flex items-center justify-between">
                    <span className="text-xs font-semibold text-white truncate">{shader.name}</span>
                    <span className="text-[10px] text-[#8E96A8] font-mono">
                      {(shader.size_bytes / (1024 * 1024)).toFixed(1)} MB
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* =========================================================================
          TAB 2: WORLDS & SAVES
          ========================================================================= */}
      {activeTab === "worlds" && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold text-white">Singleplayer Saves & Worlds</h3>
            <button
              type="button"
              onClick={() => onOpenFolder(instance.id)}
              className="text-xs text-[#E86A17] font-semibold hover:underline cursor-pointer"
            >
              Open saves folder
            </button>
          </div>

          {worlds.length === 0 ? (
            <div className="p-8 rounded-[16px] bg-[#0E121E]/60 border border-white/[0.08] text-center space-y-2">
              <Globe className="w-8 h-8 text-[#8E96A8]/50 mx-auto" />
              <h3 className="text-sm font-bold text-white/80">No Worlds Created Yet</h3>
              <p className="text-xs text-[#8E96A8] max-w-sm mx-auto">
                Launch the instance and create a singleplayer world to see it listed here.
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-3 gap-3">
              {worlds.map((world) => (
                <div
                  key={world.filename}
                  className="p-4 rounded-[16px] bg-[#0E121E]/80 border border-white/[0.08] hover:border-white/20 transition-all space-y-2"
                >
                  <div className="flex items-center gap-2">
                    <Globe className="w-4 h-4 text-emerald-400" />
                    <h4 className="text-xs font-bold text-white truncate">{world.name}</h4>
                  </div>
                  <p className="text-[10px] text-[#8E96A8] font-mono truncate">{instance.game_dir}/saves/{world.filename}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* =========================================================================
          TAB 3: SCREENSHOTS
          ========================================================================= */}
      {activeTab === "screenshots" && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold text-white">In-Game Screenshots</h3>
            <button
              type="button"
              onClick={() => onOpenFolder(instance.id)}
              className="text-xs text-[#E86A17] font-semibold hover:underline cursor-pointer"
            >
              Open screenshots folder
            </button>
          </div>

          {screenshots.length === 0 ? (
            <div className="p-8 rounded-[16px] bg-[#0E121E]/60 border border-white/[0.08] text-center space-y-2">
              <ImageIcon className="w-8 h-8 text-[#8E96A8]/50 mx-auto" />
              <h3 className="text-sm font-bold text-white/80">No Screenshots Taken</h3>
              <p className="text-xs text-[#8E96A8] max-w-sm mx-auto">
                Press F2 while playing to take in-game screenshots.
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-4 gap-3 max-h-[450px] overflow-y-auto pr-1">
              {screenshots.map((shot) => (
                <div
                  key={shot.filename}
                  className="rounded-[14px] overflow-hidden bg-black/40 border border-white/10 hover:border-[#E86A17] transition-all p-2 space-y-1.5"
                >
                  <div className="h-24 rounded-lg bg-black/60 flex items-center justify-center overflow-hidden">
                    <ImageIcon className="w-6 h-6 text-white/20" />
                  </div>
                  <p className="text-[10px] text-white/80 font-mono truncate px-1">{shot.name}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* =========================================================================
          TAB 4: SETTINGS & CONFIG
          ========================================================================= */}
      {activeTab === "settings" && (
        <div className="p-6 rounded-[20px] bg-[#0E121E]/80 border border-white/[0.08] space-y-5 max-w-2xl">
          {isSavedNotice && (
            <div className="p-3 rounded-xl bg-emerald-500/20 border border-emerald-500/30 text-xs font-semibold text-emerald-300 flex items-center gap-2">
              <Check className="w-4 h-4 text-emerald-400" />
              <span>Instance settings saved successfully!</span>
            </div>
          )}

          <div>
            <label className="text-xs font-semibold text-[#8E96A8] block mb-1">Instance Name</label>
            <input
              type="text"
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              className="w-full px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] text-sm text-white focus:border-[#E86A17] focus:outline-none"
            />
          </div>

          <div>
            <label className="text-xs font-semibold text-[#8E96A8] block mb-1">Edition</label>
            <div className="grid grid-cols-2 gap-2">
              <button
                type="button"
                onClick={() => setEditEdition("rust")}
                className={`py-2.5 px-3 rounded-xl text-xs font-bold border transition-colors flex items-center justify-center gap-2 cursor-pointer ${
                  editEdition === "rust"
                    ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/60 shadow-sm"
                    : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                }`}
              >
                <span>🦀 Horizon (Rust)</span>
              </button>
              <button
                type="button"
                onClick={() => setEditEdition("java")}
                className={`py-2.5 px-3 rounded-xl text-xs font-bold border transition-colors flex items-center justify-center gap-2 cursor-pointer ${
                  editEdition === "java"
                    ? "bg-[#E86A17]/20 text-[#FFA048] border-[#E86A17]/60 shadow-sm"
                    : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                }`}
              >
                <span>☕ Java Edition</span>
              </button>
            </div>
          </div>

          {editEdition === "java" && (
            <LoaderPicker
              loader={editLoader}
              loaderVersion={editLoaderVersion}
              gameVersion={instance.game_version}
              onLoaderChange={setEditLoader}
              onLoaderVersionChange={setEditLoaderVersion}
            />
          )}

          <div>
            <div className="flex items-center justify-between text-xs mb-1">
              <span className="font-semibold text-[#8E96A8]">Allocated Memory (RAM)</span>
              <span className="text-[#FFA048] font-bold font-mono">
                {(editRam / 1024).toFixed(1)} GB ({editRam} MB)
              </span>
            </div>
            <input
              type="range"
              min={1024}
              max={16384}
              step={512}
              value={editRam}
              onChange={(e) => setEditRam(Number(e.target.value))}
              className="w-full accent-[#E86A17] cursor-pointer"
            />
          </div>

          <div>
            <label className="text-xs font-semibold text-[#8E96A8] block mb-1">Java / JVM Arguments</label>
            <input
              type="text"
              value={editJavaArgs}
              onChange={(e) => setEditJavaArgs(e.target.value)}
              placeholder="-Xmx4G -XX:+UseG1GC"
              className="w-full px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] font-mono text-xs text-white focus:border-[#E86A17] focus:outline-none"
            />
          </div>

          <div>
            <label className="text-xs font-semibold text-[#8E96A8] block mb-1.5">Cover Art</label>
            <div className="grid grid-cols-4 gap-2">
              {PRESET_COVERS.map((cov) => {
                const isSelected = editBanner === cov.file;
                return (
                  <div
                    key={cov.file}
                    onClick={() => setEditBanner(cov.file)}
                    className={`relative h-14 rounded-lg overflow-hidden cursor-pointer border transition-all ${
                      isSelected ? "border-[#E86A17] ring-2 ring-[#E86A17]/40 scale-105" : "border-white/10 opacity-70 hover:opacity-100"
                    }`}
                    title={cov.name}
                  >
                    <img src={`/assets/${cov.file}`} alt={cov.name} className="w-full h-full object-cover" />
                    <span className="absolute bottom-0 inset-x-0 bg-black/70 text-[8px] text-center text-white truncate px-1 py-0.5">
                      {cov.name}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="pt-2">
            <button
              type="button"
              onClick={handleSaveSettings}
              className="px-6 py-2.5 rounded-xl bg-[#E86A17] hover:bg-[#F97316] text-white text-xs font-bold transition-all shadow-md shadow-[#E86A17]/30 cursor-pointer active:scale-95"
            >
              Save Changes
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
