import { useState } from "react";
import { Check, Plus } from "lucide-react";
import type { Account } from "../model";

interface AccountSwitcherProps {
  accounts: Account[];
  onSwitch: (id: string) => void;
  onAdd: (username: string) => Promise<void>;
}

/** The profile menu: pick the active account, or add an offline one. */
export default function AccountSwitcher({ accounts, onSwitch, onAdd }: AccountSwitcherProps) {
  const [name, setName] = useState("");

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const username = name.trim();
    if (!username) return;
    setName("");
    await onAdd(username);
  };

  return (
    <div role="menu" className="glass w-60 rounded-2xl p-2 flex flex-col gap-0.5">
      {accounts.map((a) => (
        <button
          key={a.id}
          type="button"
          role="menuitemradio"
          aria-checked={a.is_active}
          onClick={() => onSwitch(a.id)}
          className="flex items-center gap-3 w-full px-2.5 py-2 rounded-xl text-left hover:bg-white/10 transition-colors duration-200"
        >
          <span className="w-6 h-6 rounded-full overflow-hidden bg-[#22293F] shrink-0">
            <img src="/resources/Steve_classic.png" alt="" className="w-full h-full object-cover object-top scale-125" />
          </span>
          <span className="flex-1 min-w-0 truncate text-[13px] text-white">{a.username}</span>
          {a.is_active && <Check className="w-3.5 h-3.5 text-white/80 shrink-0" />}
        </button>
      ))}

      <form onSubmit={submit} className="flex items-center gap-2 mt-1 pt-2 px-1 border-t border-white/10">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Add offline account"
          maxLength={16}
          className="flex-1 min-w-0 bg-transparent text-[13px] text-white placeholder-white/40 outline-none px-1 py-1 select-text"
        />
        <button
          type="submit"
          aria-label="Add account"
          className="w-7 h-7 rounded-full flex items-center justify-center text-white/70 hover:bg-white/10 transition-colors duration-200"
        >
          <Plus className="w-3.5 h-3.5" />
        </button>
      </form>
    </div>
  );
}
