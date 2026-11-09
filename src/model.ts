export interface Instance {
  id: string;
  name: string;
  edition: "rust" | "java";
  game_version: string;
  loader: string;
  game_dir: string;
  banner: string;
  icon: string;
  memory_min_mb: number;
  memory_max_mb: number;
  java_args: string;
  created_at: string;
  last_played: string | null;
  loader_version?: string | null;
}

export type Tab = "home" | "instances" | "mods" | "customize" | "settings" | "servers";

/** The edition name shown on play buttons and instance tiles. */
export function editionLabel(edition: Instance["edition"]): string {
  return edition === "rust" ? "Rust Edition" : "Java Edition";
}

export interface Account {
  id: string;
  username: string;
  uuid: string;
  is_offline: boolean;
  skin_preset: string | null;
  cape_preset: string | null;
  skin_url?: string | null;
  cape_data?: string | null;
  is_active: boolean;
}
