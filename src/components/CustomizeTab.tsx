import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Download, FolderOpen, Flag, MousePointer2, Palette, User, ChevronLeft, ChevronRight } from "lucide-react";
import { BACKGROUNDS, CURSOR_PACKS, MAX_CUSTOM_BYTES, Prefs } from "../customize";
import SkinModel3D from "./SkinModel3D";

type Section = "cursors" | "skin" | "cape" | "themes";

export interface PlayerTextures {
  name: string;
  uuid: string;
  skin_url: string | null;
  cape_url: string | null;
  slim: boolean;
}

const SECTIONS: { id: Section; label: string; icon: typeof User }[] = [
  { id: "cursors", label: "Cursor", icon: MousePointer2 },
  { id: "skin", label: "Skin", icon: User },
  { id: "cape", label: "Cape", icon: Flag },
  { id: "themes", label: "Themes", icon: Palette },
];

/** Players whose skins fill the carousel. Each is looked up once and cached by the launcher. */
const FEATURED = ["Dream", "Technoblade", "Notch", "jeb_", "Grian", "Skeppy"];
const PAGE_SIZE = 3;

interface Props {
  username: string;
  skin: string | null;
  skinUrl: string | null;
  cape: string | null;
  skins: string[];
  capes: string[];
  onSkin: (skin: string) => void;
  onUseSkin: (url: string) => void;
  onCape: (cape: string | null, data: string | null) => void;
  prefs: Prefs;
  onPrefs: (prefs: Prefs) => void;
}

const cardBase = "rounded-[18px] p-5 glass border border-white/[0.08]";

async function lookUpPlayer(name: string): Promise<PlayerTextures | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<PlayerTextures>("lookup_player", { username: name });
  } catch {
    return null;
  }
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(new Error("Could not read that file."));
    reader.readAsDataURL(file);
  });
}

