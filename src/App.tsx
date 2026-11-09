import React, { useState, useEffect } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Puzzle,
  Minus,
  Square,
  X,
  Check,
  Plus,
  FolderOpen,
  Search,
  Download,
  Zap,
  Cpu,
  RefreshCw,
  HardDrive,
  Play,
  Settings as SettingsIcon,
  Terminal,
  MoreVertical,
  Pencil,
  Trash2,
  Image as ImageIcon,
  FolderDown,
  Layers
} from "lucide-react";
import ArtworkBackground from "./components/ArtworkBackground";
import CustomizeTab from "./components/CustomizeTab";
import { Prefs, applyCursorPack, backgroundSrc, loadPrefs, savePrefs, urlToDataUrl } from "./customize";
import FloatingDock from "./components/FloatingDock";
import HomeHero from "./components/HomeHero";
import LoaderPicker, { LoaderName } from "./components/LoaderPicker";
import Dropdown from "./components/Dropdown";
import InstanceDashboard from "./components/InstanceDashboard";
import type { Account, Instance } from "./model";
import AccountSwitcher from "./components/AccountSwitcher";


interface LauncherSettings {
  default_ram_mb: number;
  discrete_gpu: boolean;
  horizon_resources_path: string;
  horizon_binary_path: string;
  custom_java_path: string | null;
  close_on_launch: boolean;
  theme: string;
}

interface ModHit {
  project_id: string;
  title: string;
  description: string;
  categories: string[];
  client_side: string;
  server_side: string;
  downloads: number;
  icon_url: string | null;
  author: string;
  slug: string;
}

interface InstalledMod {
  filename: string;
  name: string;
  enabled: boolean;
  size_bytes: number;
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
  if (!banner) return "/assets/backgrounds/1.png";
  if (banner.startsWith("http://") || banner.startsWith("https://") || banner.startsWith("/")) return banner;
  return `/assets/${banner}`;
}

