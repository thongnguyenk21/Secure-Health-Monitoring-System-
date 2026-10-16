import { useSyncExternalStore } from "react";
import { getSession, subscribeSession, type Session } from "./session";

/**
 * Read the current session and re-render when it changes.
 *
 * @returns The session, or null when logged out.
 */
export function useSession(): Session | null {
  return useSyncExternalStore(subscribeSession, getSession, getSession);
}