export default function CustomizeTab({ username, skin, skinUrl, cape, skins, capes, onSkin, onUseSkin, onCape, prefs, onPrefs }: Props) {
  const [section, setSection] = useState<Section>("cursors");
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [looking, setLooking] = useState(false);
  const [lookupError, setLookupError] = useState<string | null>(null);
  const [featured, setFeatured] = useState<(PlayerTextures | null)[]>([]);
  const [page, setPage] = useState(0);
  // The figure shown large on the right: a looked-up player or an imported file.
  const [chosen, setChosen] = useState<PlayerTextures | null>(null);
  const [chosenCapeName, setChosenCapeName] = useState<string | null>(null);

  // Load the carousel's players once the skin or cape page opens.
  useEffect(() => {
    if (section !== "skin" && section !== "cape") return;
    if (featured.length > 0 || !isTauri()) return;
    Promise.all(FEATURED.map(lookUpPlayer)).then(setFeatured);
  }, [section, featured.length]);

  const lookUp = async (e?: React.FormEvent) => {
    e?.preventDefault();
    const name = query.trim();
    if (!name) return;
    setLookupError(null);
    if (!isTauri()) {
      setLookupError("Player lookup works in the launcher app.");
      return;
    }
    setLooking(true);
    try {
      const found = await invoke<PlayerTextures>("lookup_player", { username: name });
      setChosen(found);
      setChosenCapeName(null);
    } catch (err: any) {
      setLookupError(String(err));
    } finally {
      setLooking(false);
    }
  };

  const importSkin = async (file: File | undefined) => {
    setLookupError(null);
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      setLookupError("That file is not an image.");
      return;
    }
    if (file.size > MAX_CUSTOM_BYTES) {
      setLookupError("Pick an image under 2 MB.");
      return;
    }
    const url = await readFileAsDataUrl(file);
    setChosen({ name: file.name.replace(/\.[^.]+$/, ""), uuid: "", skin_url: url, cape_url: null, slim: false });
  };

  const pickCustomImage = (file: File | undefined) => {
    setUploadError(null);
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      setUploadError("That file is not an image.");
      return;
    }
    if (file.size > MAX_CUSTOM_BYTES) {
      setUploadError("Pick an image under 2 MB.");
      return;
    }
    readFileAsDataUrl(file).then((url) => onPrefs({ ...prefs, background: "custom", customBackground: url }));
  };

  const pills = (
    <nav aria-label="Customize sections" className="flex justify-center">
      <div className="inline-flex items-center gap-1 p-1 rounded-full bg-[#0B0D14]/60 border border-white/10 backdrop-blur-xl">
        {SECTIONS.map((s) => {
          const Icon = s.icon;
          const active = section === s.id;
          return (
            <button
              key={s.id}
              type="button"
              onClick={() => setSection(s.id)}
              className={`flex items-center gap-2 px-5 h-10 rounded-full text-[13px] font-semibold transition-colors cursor-pointer ${
                active ? "bg-white text-[#0B0D14]" : "text-[#8E96A8] hover:text-white"
              }`}
            >
              <Icon className="w-4 h-4" />
              {s.label}
            </button>
          );
        })}
      </div>
    </nav>
  );

  const lookupBar = (
    <form onSubmit={lookUp} className="flex items-center gap-2 p-1.5 pl-4 rounded-full bg-[#0B0D14]/60 border border-[#23293D] backdrop-blur-xl w-full max-w-xl">
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={`Enter Username (e.g. Dream, ${username})`}
        maxLength={16}
        className="flex-1 min-w-0 bg-transparent text-[13px] text-white placeholder-[#8E96A8] focus:outline-none"
      />
      <button
        type="submit"
        disabled={looking || !query.trim()}
        className="flex items-center gap-1.5 h-9 px-4 rounded-full bg-[#C8875A] hover:bg-[#D9955F] disabled:opacity-40 text-white text-[12px] font-bold transition-colors cursor-pointer"
      >
        <Download className="w-3.5 h-3.5" />
        {looking ? "Looking..." : "Get Skin"}
      </button>
      <label className="flex items-center gap-1.5 h-9 px-4 rounded-full border border-white/15 hover:border-white/30 text-white text-[12px] font-bold cursor-pointer transition-colors">
        <FolderOpen className="w-3.5 h-3.5" />
        Import Skin
        <input type="file" accept="image/png" className="sr-only" onChange={(e) => importSkin(e.target.files?.[0])} />
      </label>
    </form>
  );

  // Carousel of players (skins) or capes, three per page, with arrows and dots.
  const carouselItems: (PlayerTextures | string | null)[] = section === "skin" ? featured : capes;
  const pageCount = Math.max(1, Math.ceil(carouselItems.length / PAGE_SIZE));
  const shown = carouselItems.slice(page * PAGE_SIZE, page * PAGE_SIZE + PAGE_SIZE);
  const go = (delta: number) => setPage((p) => Math.min(pageCount - 1, Math.max(0, p + delta)));

  const shownSkinUrl = chosen?.skin_url ?? skinUrl;
  // Capes are worn on a figure: the chosen skin, or the first carousel player's when none is set.
  const defaultSkin = featured.find((p) => p?.skin_url)?.skin_url ?? null;
  const capeUrl = chosenCapeName ? `/capes/${chosenCapeName}.png` : chosen?.cape_url ?? (cape ? `/capes/${cape}.png` : null);
  const bigPreview = (() => {
    if (section === "cape") {
      const figure = shownSkinUrl ?? defaultSkin;
      return figure ? (
        <SkinModel3D src={figure} capeSrc={capeUrl} className="h-full w-full" />
      ) : capeUrl ? (
        <img src={capeUrl} alt="Cape" className="max-h-[260px] object-contain" />
      ) : (
        <span className="text-[12px] text-[#8E96A8]">No cape</span>
      );
    }
    return shownSkinUrl ? (
      <SkinModel3D src={shownSkinUrl} className="h-full w-full" />
    ) : (
      <img src="/resources/Steve_classic.png" alt="Player" className="h-64 object-contain" />
    );
  })();

  return (
    <div className="px-10 pb-8 pt-28 w-full space-y-6 overflow-y-auto h-full">
      {pills}

      {section === "cursors" && (
        <div className="grid grid-cols-3 gap-4">
          {CURSOR_PACKS.map((pack) => {
            const selected = prefs.cursorPack === pack.id;
            return (
              <button
                key={pack.id}
                type="button"
                onClick={() => onPrefs({ ...prefs, cursorPack: pack.id })}
                className={`${cardBase} text-left space-y-3 cursor-pointer transition-colors ${
                  selected ? "border-[#E86A17] bg-[#E86A17]/10" : "hover:border-white/20"
                }`}
              >
                <div className="h-16 flex items-center gap-4 justify-center">
                  {pack.dir ? (
                    ["normal", "pointer", "text"].map((n) => (
                      <img key={n} src={`${pack.dir}/${n}.png`} alt="" className="w-8 h-8" />
                    ))
                  ) : (
                    <span className="text-[12px] text-[#8E96A8]">Windows / Linux default</span>
                  )}
                </div>
                <div className="text-[13px] font-bold text-white">{pack.label}</div>
              </button>
            );
          })}
        </div>
      )}

      {(section === "skin" || section === "cape") && (
        <div className="flex flex-col items-center gap-6">
          {lookupBar}
          {lookupError && <p className="text-[12px] text-rose-400">{lookupError}</p>}

          <div className="text-center space-y-1">
            <h2 className="text-[26px] font-bold tracking-[0.2em] text-white uppercase">
              {section === "skin" ? "Choose your skin" : "Choose your cape"}
            </h2>
            <p className="text-[13px] text-[#C5CCD8]">
              {section === "skin" ? "Select a skin to customize your appearance." : "Select a cape to equip on your account."}
            </p>
          </div>

          <div className="w-full grid grid-cols-[1fr_minmax(280px,340px)] gap-8 items-center">
            <div className="flex flex-col items-center gap-4 min-w-0">
              <div className="flex items-center gap-3 w-full">
                <button
                  type="button"
                  onClick={() => go(-1)}
                  disabled={page === 0}
                  aria-label="Previous"
                  className="shrink-0 w-10 h-10 rounded-full bg-[#0B0D14]/60 border border-white/15 flex items-center justify-center text-white disabled:opacity-30 cursor-pointer"
                >
                  <ChevronLeft className="w-5 h-5" />
                </button>
                <div className="grid grid-cols-3 gap-3 flex-1 min-w-0">
                  {section === "skin" &&
                    shown.map((item, i) => {
                      const p = item as PlayerTextures | null;
                      const isChosen = p && chosen?.name === p.name;
                      return (
                        <button
                          key={`${page}-${i}`}
                          type="button"
                          disabled={!p?.skin_url}
                          onClick={() => p && (setChosen(p), setChosenCapeName(null))}
                          className={`${cardBase} p-3 flex flex-col items-center gap-2 cursor-pointer transition-colors disabled:cursor-default ${
                            isChosen ? "border-[#E86A17] bg-[#E86A17]/10" : "hover:border-white/25"
                          }`}
                        >
                          <div className="h-44 w-full flex items-center justify-center">
                            {p?.skin_url ? (
                              <SkinModel3D src={p.skin_url} autoRotate={false} className="h-40 w-full" />
                            ) : (
                              <span className="text-[11px] text-[#8E96A8]">Loading...</span>
                            )}
                          </div>
                          <span className="text-[12px] font-semibold text-white">{p?.name ?? ""}</span>
                        </button>
                      );
                    })}
                  {section === "cape" &&
                    shown.map((item) => {
                      const c = item as string;
                      const isChosen = chosenCapeName === c;
                      return (
                        <button
                          key={c}
                          type="button"
                          onClick={() => setChosenCapeName(c)}
                          className={`${cardBase} p-3 flex flex-col items-center gap-2 cursor-pointer transition-colors ${
                            isChosen ? "border-[#E86A17] bg-[#E86A17]/10" : "hover:border-white/25"
                          }`}
                        >
                          <div className="h-44 w-full flex items-center justify-center">
                            {defaultSkin ? (
                              <SkinModel3D src={defaultSkin} capeSrc={`/capes/${c}.png`} autoRotate={false} className="h-40" />
                            ) : (
                              <img src={`/capes/${c}.png`} alt={c} className="h-40 object-contain" />
                            )}
                          </div>
                          <span className="text-[12px] font-semibold text-white capitalize">{c} Cape</span>
                        </button>
                      );
                    })}
                </div>
                <button
                  type="button"
                  onClick={() => go(1)}
                  disabled={page >= pageCount - 1}
                  aria-label="Next"
                  className="shrink-0 w-10 h-10 rounded-full bg-[#0B0D14]/60 border border-white/15 flex items-center justify-center text-white disabled:opacity-30 cursor-pointer"
                >
                  <ChevronRight className="w-5 h-5" />
                </button>
              </div>
              <div className="flex items-center gap-2">
                {Array.from({ length: pageCount }).map((_, i) => (
                  <button
                    key={i}
                    type="button"
                    aria-label={`Page ${i + 1}`}
                    onClick={() => setPage(i)}
                    className={`h-2 rounded-full transition-all cursor-pointer ${i === page ? "w-6 bg-white" : "w-2 bg-white/30"}`}
                  />
                ))}
              </div>
            </div>

            <div className={`${cardBase} h-[360px] flex flex-col items-center justify-between p-4`}>
              <div className="relative flex-1 min-h-0 w-full flex items-center justify-center">{bigPreview}</div>
              <div className="text-center space-y-2">
                <div>
                  <p className="text-[13px] font-bold text-white">
                    {section === "skin" ? chosen?.name ?? username : chosen?.name ?? "Cape"}
                  </p>
                  <p className="text-[11px] text-[#8E96A8]">
                    {section === "skin" ? "Drag to turn" : chosenCapeName ?? (cape ? `Equipped: ${cape}` : "No cape selected")}
                  </p>
                </div>
                {section === "skin" && chosen?.skin_url && chosen.skin_url !== skinUrl && (
                  <button
                    type="button"
                    onClick={() => onUseSkin(chosen.skin_url!)}
                    className="h-9 px-5 rounded-full bg-[#E86A17] hover:bg-[#F97316] text-white text-[12px] font-bold cursor-pointer transition-colors"
                  >
                    Use on account
                  </button>
                )}
              </div>
            </div>
          </div>

          {section === "skin" && (
            <div className="flex flex-wrap justify-center gap-2">
              {skins.map((s) => (
                <button
                  key={s}
                  type="button"
                  onClick={() => onSkin(s)}
                  className={`px-3 py-1.5 rounded-full text-[11px] font-bold transition-all cursor-pointer ${
                    skin === s ? "bg-[#E86A17] text-white" : "bg-[#0B0D14]/60 text-[#8E96A8] hover:text-white border border-[#23293D]"
                  }`}
                >
                  {s}
                </button>
              ))}
            </div>
          )}
          {section === "cape" && (
            <div className="flex justify-center">
              <button
                type="button"
                disabled={!chosenCapeName && !chosen?.cape_url}
                onClick={() => (chosenCapeName ? onCape(chosenCapeName, null) : onCape(null, chosen?.cape_url ?? null))}
                className="h-10 px-6 rounded-full bg-[#E86A17] hover:bg-[#F97316] disabled:opacity-40 text-white text-[13px] font-bold cursor-pointer transition-colors"
              >
                Equip cape
              </button>
            </div>
          )}
        </div>
      )}

      {section === "themes" && (
        <div className="space-y-4">
          <div className="grid grid-cols-3 gap-4">
            {BACKGROUNDS.map((bg) => {
              const selected = prefs.background === bg.id;
              return (
                <button
                  key={bg.id}
                  type="button"
                  onClick={() => onPrefs({ ...prefs, background: bg.id })}
                  className={`${cardBase} p-2 text-left space-y-2 cursor-pointer transition-colors ${
                    selected ? "border-[#E86A17] bg-[#E86A17]/10" : "hover:border-white/20"
                  }`}
                >
                  <img src={bg.src} alt={bg.label} className="w-full h-28 object-cover rounded-xl" />
                  <div className="px-1 pb-1 text-[13px] font-bold text-white">{bg.label}</div>
                </button>
              );
            })}
            <label
              className={`${cardBase} p-2 space-y-2 cursor-pointer transition-colors ${
                prefs.background === "custom" ? "border-[#E86A17] bg-[#E86A17]/10" : "hover:border-white/20"
              }`}
            >
              <div className="w-full h-28 rounded-xl bg-[#0F121C] flex items-center justify-center overflow-hidden">
                {prefs.background === "custom" && prefs.customBackground ? (
                  <img src={prefs.customBackground} alt="Custom background" className="w-full h-full object-cover" />
                ) : (
                  <span className="text-[12px] text-[#8E96A8]">Choose an image</span>
                )}
              </div>
              <div className="px-1 pb-1 text-[13px] font-bold text-white">Custom image</div>
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp"
                className="sr-only"
                onChange={(e) => pickCustomImage(e.target.files?.[0])}
              />
            </label>
          </div>
          {uploadError && <p className="text-[12px] text-rose-400">{uploadError}</p>}
          <p className="text-[12px] text-[#8E96A8]">More backgrounds are coming. Custom images stay on this computer.</p>
        </div>
      )}
    </div>
  );
}
