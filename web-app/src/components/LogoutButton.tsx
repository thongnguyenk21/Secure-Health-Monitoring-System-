import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../api/client";
import { vi } from "../i18n/vi";

/** Button that revokes the session, clears cached data and returns to login. */
export function LogoutButton() {
  const qc = useQueryClient();
  const [busy, setBusy] = useState(false);
  const onClick = async () => {
    setBusy(true);
    await api.logout();
    qc.clear();
  };
  return (
    <button type="button" className="btn-secondary" onClick={() => void onClick()} disabled={busy}>
      {vi.common.logout}
    </button>
  );
}