export default function App() {
  const [activeTab, setActiveTab] = useState<"home" | "instances" | "mods" | "customize" | "settings" | "servers">("home");
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
  useEffect(() => {
    savePrefs(prefs);
    applyCursorPack(prefs.cursorPack);
  }, [prefs]);

  const [instances, setInstances] = useState<Instance[]>([
    {
      id: "fabulously-optimized",
      name: "Fabulously Optimized",
      edition: "rust",
      game_version: "26.3",
      loader: "horizon-rust",
      game_dir: "~/.local/share/fugo-launcher/instances/fabulously-optimized",
      banner: "backgrounds/1.png",
      icon: "minecraft_icon.png",
      memory_min_mb: 2048,
      memory_max_mb: 4096,
      java_args: "-XX:+UseG1GC",
      created_at: "2026-11-08",
      last_played: "Just now"
    },
    {
      id: "vanilla-minecraft",
      name: "Vanilla Minecraft",
      edition: "java",
      game_version: "1.21.1",
      loader: "vanilla",
      game_dir: "~/.local/share/fugo-launcher/instances/vanilla-minecraft",
      banner: "backgrounds/1.png",
      icon: "minecraft_icon.png",
      memory_min_mb: 2048,
      memory_max_mb: 4096,
      java_args: "-Xmx4G -XX:+UseG1GC",
      created_at: "2026-11-08",
      last_played: null
    }
  ]);

  const [selectedInstanceId, setSelectedInstanceId] = useState<string>("fabulously-optimized");
  const [runningInstances, setRunningInstances] = useState<string[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([
    {
      id: "user-deathdragon",
      username: "DeathDragon13",
      uuid: "offline-uuid-deathdragon",
      is_offline: true,
      skin_preset: "Steve",
      cape_preset: "2016",
      is_active: true
    }
  ]);

  const [settings, setSettings] = useState<LauncherSettings>({
    default_ram_mb: 4096,
    discrete_gpu: true,
    horizon_resources_path: ".local/horizon-client/26.3",
    horizon_binary_path: "target/fast/horizon-client",
    custom_java_path: null,
    close_on_launch: false,
    theme: "obsidian"
  });

  // Modrinth search state
  const [modQuery, setModQuery] = useState("");
  const [modType, setModType] = useState<"mod" | "modpack" | "resourcepack" | "shader">("mod");
  const [modSort, setModSort] = useState<"follows" | "relevance" | "downloads" | "newest" | "updated">("follows");
  const [modResults, setModResults] = useState<ModHit[]>([]);
  const [modLoading, setModLoading] = useState(false);
  const [installedMods, setInstalledMods] = useState<InstalledMod[]>([]);
  const [installingModId, setInstallingModId] = useState<string | null>(null);

  // Server copy state
  const [copiedServer, setCopiedServer] = useState<string | null>(null);

  // New instance modal state
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newInstanceName, setNewInstanceName] = useState("");
  const [newInstanceEdition, setNewInstanceEdition] = useState<"rust" | "java">("rust");
  const [newInstanceLoader, setNewInstanceLoader] = useState<LoaderName>("vanilla");
  const [newInstanceLoaderVersion, setNewInstanceLoaderVersion] = useState<string | null>(null);
  const [newInstanceVersion, setNewInstanceVersion] = useState("26.3");
  const [newInstanceCover, setNewInstanceCover] = useState("mountains_misty.png");
  const [newInstanceRam, setNewInstanceRam] = useState(4096);

  // Import modals state
  const [showImportDirModal, setShowImportDirModal] = useState(false);
  const [importDirPath, setImportDirPath] = useState("");
  const [showImportZipModal, setShowImportZipModal] = useState(false);
  const [importZipPath, setImportZipPath] = useState("");
  const [isImporting, setIsImporting] = useState(false);
  const [importEdition, setImportEdition] = useState<"auto" | "rust" | "java">("auto");

  // Instance management & modal state
  const [dashboardInstance, setDashboardInstance] = useState<Instance | null>(null);
  const [editingInstance, setEditingInstance] = useState<Instance | null>(null);
  const [deletingInstanceId, setDeletingInstanceId] = useState<string | null>(null);
  const [contextMenuInstanceId, setContextMenuInstanceId] = useState<string | null>(null);
  const [showLogsInstance, setShowLogsInstance] = useState<Instance | null>(null);

  // Horizon compiler status
  const [buildStatus, setBuildStatus] = useState<string | null>(null);

  // Install/download progress for the selected launch, shown under Home.
  const [download, setDownload] = useState<{ stage: string; done: number; total: number } | null>(null);
  const [isBuildingHorizon, setIsBuildingHorizon] = useState(false);

  const selectedInstance = instances.find((i) => i.id === selectedInstanceId) || instances[0];
  const currentDashboardInstance = dashboardInstance
    ? instances.find((i) => i.id === dashboardInstance.id) || dashboardInstance
    : null;
  const activeAccount = accounts.find((a) => a.is_active) || accounts[0];
  const isSelectedRunning = runningInstances.includes(selectedInstance?.id || "");

  useEffect(() => {
    if (isTauri()) {
      invoke<Instance[]>("get_instances").then((res) => {
        if (res && res.length > 0) setInstances(res);
      }).catch(console.error);

      invoke<Account[]>("get_accounts").then((res) => {
        if (res && res.length > 0) setAccounts(res);
      }).catch(console.error);

      invoke<LauncherSettings>("get_settings").then(setSettings).catch(console.error);
      invoke<string[]>("get_running_instances").then(setRunningInstances).catch(console.error);

      const unlistenExit = listen<string>("instance-stopped", (event) => {
        setRunningInstances((prev) => prev.filter((id) => id !== event.payload));
        setDownload(null);
      });

      const unlistenDownload = listen<{ stage: string; done: number; total: number }>("install-progress", (event) => {
        setDownload(event.payload.stage === "Ready" ? null : event.payload);
      });

      const unlistenBuild = listen<string>("build-status", (event) => {
        setBuildStatus(event.payload);
        if (event.payload === "BUILD_COMPLETE" || event.payload === "BUILD_FAILED") {
          setIsBuildingHorizon(false);
        }
      });

      return () => {
        unlistenExit.then((f) => f());
        unlistenDownload.then((f) => f());
        unlistenBuild.then((f) => f());
      };
    }
  }, []);

  useEffect(() => {
    if (isTauri() && selectedInstance) {
      invoke<InstalledMod[]>("list_instance_mods", { instanceId: selectedInstance.id })
        .then(setInstalledMods)
        .catch(() => setInstalledMods([]));
    }
  }, [selectedInstanceId]);

  const handleLaunch = async () => {
    if (!selectedInstance) return;
    if (isSelectedRunning) {
      if (isTauri()) {
        try {
          await invoke("kill_instance", { instanceId: selectedInstance.id });
          setRunningInstances((prev) => prev.filter((id) => id !== selectedInstance.id));
        } catch (e: any) {
          alert(e);
        }
      } else {
        setRunningInstances([]);
      }
    } else {
      if (isTauri()) {
        try {
          await invoke("launch_instance", { instanceId: selectedInstance.id });
          setRunningInstances((prev) => [...prev, selectedInstance.id]);
        } catch (e: any) {
          alert(`Launch failed: ${e}`);
        }
      } else {
        setRunningInstances([selectedInstance.id]);
      }
    }
  };

  const handleCopyIp = (ip: string, id: string) => {
    navigator.clipboard.writeText(ip);
    setCopiedServer(id);
    setTimeout(() => setCopiedServer(null), 2000);
  };

  const handleModSearch = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    setModLoading(true);
    if (isTauri()) {
      try {
        const res: any = await invoke("search_modrinth", {
          query: modQuery,
          loader: selectedInstance?.loader || undefined,
          gameVersion: selectedInstance?.game_version || undefined,
          limit: 24,
          projectType: modType,
          sort: modSort
        });
        setModResults(res.hits || []);
      } catch (err: any) {
        alert(err);
      } finally {
        setModLoading(false);
      }
    } else {
      setTimeout(() => {
        setModResults([
          {
            project_id: "sodium",
            title: "Sodium",
            description: "Modern rendering engine and client-side optimization mod for Minecraft.",
            categories: ["optimization"],
            client_side: "required",
            server_side: "unsupported",
            downloads: 48920102,
            icon_url: "https://cdn.modrinth.com/data/AANobbMI/icon.png",
            author: "jellysquid3",
            slug: "sodium"
          },
          {
            project_id: "iris",
            title: "Iris Shaders",
            description: "A modern shaders mod for Minecraft compatible with existing shaders.",
            categories: ["optimization", "shaders"],
            client_side: "required",
            server_side: "unsupported",
            downloads: 35129032,
            icon_url: "https://cdn.modrinth.com/data/YL57xq9U/icon.png",
            author: "coderbot",
            slug: "iris"
          }
        ]);
        setModLoading(false);
      }, 500);
    }
  };

  // The recommended list loads on open, and again when the type, sort or instance changes.
  useEffect(() => {
    if (!isTauri()) return;
    handleModSearch();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [modType, modSort, selectedInstance?.id]);

  const handleInstallMod = async (mod: ModHit) => {
    if (modType !== "modpack" && !selectedInstance) return;
    setInstallingModId(mod.project_id);
    if (isTauri()) {
      try {
        if (modType === "modpack") {
          const created: Instance = await invoke("install_modrinth_modpack", {
            projectId: mod.project_id,
            versionId: null
          });
          setInstances((prev) => [...prev, created]);
          setSelectedInstanceId(created.id);
        } else {
          await invoke<InstalledMod>("download_modrinth_mod", {
            instanceId: selectedInstance!.id,
            projectId: mod.project_id,
            versionId: null
          });
          if (modType === "mod") {
            const mods = await invoke<InstalledMod[]>("list_instance_mods", { instanceId: selectedInstance!.id });
            setInstalledMods(mods);
          }
        }
      } catch (err: any) {
        alert(`Failed to install ${mod.title}: ${err}`);
      } finally {
        setInstallingModId(null);
      }
    } else {
      setTimeout(() => {
        setInstalledMods((prev) => [
          ...prev,
          { filename: `${mod.slug}.jar`, name: mod.title, enabled: true, size_bytes: 1250000 }
        ]);
        setInstallingModId(null);
      }, 700);
    }
  };

  const handleImportDir = async () => {
    if (!isTauri()) {
      setShowImportDirModal(true);
      return;
    }
    try {
      setIsImporting(true);
      const picked: string | null = await invoke("pick_directory");
      if (picked) {
        const edition = importEdition === "auto" ? null : importEdition;
        const imported: Instance = await invoke("import_instance_directory", { sourcePath: picked, edition });
        setInstances((prev) => {
          const filtered = prev.filter((i) => i.id !== imported.id);
          return [...filtered, imported];
        });
        setSelectedInstanceId(imported.id);
        alert(`Successfully imported instance "${imported.name}" (${imported.edition === "rust" ? "Horizon Rust" : "Java"}) into ~/.minecraft/death-client/instances!`);
      } else {
        setShowImportDirModal(true);
      }
    } catch (err: any) {
      alert(`Import directory failed: ${err?.message || err}`);
      setShowImportDirModal(true);
    } finally {
      setIsImporting(false);
    }
  };

  const handleManualImportDir = async () => {
    if (!importDirPath.trim()) return;
    try {
      setIsImporting(true);
      if (isTauri()) {
        const edition = importEdition === "auto" ? null : importEdition;
        const imported: Instance = await invoke("import_instance_directory", { sourcePath: importDirPath.trim(), edition });
        setInstances((prev) => {
          const filtered = prev.filter((i) => i.id !== imported.id);
          return [...filtered, imported];
        });
        setSelectedInstanceId(imported.id);
        setShowImportDirModal(false);
        setImportDirPath("");
        alert(`Successfully imported instance "${imported.name}" (${imported.edition === "rust" ? "Horizon Rust" : "Java"}) into ~/.minecraft/death-client/instances!`);
      }
    } catch (err: any) {
      alert(`Import directory failed: ${err?.message || err}`);
    } finally {
      setIsImporting(false);
    }
  };

  const handleImportZip = async () => {
    if (!isTauri()) {
      setShowImportZipModal(true);
      return;
    }
    try {
      setIsImporting(true);
      const picked: string | null = await invoke("pick_zip_file");
      if (picked) {
        const edition = importEdition === "auto" ? null : importEdition;
        const imported: Instance = await invoke("import_instance_zip", { zipPath: picked, edition });
        setInstances((prev) => {
          const filtered = prev.filter((i) => i.id !== imported.id);
          return [...filtered, imported];
        });
        setSelectedInstanceId(imported.id);
        alert(`Successfully imported instance "${imported.name}" (${imported.edition === "rust" ? "Horizon Rust" : "Java"}) from zip!`);
      } else {
        setShowImportZipModal(true);
      }
    } catch (err: any) {
      alert(`Import zip failed: ${err?.message || err}`);
      setShowImportZipModal(true);
    } finally {
      setIsImporting(false);
    }
  };

  const handleManualImportZip = async () => {
    if (!importZipPath.trim()) return;
    try {
      setIsImporting(true);
      if (isTauri()) {
        const edition = importEdition === "auto" ? null : importEdition;
        const imported: Instance = await invoke("import_instance_zip", { zipPath: importZipPath.trim(), edition });
        setInstances((prev) => {
          const filtered = prev.filter((i) => i.id !== imported.id);
          return [...filtered, imported];
        });
        setSelectedInstanceId(imported.id);
        setShowImportZipModal(false);
        setImportZipPath("");
        alert(`Successfully imported instance "${imported.name}" (${imported.edition === "rust" ? "Horizon Rust" : "Java"})!`);
      }
    } catch (err: any) {
      alert(`Import zip failed: ${err?.message || err}`);
    } finally {
      setIsImporting(false);
    }
  };

  const handleCreateInstance = async () => {
    if (!newInstanceName.trim()) return;
    const req = {
      name: newInstanceName.trim(),
      edition: newInstanceEdition,
      game_version: newInstanceVersion,
      loader: newInstanceEdition === "rust" ? "horizon-rust" : newInstanceLoader,
      banner: newInstanceCover
    };

    if (isTauri()) {
      try {
        let created: Instance = await invoke("create_instance", { req });
        if (newInstanceLoaderVersion && created.loader !== "vanilla") {
          created = { ...created, loader_version: newInstanceLoaderVersion };
          await invoke("update_instance", { instance: created }).catch(console.error);
        }
        if (newInstanceRam !== 4096) {
          const updated = { ...created, memory_max_mb: newInstanceRam };
          await invoke("update_instance", { instance: updated }).catch(console.error);
          setInstances((prev) => [...prev, updated]);
        } else {
          setInstances((prev) => [...prev, created]);
        }
        setSelectedInstanceId(created.id);
        setShowCreateModal(false);
        setNewInstanceName("");
      } catch (err: any) {
        alert(err);
      }
    } else {
      const mock: Instance = {
        id: `inst-${Date.now()}`,
        name: newInstanceName.trim(),
        edition: newInstanceEdition,
        game_version: newInstanceVersion,
        loader: newInstanceEdition === "rust" ? "horizon-rust" : newInstanceLoader,
        game_dir: `instances/${newInstanceName.trim().toLowerCase().replace(/\s+/g, "-")}`,
        banner: newInstanceCover,
        icon: "minecraft_icon.png",
        memory_min_mb: 2048,
        memory_max_mb: newInstanceRam,
        java_args: "-Xmx4G -XX:+UseG1GC",
        created_at: "2026-11-09",
        last_played: null
      };
      setInstances((prev) => [...prev, mock]);
      setSelectedInstanceId(mock.id);
      setShowCreateModal(false);
      setNewInstanceName("");
    }
  };

  const handleSaveEdit = async () => {
    if (!editingInstance) return;
    if (isTauri()) {
      try {
        await invoke("update_instance", { instance: editingInstance });
        setInstances((prev) => prev.map((i) => (i.id === editingInstance.id ? editingInstance : i)));
        setEditingInstance(null);
      } catch (err: any) {
        alert(`Failed to save: ${err}`);
      }
    } else {
      setInstances((prev) => prev.map((i) => (i.id === editingInstance.id ? editingInstance : i)));
      setEditingInstance(null);
    }
  };

  const handleDeleteInstance = async (id: string) => {
    if (isTauri()) {
      try {
        await invoke("delete_instance", { id });
        setInstances((prev) => prev.filter((i) => i.id !== id));
        if (selectedInstanceId === id) {
          const remaining = instances.filter((i) => i.id !== id);
          if (remaining.length > 0) setSelectedInstanceId(remaining[0].id);
        }
      } catch (e: any) {
        alert(`Delete failed: ${e}`);
      }
    } else {
      setInstances((prev) => prev.filter((i) => i.id !== id));
      if (selectedInstanceId === id) {
        const remaining = instances.filter((i) => i.id !== id);
        if (remaining.length > 0) setSelectedInstanceId(remaining[0].id);
      }
    }
    setDeletingInstanceId(null);
    setContextMenuInstanceId(null);
  };

  const handleLaunchInstance = async (inst: Instance) => {
    setSelectedInstanceId(inst.id);
    const isRunning = runningInstances.includes(inst.id);
    if (isRunning) {
      if (isTauri()) {
        try {
          await invoke("kill_instance", { instanceId: inst.id });
          setRunningInstances((prev) => prev.filter((id) => id !== inst.id));
        } catch (e: any) {
          alert(e);
        }
      } else {
        setRunningInstances((prev) => prev.filter((id) => id !== inst.id));
      }
    } else {
      if (isTauri()) {
        try {
          await invoke("launch_instance", { instanceId: inst.id });
          setRunningInstances((prev) => [...prev, inst.id]);
        } catch (e: any) {
          alert(`Launch failed: ${e}`);
        }
      } else {
        setRunningInstances((prev) => [...prev, inst.id]);
      }
    }
  };

  const handleBuildHorizon = async () => {
    setIsBuildingHorizon(true);
    setBuildStatus("Initializing cargo compiler worker...");
    if (isTauri()) {
      try {
        await invoke("build_horizon");
      } catch (e: any) {
        alert(e);
        setIsBuildingHorizon(false);
      }
    } else {
      setTimeout(() => {
        setBuildStatus("BUILD_COMPLETE");
        setIsBuildingHorizon(false);
      }, 2000);
    }
  };

  const switchAccount = async (id: string) => {
    setAccounts((prev) => prev.map((a) => ({ ...a, is_active: a.id === id })));
    setShowAccounts(false);
    if (isTauri()) {
      try {
        await invoke("set_active_account", { id });
      } catch (e: any) {
        alert(e);
      }
    }
  };

  const addAccount = async (username: string) => {
    if (isTauri()) {
      try {
        const created = await invoke<Account>("create_offline_account", { username, skin: null, cape: null });
        setAccounts((prev) => [...prev.map((a) => ({ ...a, is_active: false })), created]);
      } catch (e: any) {
        alert(e);
      }
    } else {
      setAccounts((prev) => [
        ...prev.map((a) => ({ ...a, is_active: false })),
        { id: `user-${Date.now()}`, username, uuid: `offline-${username}`, is_offline: true, skin_preset: null, cape_preset: null, is_active: true },
      ]);
    }
  };

  const handleMinimize = () => isTauri() && invoke("window_minimize");
  const handleMaximize = () => isTauri() && invoke("window_maximize");
  const handleClose = () => isTauri() && invoke("window_close");

  const capes = ["2011", "2012", "2013", "2015", "2016", "crafter", "migrator"];
  const skins = ["Steve", "Alex", "Efe", "Noor", "Sunny", "Ari", "Kai", "Makena"];

  const [showAccounts, setShowAccounts] = useState(false);

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-[#07080D] text-[#F3F4F6] select-none">
      {/* Fixed original atmospheric Fugo misty mountain background */}
      <ArtworkBackground src={backgroundSrc(prefs)} />

      {/* Floating chrome: brand, section dock, profile and window controls */}
      <header
        data-tauri-drag-region
        className="absolute inset-x-0 top-0 z-30 grid grid-cols-[1fr_auto_1fr] items-center gap-4 px-5 pt-4"
      >
        <div data-tauri-drag-region className="flex items-center gap-2.5 justify-self-start">
          <img src="/assets/fugo-logo-transparent.png" alt="Fugo" className="w-7 h-7 object-contain" />
          <span className="text-[12px] font-extrabold tracking-[0.2em] text-white/90">FUGO</span>
        </div>

        <FloatingDock
          active={activeTab}
          onSelect={(tab) => {
            if (tab === "instances" && activeTab === "instances" && dashboardInstance) {
              setDashboardInstance(null);
            } else {
              setActiveTab(tab);
            }
          }}
        />

        <div className="flex items-center gap-2 justify-self-end">
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowAccounts((v) => !v)}
              aria-haspopup="menu"
              aria-expanded={showAccounts}
              className="glass flex items-center gap-2 h-9 pl-1 pr-3.5 rounded-full"
            >
              <span className="w-7 h-7 rounded-full overflow-hidden bg-[#22293F]">
                <img src="/resources/Steve_classic.png" alt="" className="w-full h-full object-cover object-top scale-125" />
              </span>
              <span className="text-[12px] font-semibold text-white max-w-[110px] truncate">{activeAccount.username}</span>
            </button>
            {showAccounts && (
              <div className="absolute right-0 top-11">
                <AccountSwitcher accounts={accounts} onSwitch={switchAccount} onAdd={addAccount} />
              </div>
            )}
          </div>

          <div
            className="glass flex items-center gap-0.5 h-9 px-1 rounded-full"
          >
            <button type="button" onClick={handleMinimize} className="win-btn" aria-label="Minimize">
              <Minus className="w-3.5 h-3.5" />
            </button>
            <button type="button" onClick={handleMaximize} className="win-btn" aria-label="Maximize">
              <Square className="w-3 h-3" />
            </button>
            <button type="button" onClick={handleClose} className="win-btn is-danger" aria-label="Close">
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </header>

      {showAccounts && <div className="fixed inset-0 z-20" onClick={() => setShowAccounts(false)} />}

      <main className="absolute inset-0 z-10 flex flex-col overflow-hidden">
        {/* =========================================================================
            HOME: greeting, edition-aware play button, instance selector only
            ========================================================================= */}
        {activeTab === "home" && (
          <div className="relative flex-1 min-h-0 flex flex-col items-center justify-end px-8 pt-28 pb-[30vh]">
            <HomeHero
              username={activeAccount.username}
              instances={instances}
              selected={selectedInstance}
              running={isSelectedRunning}
              download={download}
              onSelect={setSelectedInstanceId}
              onLaunch={handleLaunch}
              onNewInstance={() => setShowCreateModal(true)}
              onOpenDashboard={(inst) => {
                setSelectedInstanceId(inst.id);
                setDashboardInstance(inst);
                setActiveTab("instances");
              }}
            />
          </div>
        )}

        {/* =========================================================================
            TAB 2: INSTANCES PAGE (Fugo Launcher Profiles Deck & Dashboard)
            ========================================================================= */}
        {activeTab === "instances" && (
          <div className="px-8 pb-12 pt-24 max-w-6xl mx-auto space-y-6 overflow-y-auto h-full">
            {currentDashboardInstance ? (
              <InstanceDashboard
                instance={currentDashboardInstance}
                isRunning={runningInstances.includes(currentDashboardInstance.id)}
                onBack={() => setDashboardInstance(null)}
                onLaunch={(inst) => handleLaunchInstance(inst)}
                onKill={(id) => {
                  const target = instances.find((i) => i.id === id) || currentDashboardInstance;
                  handleLaunchInstance(target);
                }}
                onUpdate={(updated) => {
                  setInstances((prev) => prev.map((i) => (i.id === updated.id ? updated : i)));
                  setDashboardInstance(updated);
                  if (selectedInstanceId === updated.id) {
                    setSelectedInstanceId(updated.id);
                  }
                }}
                onDelete={(id) => {
                  handleDeleteInstance(id);
                  setDashboardInstance(null);
                }}
                onDuplicate={async (id) => {
                  if (isTauri()) {
                    try {
                      const newInst = await invoke<Instance>("duplicate_instance", { instanceId: id });
                      setInstances((prev) => [...prev, newInst]);
                      setDashboardInstance(newInst);
                    } catch (err: any) {
                      alert(`Duplicate failed: ${err}`);
                    }
                  }
                }}
                onOpenFolder={(id) => {
                  if (isTauri()) invoke("open_instance_folder", { id });
                }}
              />
            ) : (
              <>
                {/* Header: Instances Title & Compact Action Buttons */}
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-4">
              <div>
                <h1 className="text-[20px] font-bold text-white tracking-tight flex items-center gap-3">
                  <span>Instances</span>
                  <span className="text-[11px] font-bold text-[#8E96A8] bg-white/[0.06] border border-white/[0.08] px-2.5 py-0.5 rounded-full">
                    {instances.length}
                  </span>
                </h1>
                <p className="text-[12px] text-[#8E96A8] mt-0.5">
                  Manage, customize, and launch isolated Minecraft installations
                </p>
              </div>

              <div className="flex items-center gap-2.5">
                <button
                  type="button"
                  onClick={handleImportDir}
                  disabled={isImporting}
                  className="fugo-secondary-btn cursor-pointer"
                  title="Import Minecraft Instance Directory"
                >
                  <FolderOpen className="w-3.5 h-3.5 text-[#E86A17]" />
                  <span>{isImporting ? "Importing..." : "Import Dir"}</span>
                </button>

                <button
                  type="button"
                  onClick={handleImportZip}
                  disabled={isImporting}
                  className="fugo-secondary-btn cursor-pointer"
                  title="Import Instance from Zip Backup or Modpack"
                >
                  <FolderDown className="w-3.5 h-3.5 text-[#E86A17]" />
                  <span>Import Zip</span>
                </button>

                <button
                  type="button"
                  onClick={() => setShowCreateModal(true)}
                  className="h-8 px-3.5 rounded-[10px] bg-[#E86A17] hover:bg-[#F97316] text-white text-[12px] font-bold flex items-center gap-1.5 transition-all shadow-md shadow-[#E86A17]/25 cursor-pointer active:scale-95"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>New Instance</span>
                </button>
              </div>
            </div>

            {/* Instances Cards Grid (290px x 160px with hover overlays) */}
            <div className="flex flex-wrap gap-4 items-start">
              {/* "+ Add New Instance" Placeholder Card */}
              <div
                onClick={() => setShowCreateModal(true)}
                className="fugo-add-card"
                title="Create New Instance"
              >
                <div className="w-9 h-9 rounded-full border border-[#E86A17]/70 flex items-center justify-center text-[#E86A17]">
                  <Plus className="w-5 h-5" />
                </div>
                <span className="text-[13px] font-semibold text-[#E86A17]">
                  Add New Instance
                </span>
              </div>

              {/* Normal Instance Cards */}
              {instances.map((inst) => {
                const isSelected = inst.id === selectedInstanceId;
                const isRunning = runningInstances.includes(inst.id);
                const isFugo = inst.edition === "rust" || inst.name.toLowerCase().includes("horizon") || inst.name.toLowerCase().includes("fugo");
                const loaderBadge = inst.edition === "rust"
                  ? "HORIZON RUST"
                  : inst.loader
                    ? [inst.loader.toUpperCase(), inst.loader_version].filter(Boolean).join(" ")
                    : "VANILLA";
                const coverUrl = getCoverSrc(inst.banner);

                return (
                  <div
                    key={inst.id}
                    onClick={() => setSelectedInstanceId(inst.id)}
                    onDoubleClick={() => {
                      setSelectedInstanceId(inst.id);
                      setDashboardInstance(inst);
                    }}
                    title={`${inst.name} (Double-click to open dashboard)`}
                    className={`fugo-inst-card group ${isSelected ? "is-selected" : ""}`}
                  >
                    {/* Background Cover Image with Vignette */}
                    <img
                      src={coverUrl}
                      alt={inst.name}
                      className="absolute inset-0 w-full h-full object-cover object-center filter brightness-[0.85] transition-transform duration-300 group-hover:scale-105"
                      onError={(e) => {
                        (e.target as HTMLImageElement).src = "/assets/instance_default_cover_1.png";
                      }}
                    />
                    <div className="absolute inset-0 bg-gradient-to-t from-[#090C14] via-[#090C14]/40 to-transparent" />

                    {/* Normal Content (Dims out smoothly when hovered) */}
                    <div className="relative z-10 w-full h-full p-3.5 flex flex-col justify-between transition-opacity duration-200 group-hover:opacity-15">
                      {/* Top Row: Icon Container, Loader Badge, Menu Button */}
                      <div className="flex items-center justify-between">
                        {/* Icon */}
                        <div className="w-9 h-9 rounded-lg bg-[#141824]/90 border border-white/10 p-1.5 flex items-center justify-center shadow">
                          <img
                            src={isFugo ? "/assets/fugo-logo-transparent.png" : "/assets/minecraft_icon.png"}
                            alt=""
                            className="w-full h-full object-contain"
                          />
                        </div>

                        {/* Loader Badge & Context Menu */}
                        <div className="flex items-center gap-1.5">
                          <span className={`text-[9px] font-bold tracking-wider px-2 py-0.5 rounded border uppercase ${
                            inst.edition === "rust"
                              ? "bg-emerald-500/20 text-emerald-400 border-emerald-500/40"
                              : "bg-white/10 text-[#C0CCDD] border-white/15"
                          }`}>
                            {loaderBadge}
                          </span>

                          <div className="relative">
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                setContextMenuInstanceId(contextMenuInstanceId === inst.id ? null : inst.id);
                              }}
                              className="w-6 h-6 rounded flex items-center justify-center text-white/70 hover:text-white hover:bg-white/15 transition-colors"
                              title="Options"
                            >
                              <MoreVertical className="w-3.5 h-3.5" />
                            </button>

                            {/* Dropdown Menu */}
                            {contextMenuInstanceId === inst.id && (
                              <div
                                onClick={(e) => e.stopPropagation()}
                                className="absolute right-0 top-7 w-48 rounded-xl bg-[#121622] border border-white/15 shadow-2xl p-1 z-30 flex flex-col text-xs"
                              >
                                <button
                                  type="button"
                                  onClick={() => {
                                    setSelectedInstanceId(inst.id);
                                    setDashboardInstance(inst);
                                    setContextMenuInstanceId(null);
                                  }}
                                  className="flex items-center gap-2 px-3 py-2 rounded-lg text-[#FFA048] hover:bg-white/10 text-left transition-colors cursor-pointer font-semibold"
                                >
                                  <Layers className="w-3.5 h-3.5 text-[#E86A17]" />
                                  <span>📊 View Dashboard</span>
                                </button>

                                <button
                                  type="button"
                                  onClick={() => {
                                    setEditingInstance({ ...inst });
                                    setContextMenuInstanceId(null);
                                  }}
                                  className="flex items-center gap-2 px-3 py-2 rounded-lg text-white/90 hover:bg-white/10 text-left transition-colors cursor-pointer"
                                >
                                  <Pencil className="w-3.5 h-3.5 text-[#E86A17]" />
                                  <span>✎ Edit Instance</span>
                                </button>

                                <button
                                  type="button"
                                  onClick={() => {
                                    if (isTauri()) invoke("open_instance_folder", { id: inst.id });
                                    setContextMenuInstanceId(null);
                                  }}
                                  className="flex items-center gap-2 px-3 py-2 rounded-lg text-white/90 hover:bg-white/10 text-left transition-colors cursor-pointer"
                                >
                                  <FolderOpen className="w-3.5 h-3.5 text-blue-400" />
                                  <span>📂 Open Folder</span>
                                </button>

                                <button
                                  type="button"
                                  onClick={() => {
                                    setEditingInstance({ ...inst });
                                    setContextMenuInstanceId(null);
                                  }}
                                  className="flex items-center gap-2 px-3 py-2 rounded-lg text-white/90 hover:bg-white/10 text-left transition-colors cursor-pointer"
                                >
                                  <ImageIcon className="w-3.5 h-3.5 text-purple-400" />
                                  <span>🖼 Change Cover</span>
                                </button>

                                <div className="h-[1px] bg-white/10 my-1" />

                                <button
                                  type="button"
                                  onClick={() => {
                                    setDeletingInstanceId(inst.id);
                                    setContextMenuInstanceId(null);
                                  }}
                                  className="flex items-center gap-2 px-3 py-2 rounded-lg text-rose-400 hover:bg-rose-500/10 text-left transition-colors cursor-pointer"
                                >
                                  <Trash2 className="w-3.5 h-3.5 text-rose-400" />
                                  <span>🗑 Delete Instance</span>
                                </button>
                              </div>
                            )}
                          </div>
                        </div>
                      </div>

                      {/* Bottom Text Stack */}
                      <div className="space-y-0.5">
                        <h3 className="text-[14px] font-bold text-white truncate drop-shadow-sm leading-tight">
                          {inst.name}
                        </h3>
                        <p className="text-[11px] text-[#8E96A8] truncate">
                          {inst.game_version} • {inst.loader || "vanilla"}
                        </p>
                        <p className="text-[9.5px] text-[#6C7A9C] truncate">
                          {isRunning ? (
                            <span className="text-emerald-400 font-bold">• Running</span>
                          ) : inst.last_played ? (
                            `Played: ${inst.last_played}`
                          ) : (
                            `${inst.memory_max_mb / 1024} GB RAM`
                          )}
                        </p>
                      </div>
                    </div>

                    {/* Interactive Hover Action Overlay */}
                    <div
                      onClick={(e) => e.stopPropagation()}
                      className="absolute inset-0 z-20 flex items-center justify-center gap-2.5 opacity-0 group-hover:opacity-100 transition-opacity duration-200 bg-[#0A0D18]/85 backdrop-blur-[6px] pointer-events-none group-hover:pointer-events-auto"
                    >
                      {/* Console Logs button (>_) */}
                      <button
                        type="button"
                        onClick={() => setShowLogsInstance(inst)}
                        className="w-8 h-8 rounded-full bg-white/10 hover:bg-white/20 border border-white/15 flex items-center justify-center text-white text-xs font-mono transition-transform hover:scale-105 active:scale-95 shadow cursor-pointer"
                        title="Console Logs"
                      >
                        <Terminal className="w-3.5 h-3.5" />
                      </button>

                      {/* Play / Stop button */}
                      <button
                        type="button"
                        onClick={() => handleLaunchInstance(inst)}
                        className={`h-[34px] px-3.5 rounded-[10px] text-white font-bold text-[11px] flex items-center gap-1.5 shadow-lg transition-transform hover:scale-105 active:scale-95 cursor-pointer ${
                          isRunning
                            ? "bg-rose-600 hover:bg-rose-500 shadow-rose-900/40"
                            : "bg-[#E86A17] hover:bg-[#F97316] shadow-[#E86A17]/35"
                        }`}
                      >
                        {isRunning ? (
                          <>
                            <Square className="w-3 h-3 fill-current" />
                            <span>Stop</span>
                          </>
                        ) : (
                          <>
                            <Play className="w-3 h-3 fill-current" />
                            <span>Play Now</span>
                          </>
                        )}
                      </button>

                      {/* Settings / Edit button */}
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedInstanceId(inst.id);
                          setDashboardInstance(inst);
                        }}
                        className="w-8 h-8 rounded-full bg-white/10 hover:bg-white/20 border border-white/15 flex items-center justify-center text-white transition-transform hover:scale-105 active:scale-95 shadow cursor-pointer"
                        title="Open Instance Dashboard"
                      >
                        <SettingsIcon className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
            </>
            )}
          </div>
        )}

        {/* =========================================================================
            TAB 3: MODS PAGE (Modrinth Store)
            ========================================================================= */}
        {activeTab === "mods" && (
          <div className="px-8 pb-8 pt-28 max-w-6xl mx-auto space-y-6 overflow-y-auto h-full">
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-2xl font-bold text-white tracking-tight">Mods (Modrinth)</h1>
                <p className="text-[13px] text-white/70 mt-0.5">
                  Browse and install mods directly into <span className="text-[#E86A17] font-semibold">{selectedInstance?.name}</span>
                </p>
              </div>

              <form onSubmit={handleModSearch} className="flex items-center gap-2">
                <Dropdown
                  ariaLabel="Content type"
                  className="w-40"
                  value={modType}
                  onChange={(v) => {
                    setModType(v);
                    setModResults([]);
                  }}
                  options={[
                    { value: "mod", label: "Mods" },
                    { value: "modpack", label: "Modpacks" },
                    { value: "resourcepack", label: "Resource Packs" },
                    { value: "shader", label: "Shader Packs" },
                  ]}
                />
                <Dropdown
                  ariaLabel="Sort by"
                  className="w-44"
                  value={modSort}
                  onChange={setModSort}
                  options={[
                    { value: "follows", label: "Recommended" },
                    { value: "relevance", label: "Relevance" },
                    { value: "downloads", label: "Most downloaded" },
                    { value: "newest", label: "Newest" },
                    { value: "updated", label: "Recently updated" },
                  ]}
                />
                <div className="relative">
                  <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-[#8E96A8]" />
                  <input
                    type="text"
                    placeholder={`Search Modrinth ${modType === "mod" ? "mods" : modType === "modpack" ? "modpacks" : modType === "resourcepack" ? "resource packs" : "shader packs"}...`}
                    value={modQuery}
                    onChange={(e) => setModQuery(e.target.value)}
                    className="h-9 w-64 pl-9 pr-3 rounded-xl bg-[#141824] border border-[#23293D] text-[13px] text-white placeholder-[#8E96A8] focus:outline-none focus:border-[#E86A17]"
                  />
                </div>
                <button
                  type="submit"
                  className="px-4 py-2 rounded-xl bg-[#E86A17] text-white text-xs font-bold hover:bg-[#F97316] transition-colors"
                >
                  Search
                </button>
              </form>
            </div>

            {modLoading ? (
              <div className="flex items-center justify-center py-20 text-[#8E96A8] gap-3">
                <RefreshCw className="w-5 h-5 animate-spin text-[#E86A17]" />
                <span>Searching Modrinth...</span>
              </div>
            ) : modResults.length > 0 ? (
              <div className="grid grid-cols-3 gap-4">
                {modResults.map((mod) => {
                  const isInstalled = modType === "mod" && installedMods.some((m) => m.name.toLowerCase() === mod.title.toLowerCase() || m.filename.includes(mod.slug));
                  const isInstalling = installingModId === mod.project_id;

                  return (
                    <div key={mod.project_id} className="rounded-[18px] p-5 glass border border-white/[0.08] flex flex-col justify-between">
                      <div className="space-y-3">
                        <div className="flex items-start gap-3">
                          {mod.icon_url ? (
                            <img src={mod.icon_url} alt={mod.title} className="w-11 h-11 rounded-lg object-contain bg-black/30" />
                          ) : (
                            <div className="w-11 h-11 rounded-lg bg-[#E86A17]/20 flex items-center justify-center font-bold text-[#E86A17]">
                              {mod.title.charAt(0)}
                            </div>
                          )}
                          <div className="flex-1 min-w-0">
                            <h3 className="font-bold text-white text-[14px] truncate">{mod.title}</h3>
                            <p className="text-[11px] text-[#8E96A8]">by {mod.author}</p>
                          </div>
                        </div>

                        <p className="text-[12px] text-[#C5CCD8] line-clamp-2 leading-relaxed">
                          {mod.description}
                        </p>
                      </div>

                      <div className="mt-4 flex items-center justify-between pt-3 border-t border-white/[0.08]">
                        <span className="text-[11px] text-[#8E96A8]">{Number(mod.downloads).toLocaleString()} dl</span>
                        <button
                          onClick={() => handleInstallMod(mod)}
                          disabled={isInstalled || isInstalling}
                          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold ${
                            isInstalled
                              ? "bg-emerald-500/20 text-emerald-400"
                              : isInstalling
                              ? "bg-[#E86A17]/20 text-[#E86A17] animate-pulse"
                              : "bg-[#1A1F2C] border border-[#E86A17]/50 text-[#E86A17] hover:bg-[#E86A17] hover:text-white transition-colors"
                          }`}
                        >
                          {isInstalled ? <Check className="w-3.5 h-3.5" /> : isInstalling ? <RefreshCw className="w-3.5 h-3.5 animate-spin" /> : <Download className="w-3.5 h-3.5" />}
                          {isInstalled ? "Installed" : isInstalling ? "Installing..." : "Install"}
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="text-center py-20 glass rounded-[18px] border border-white/[0.08]">
                <Puzzle className="w-10 h-10 mx-auto text-[#8E96A8] mb-2" />
                <h3 className="text-base font-bold text-white">Search Modrinth</h3>
                <p className="text-xs text-[#8E96A8] mt-1">Search and install thousands of mods with one click</p>
              </div>
            )}
          </div>
        )}

        {/* =========================================================================
            TAB 4: CUSTOMIZE (Skins & Capes)
            ========================================================================= */}
        {activeTab === "customize" && (
          <CustomizeTab
            username={activeAccount.username}
            skin={activeAccount.skin_preset}
            skinUrl={activeAccount.skin_url ?? null}
            cape={activeAccount.cape_preset}
            skins={skins}
            capes={capes}
            onSkin={(s) => {
              setAccounts((prev) => prev.map((a) => (a.is_active ? { ...a, skin_preset: s, skin_url: null } : a)));
              if (isTauri()) invoke("set_active_skin", { skinUrl: null, preset: s }).catch((err) => alert(err));
            }}
            onUseSkin={(url) => {
              setAccounts((prev) => prev.map((a) => (a.is_active ? { ...a, skin_url: url } : a)));
              if (isTauri()) invoke("set_active_skin", { skinUrl: url, preset: null }).catch((err) => alert(err));
            }}
            onCape={async (c, data) => {
              const capeData = data ?? (c ? await urlToDataUrl(`/capes/${c}.png`) : null);
              setAccounts((prev) =>
                prev.map((a) => (a.is_active ? { ...a, cape_preset: c, cape_data: capeData } : a))
              );
              if (isTauri()) invoke("set_active_cape", { cape: c, capeData }).catch((err) => alert(err));
            }}
            prefs={prefs}
            onPrefs={setPrefs}
          />
        )}

        {/* =========================================================================
            TAB 5: SETTINGS
            ========================================================================= */}
        {activeTab === "settings" && (
          <div className="px-8 pb-8 pt-28 max-w-4xl mx-auto space-y-6 overflow-y-auto h-full">
            <div>
              <h1 className="text-2xl font-bold text-white tracking-tight">Settings & Engine</h1>
              <p className="text-[13px] text-white/70 mt-0.5">Configure graphics offload, memory, and Horizon Rust engine</p>
            </div>

            <div className="space-y-4">
              {/* Horizon Rust Engine Compiler */}
              <div className="rounded-[18px] p-6 glass border border-white/[0.08] flex items-center justify-between">
                <div>
                  <h3 className="text-base font-bold text-white flex items-center gap-2">
                    <Zap className="w-5 h-5 text-[#E86A17]" />
                    Horizon Rust Engine
                  </h3>
                  <p className="text-xs text-[#8E96A8] mt-0.5">
                    Build `crates/horizon-client` with the fast optimized profile
                  </p>
                </div>

                <button
                  onClick={handleBuildHorizon}
                  disabled={isBuildingHorizon}
                  className="px-4 py-2 rounded-xl bg-[#E86A17] text-white text-xs font-bold hover:bg-[#F97316] transition-colors"
                >
                  {isBuildingHorizon ? "Compiling Engine..." : "Compile Engine"}
                </button>
              </div>

              {buildStatus && (
                <div className="p-3 rounded-xl bg-[#090B12] border border-white/[0.08] font-mono text-xs text-emerald-400">
                  {buildStatus}
                </div>
              )}

              {/* Hardware Acceleration */}
              <div className="rounded-[18px] p-6 glass border border-white/[0.08] space-y-4">
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <Cpu className="w-5 h-5 text-[#E86A17]" />
                  Performance Acceleration
                </h3>

                <div className="flex items-center justify-between border-b border-white/[0.08] pb-4">
                  <div>
                    <h4 className="text-sm font-semibold text-white">NVIDIA PRIME Discrete GPU Offload</h4>
                    <p className="text-xs text-[#8E96A8]">
                      Sets `__NV_PRIME_RENDER_OFFLOAD=1` and Vulkan wgpu backend
                    </p>
                  </div>
                  <input
                    type="checkbox"
                    checked={settings.discrete_gpu}
                    onChange={(e) => setSettings({ ...settings, discrete_gpu: e.target.checked })}
                    className="w-4 h-4 accent-[#E86A17] cursor-pointer"
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <h4 className="text-sm font-semibold text-white">Close Launcher on Start</h4>
                    <p className="text-xs text-[#8E96A8]">Minimize launcher once Minecraft is launched</p>
                  </div>
                  <input
                    type="checkbox"
                    checked={settings.close_on_launch}
                    onChange={(e) => setSettings({ ...settings, close_on_launch: e.target.checked })}
                    className="w-4 h-4 accent-[#E86A17] cursor-pointer"
                  />
                </div>
              </div>

              {/* Resource Paths */}
              <div className="rounded-[18px] p-6 glass border border-white/[0.08] space-y-3">
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <HardDrive className="w-5 h-5 text-[#E86A17]" />
                  Storage Paths
                </h3>
                <div className="space-y-1">
                  <label className="text-xs text-[#8E96A8] font-semibold">Horizon Prepared Resources</label>
                  <input
                    type="text"
                    value={settings.horizon_resources_path}
                    onChange={(e) => setSettings({ ...settings, horizon_resources_path: e.target.value })}
                    className="w-full px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] text-xs text-white font-mono"
                  />
                </div>
              </div>
            </div>
          </div>
        )}

        {/* =========================================================================
            TAB 6: SERVERS PAGE
            ========================================================================= */}
        {activeTab === "servers" && (
          <div className="px-8 pb-8 pt-28 max-w-5xl mx-auto space-y-6 overflow-y-auto h-full">
            <div>
              <h1 className="text-2xl font-bold text-white tracking-tight">Featured Minecraft Servers</h1>
              <p className="text-[13px] text-white/70 mt-0.5">Popular servers with one-click IP copy</p>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="rounded-[18px] p-5 glass border border-white/[0.08] flex items-center justify-between">
                <div className="space-y-1">
                  <span className="text-[9px] font-bold uppercase text-[#7E6AFF] bg-[#7E6AFF]/15 px-2 py-0.5 rounded">FEATURED</span>
                  <h3 className="text-base font-bold text-white">BreakPoint MC</h3>
                  <p className="text-xs text-[#8E96A8]">breakpoint.mcsrv.net</p>
                </div>
                <button
                  onClick={() => handleCopyIp("breakpoint.mcsrv.net", "bp-full")}
                  className="px-3.5 py-1.5 rounded-lg bg-[#E86A17] text-white text-xs font-bold"
                >
                  {copiedServer === "bp-full" ? "Copied!" : "Copy IP"}
                </button>
              </div>

              <div className="rounded-[18px] p-5 glass border border-white/[0.08] flex items-center justify-between">
                <div className="space-y-1">
                  <span className="text-[9px] font-bold uppercase text-[#F4C430] bg-[#F4C430]/15 px-2 py-0.5 rounded">MINI-GAMES</span>
                  <h3 className="text-base font-bold text-white">Hypixel</h3>
                  <p className="text-xs text-[#8E96A8]">mc.hypixel.net</p>
                </div>
                <button
                  onClick={() => handleCopyIp("mc.hypixel.net", "hyp-full")}
                  className="px-3.5 py-1.5 rounded-lg bg-[#E86A17] text-white text-xs font-bold"
                >
                  {copiedServer === "hyp-full" ? "Copied!" : "Copy IP"}
                </button>
              </div>

              <div className="rounded-[18px] p-5 glass border border-white/[0.08] flex items-center justify-between">
                <div className="space-y-1">
                  <span className="text-[9px] font-bold uppercase text-[#FF8C42] bg-[#FF8C42]/15 px-2 py-0.5 rounded">SURVIVAL</span>
                  <h3 className="text-base font-bold text-white">Donut SMP</h3>
                  <p className="text-xs text-[#8E96A8]">play.donutsmp.net</p>
                </div>
                <button
                  onClick={() => handleCopyIp("play.donutsmp.net", "donut-full")}
                  className="px-3.5 py-1.5 rounded-lg bg-[#E86A17] text-white text-xs font-bold"
                >
                  {copiedServer === "donut-full" ? "Copied!" : "Copy IP"}
                </button>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* =========================================================================
          CREATE INSTANCE MODAL
          ========================================================================= */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fugo-modal-backdrop p-4">
          <div className="w-full max-w-lg rounded-[20px] p-6 fugo-modal-content space-y-4 shadow-2xl">
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-3">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <Plus className="w-4 h-4 text-[#E86A17]" />
                <span>Add New Instance</span>
              </h3>
              <button
                type="button"
                onClick={() => setShowCreateModal(false)}
                className="text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-4 max-h-[70vh] overflow-y-auto pr-1">
              {/* Instance Name */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Instance Name</label>
                <input
                  type="text"
                  placeholder="e.g. Fabulously Optimized"
                  value={newInstanceName}
                  onChange={(e) => setNewInstanceName(e.target.value)}
                  className="w-full px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] text-sm text-white mt-1 focus:border-[#E86A17] focus:outline-none"
                />
              </div>

              {/* Edition */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Edition</label>
                <div className="grid grid-cols-2 gap-2 mt-1">
                  <button
                    type="button"
                    onClick={() => {
                      setNewInstanceEdition("rust");
                      setNewInstanceVersion("26.3");
                    }}
                    className={`py-2 px-3 rounded-xl text-xs font-bold border transition-colors flex items-center justify-center gap-2 cursor-pointer ${
                      newInstanceEdition === "rust"
                        ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:bg-[#1A2030]"
                    }`}
                  >
                    <Zap className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Horizon Rust (26.3)</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setNewInstanceEdition("java");
                      setNewInstanceVersion("1.21.1");
                    }}
                    className={`py-2 px-3 rounded-xl text-xs font-bold border transition-colors flex items-center justify-center gap-2 cursor-pointer ${
                      newInstanceEdition === "java"
                        ? "bg-[#E86A17]/20 text-[#E86A17] border-[#E86A17]/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:bg-[#1A2030]"
                    }`}
                  >
                    <Puzzle className="w-3.5 h-3.5 text-[#E86A17]" />
                    <span>Java Edition</span>
                  </button>
                </div>
              </div>

              {/* Mod Loader and build (Java Edition only) */}
              {newInstanceEdition === "java" && (
                <LoaderPicker
                  loader={newInstanceLoader}
                  loaderVersion={newInstanceLoaderVersion}
                  gameVersion={newInstanceVersion}
                  onLoaderChange={setNewInstanceLoader}
                  onLoaderVersionChange={setNewInstanceLoaderVersion}
                />
              )}

              {/* Game Version */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Game Version</label>
                <input
                  type="text"
                  value={newInstanceVersion}
                  onChange={(e) => setNewInstanceVersion(e.target.value)}
                  className="w-full px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] text-sm text-white mt-1 focus:border-[#E86A17] focus:outline-none"
                />
              </div>

              {/* Memory Slider */}
              <div>
                <div className="flex items-center justify-between text-xs">
                  <span className="font-semibold text-[#8E96A8]">Allocated Memory</span>
                  <span className="text-[#FFA048] font-bold">{(newInstanceRam / 1024).toFixed(1)} GB ({newInstanceRam} MB)</span>
                </div>
                <input
                  type="range"
                  min={1024}
                  max={16384}
                  step={512}
                  value={newInstanceRam}
                  onChange={(e) => setNewInstanceRam(Number(e.target.value))}
                  className="w-full mt-2 accent-[#E86A17] cursor-pointer"
                />
              </div>

              {/* Cover Image Picker */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8] block mb-1.5">Cover Art</label>
                <div className="grid grid-cols-4 gap-2">
                  {PRESET_COVERS.map((cov) => {
                    const isSelected = newInstanceCover === cov.file;
                    return (
                      <div
                        key={cov.file}
                        onClick={() => setNewInstanceCover(cov.file)}
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
            </div>

            <div className="flex items-center justify-end gap-2.5 pt-3 border-t border-white/[0.08]">
              <button
                type="button"
                onClick={() => setShowCreateModal(false)}
                className="px-4 py-2 text-xs font-semibold text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleCreateInstance}
                className="px-5 py-2 rounded-xl bg-[#E86A17] text-white text-xs font-bold hover:bg-[#F97316] transition-colors shadow-md shadow-[#E86A17]/25 cursor-pointer active:scale-95"
              >
                Create Instance
              </button>
            </div>
          </div>
        </div>
      )}

      {/* =========================================================================
          IMPORT DIRECTORY MODAL
          ========================================================================= */}
      {showImportDirModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fugo-modal-backdrop p-4">
          <div className="w-full max-w-lg rounded-[20px] p-6 fugo-modal-content space-y-4 shadow-2xl">
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-3">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <FolderOpen className="w-4 h-4 text-[#E86A17]" />
                <span>Import Instance Directory</span>
              </h3>
              <button
                type="button"
                onClick={() => setShowImportDirModal(false)}
                className="text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <p className="text-xs text-[#8E96A8]">
              Select an existing Minecraft instance directory or paste its path. It will be imported into <span className="text-[#E86A17] font-mono">~/.minecraft/death-client/instances/</span> with its mods, configs, and save data.
            </p>

            <div className="space-y-3">
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Directory Path</label>
                <div className="flex gap-2 mt-1">
                  <input
                    type="text"
                    placeholder="/path/to/minecraft/instance"
                    value={importDirPath}
                    onChange={(e) => setImportDirPath(e.target.value)}
                    className="flex-1 px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] font-mono text-xs text-white focus:border-[#E86A17] focus:outline-none"
                  />
                  <button
                    type="button"
                    onClick={async () => {
                      if (isTauri()) {
                        const picked: string | null = await invoke("pick_directory");
                        if (picked) setImportDirPath(picked);
                      }
                    }}
                    className="px-3 py-2 rounded-xl bg-white/10 hover:bg-white/15 text-xs text-white font-semibold transition-all cursor-pointer"
                  >
                    Browse...
                  </button>
                </div>
              </div>

              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Edition Detection</label>
                <div className="grid grid-cols-3 gap-2 mt-1">
                  <button
                    type="button"
                    onClick={() => setImportEdition("auto")}
                    className={`py-2 px-2 rounded-xl text-[11px] font-bold border transition-colors flex items-center justify-center gap-1.5 cursor-pointer ${
                      importEdition === "auto"
                        ? "bg-purple-500/20 text-purple-300 border-purple-500/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>⚡ Auto-Detect</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setImportEdition("rust")}
                    className={`py-2 px-2 rounded-xl text-[11px] font-bold border transition-colors flex items-center justify-center gap-1.5 cursor-pointer ${
                      importEdition === "rust"
                        ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>🦀 Horizon (Rust)</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setImportEdition("java")}
                    className={`py-2 px-2 rounded-xl text-[11px] font-bold border transition-colors flex items-center justify-center gap-1.5 cursor-pointer ${
                      importEdition === "java"
                        ? "bg-[#E86A17]/20 text-[#FFA048] border-[#E86A17]/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>☕ Java Edition</span>
                  </button>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-end gap-3 pt-3 border-t border-white/[0.08]">
              <button
                type="button"
                onClick={() => setShowImportDirModal(false)}
                className="px-4 py-2 rounded-xl text-xs font-semibold text-[#8E96A8] hover:text-white hover:bg-white/5 transition-all cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={!importDirPath.trim() || isImporting}
                onClick={handleManualImportDir}
                className="px-5 py-2 rounded-xl bg-[#E86A17] hover:bg-[#F97316] disabled:opacity-50 text-white text-xs font-bold transition-all shadow-md shadow-[#E86A17]/25 cursor-pointer"
              >
                {isImporting ? "Importing..." : "Import Instance"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* =========================================================================
          IMPORT ZIP MODAL
          ========================================================================= */}
      {showImportZipModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fugo-modal-backdrop p-4">
          <div className="w-full max-w-lg rounded-[20px] p-6 fugo-modal-content space-y-4 shadow-2xl">
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-3">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <FolderDown className="w-4 h-4 text-[#E86A17]" />
                <span>Import Instance Zip Backup</span>
              </h3>
              <button
                type="button"
                onClick={() => setShowImportZipModal(false)}
                className="text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <p className="text-xs text-[#8E96A8]">
              Select a .zip profile backup or instance archive. It will be extracted and imported directly into <span className="text-[#E86A17] font-mono">~/.minecraft/death-client/instances/</span>.
            </p>

            <div className="space-y-3">
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Zip File Path</label>
                <div className="flex gap-2 mt-1">
                  <input
                    type="text"
                    placeholder="/path/to/backup.zip"
                    value={importZipPath}
                    onChange={(e) => setImportZipPath(e.target.value)}
                    className="flex-1 px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] font-mono text-xs text-white focus:border-[#E86A17] focus:outline-none"
                  />
                  <button
                    type="button"
                    onClick={async () => {
                      if (isTauri()) {
                        const picked: string | null = await invoke("pick_zip_file");
                        if (picked) setImportZipPath(picked);
                      }
                    }}
                    className="px-3 py-2 rounded-xl bg-white/10 hover:bg-white/15 text-xs text-white font-semibold transition-all cursor-pointer"
                  >
                    Browse...
                  </button>
                </div>
              </div>

              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Edition Detection</label>
                <div className="grid grid-cols-3 gap-2 mt-1">
                  <button
                    type="button"
                    onClick={() => setImportEdition("auto")}
                    className={`py-2 px-2 rounded-xl text-[11px] font-bold border transition-colors flex items-center justify-center gap-1.5 cursor-pointer ${
                      importEdition === "auto"
                        ? "bg-purple-500/20 text-purple-300 border-purple-500/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>⚡ Auto-Detect</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setImportEdition("rust")}
                    className={`py-2 px-2 rounded-xl text-[11px] font-bold border transition-colors flex items-center justify-center gap-1.5 cursor-pointer ${
                      importEdition === "rust"
                        ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>🦀 Horizon (Rust)</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setImportEdition("java")}
                    className={`py-2 px-2 rounded-xl text-[11px] font-bold border transition-colors flex items-center justify-center gap-1.5 cursor-pointer ${
                      importEdition === "java"
                        ? "bg-[#E86A17]/20 text-[#FFA048] border-[#E86A17]/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>☕ Java Edition</span>
                  </button>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-end gap-3 pt-3 border-t border-white/[0.08]">
              <button
                type="button"
                onClick={() => setShowImportZipModal(false)}
                className="px-4 py-2 rounded-xl text-xs font-semibold text-[#8E96A8] hover:text-white hover:bg-white/5 transition-all cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={!importZipPath.trim() || isImporting}
                onClick={handleManualImportZip}
                className="px-5 py-2 rounded-xl bg-[#E86A17] hover:bg-[#F97316] disabled:opacity-50 text-white text-xs font-bold transition-all shadow-md shadow-[#E86A17]/25 cursor-pointer"
              >
                {isImporting ? "Extracting & Importing..." : "Import Zip"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* =========================================================================
          EDIT INSTANCE MODAL
          ========================================================================= */}
      {editingInstance && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fugo-modal-backdrop p-4">
          <div className="w-full max-w-lg rounded-[20px] p-6 fugo-modal-content space-y-4 shadow-2xl">
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-3">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <Pencil className="w-4 h-4 text-[#E86A17]" />
                <span>Edit Instance — {editingInstance.name}</span>
              </h3>
              <button
                type="button"
                onClick={() => setEditingInstance(null)}
                className="text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-4 max-h-[70vh] overflow-y-auto pr-1">
              {/* Name */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Instance Name</label>
                <input
                  type="text"
                  value={editingInstance.name}
                  onChange={(e) => setEditingInstance({ ...editingInstance, name: e.target.value })}
                  className="w-full px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] text-sm text-white mt-1 focus:border-[#E86A17] focus:outline-none"
                />
              </div>

              {/* Edition */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Edition</label>
                <div className="grid grid-cols-2 gap-2 mt-1">
                  <button
                    type="button"
                    onClick={() => {
                      setEditingInstance({
                        ...editingInstance,
                        edition: "rust",
                        loader: "horizon-rust",
                        game_version: "26.3"
                      });
                    }}
                    className={`py-2 px-3 rounded-xl text-xs font-bold border transition-colors flex items-center justify-center gap-2 cursor-pointer ${
                      editingInstance.edition === "rust"
                        ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>🦀 Horizon (Rust)</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setEditingInstance({
                        ...editingInstance,
                        edition: "java",
                        loader: editingInstance.loader === "horizon-rust" ? "vanilla" : editingInstance.loader
                      });
                    }}
                    className={`py-2 px-3 rounded-xl text-xs font-bold border transition-colors flex items-center justify-center gap-2 cursor-pointer ${
                      editingInstance.edition === "java"
                        ? "bg-[#E86A17]/20 text-[#FFA048] border-[#E86A17]/60 shadow-sm"
                        : "bg-[#141824] text-[#8E96A8] border-[#23293D] hover:text-white"
                    }`}
                  >
                    <span>☕ Java Edition</span>
                  </button>
                </div>
              </div>

              {editingInstance.edition === "java" && (
                <LoaderPicker
                  loader={(editingInstance.loader as LoaderName) || "vanilla"}
                  loaderVersion={editingInstance.loader_version ?? null}
                  gameVersion={editingInstance.game_version}
                  onLoaderChange={(l) => setEditingInstance({ ...editingInstance, loader: l, loader_version: null })}
                  onLoaderVersionChange={(v) => setEditingInstance({ ...editingInstance, loader_version: v })}
                />
              )}

              {/* Memory Slider */}
              <div>
                <div className="flex items-center justify-between text-xs">
                  <span className="font-semibold text-[#8E96A8]">Maximum Memory (RAM)</span>
                  <span className="text-[#FFA048] font-bold">
                    {(editingInstance.memory_max_mb / 1024).toFixed(1)} GB ({editingInstance.memory_max_mb} MB)
                  </span>
                </div>
                <input
                  type="range"
                  min={1024}
                  max={16384}
                  step={512}
                  value={editingInstance.memory_max_mb}
                  onChange={(e) =>
                    setEditingInstance({ ...editingInstance, memory_max_mb: Number(e.target.value) })
                  }
                  className="w-full mt-2 accent-[#E86A17] cursor-pointer"
                />
              </div>

              {/* Java Arguments */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8]">Java Arguments</label>
                <input
                  type="text"
                  value={editingInstance.java_args || ""}
                  onChange={(e) => setEditingInstance({ ...editingInstance, java_args: e.target.value })}
                  placeholder="-Xmx4G -XX:+UseG1GC"
                  className="w-full px-3 py-2 rounded-xl bg-[#141824] border border-[#23293D] font-mono text-xs text-white mt-1 focus:border-[#E86A17] focus:outline-none"
                />
              </div>

              {/* Cover Art Picker */}
              <div>
                <label className="text-xs font-semibold text-[#8E96A8] block mb-1.5">Change Cover Art</label>
                <div className="grid grid-cols-4 gap-2">
                  {PRESET_COVERS.map((cov) => {
                    const isSelected = editingInstance.banner === cov.file;
                    return (
                      <div
                        key={cov.file}
                        onClick={() => setEditingInstance({ ...editingInstance, banner: cov.file })}
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
            </div>

            <div className="flex items-center justify-end gap-2.5 pt-3 border-t border-white/[0.08]">
              <button
                type="button"
                onClick={() => setEditingInstance(null)}
                className="px-4 py-2 text-xs font-semibold text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleSaveEdit}
                className="px-5 py-2 rounded-xl bg-[#E86A17] text-white text-xs font-bold hover:bg-[#F97316] transition-colors shadow-md shadow-[#E86A17]/25 cursor-pointer active:scale-95"
              >
                Save Changes
              </button>
            </div>
          </div>
        </div>
      )}

      {/* =========================================================================
          DELETE INSTANCE CONFIRMATION MODAL
          ========================================================================= */}
      {deletingInstanceId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fugo-modal-backdrop p-4">
          <div className="w-full max-w-sm rounded-[20px] p-6 fugo-modal-content space-y-4 shadow-2xl">
            <div className="flex items-center gap-3 text-rose-400">
              <div className="w-9 h-9 rounded-full bg-rose-500/15 border border-rose-500/30 flex items-center justify-center shrink-0">
                <Trash2 className="w-4 h-4 text-rose-400" />
              </div>
              <div>
                <h3 className="text-base font-bold text-white">Delete Instance?</h3>
                <p className="text-[11px] text-[#8E96A8]">This action cannot be undone.</p>
              </div>
            </div>

            <p className="text-xs text-[#C0CCDD] leading-relaxed">
              Are you sure you want to delete{" "}
              <span className="font-bold text-white">
                {instances.find((i) => i.id === deletingInstanceId)?.name}
              </span>
              ? All worlds, installed mods, and configurations inside this instance will be permanently removed.
            </p>

            <div className="flex items-center justify-end gap-2.5 pt-2 border-t border-white/[0.08]">
              <button
                type="button"
                onClick={() => setDeletingInstanceId(null)}
                className="px-4 py-2 text-xs font-semibold text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => handleDeleteInstance(deletingInstanceId)}
                className="px-4 py-2 rounded-xl bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold transition-colors shadow-md shadow-rose-900/40 cursor-pointer active:scale-95"
              >
                Delete Permanently
              </button>
            </div>
          </div>
        </div>
      )}

      {/* =========================================================================
          CONSOLE LOGS DRAWER / MODAL
          ========================================================================= */}
      {showLogsInstance && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fugo-modal-backdrop p-6">
          <div className="w-full max-w-3xl h-[460px] rounded-[20px] p-6 fugo-modal-content flex flex-col justify-between shadow-2xl">
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-3">
              <div className="flex items-center gap-2">
                <Terminal className="w-4 h-4 text-[#E86A17]" />
                <h3 className="text-sm font-bold text-white font-mono">
                  Console Logs — {showLogsInstance.name}
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setShowLogsInstance(null)}
                className="text-[#8E96A8] hover:text-white transition-colors cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Terminal Window Output */}
            <div className="flex-1 my-4 p-4 rounded-xl bg-[#06080F] border border-white/10 font-mono text-[11px] text-emerald-400 overflow-y-auto space-y-1 select-text">
              <p className="text-[#6C7A9C]">// Fugo Launcher Process Supervisor</p>
              <p className="text-[#6C7A9C]">// Instance Directory: {showLogsInstance.game_dir}</p>
              <p className="text-[#6C7A9C]">// Game Version: {showLogsInstance.game_version} ({showLogsInstance.edition})</p>
              <p className="text-[#8E96A8]">[{new Date().toLocaleTimeString()}] [Client thread/INFO]: Setting user: {activeAccount.username}</p>
              <p className="text-emerald-400">[{new Date().toLocaleTimeString()}] [Client thread/INFO]: Backend engine: {showLogsInstance.edition === "rust" ? "Horizon (wgpu/Vulkan)" : "Java HotSpot"}</p>
              <p className="text-[#8E96A8]">[{new Date().toLocaleTimeString()}] [Client thread/INFO]: Memory heap limit: {showLogsInstance.memory_max_mb} MB</p>
              {runningInstances.includes(showLogsInstance.id) ? (
                <p className="text-emerald-300 font-bold animate-pulse">● Instance is currently executing and active.</p>
              ) : (
                <p className="text-[#8E96A8]">• Process currently idle. Click "Play Now" to launch this instance.</p>
              )}
            </div>

            <div className="flex items-center justify-between pt-2 border-t border-white/[0.08]">
              <span className="text-[10px] text-[#6C7A9C] font-mono">Status: {runningInstances.includes(showLogsInstance.id) ? "ACTIVE" : "IDLE"}</span>
              <button
                type="button"
                onClick={() => setShowLogsInstance(null)}
                className="px-4 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white text-xs font-semibold transition-colors cursor-pointer"
              >
                Close Logs
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
