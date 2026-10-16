import * as SecureStore from "expo-secure-store";
import { Platform } from "react-native";

/**
 * Persistent storage for the serialised session.
 *
 * @remarks
 * Async so the native store (Android Keystore through expo-secure-store) and
 * the web fallback share one interface.
 */
export interface TokenStore {
  /** Read the stored value, or null when nothing is stored. */
  load(): Promise<string | null>;
  /** Replace the stored value. */
  save(value: string): Promise<void>;
  /** Remove the stored value. */
  clear(): Promise<void>;
}

/** Storage key of the session; SecureStore allows only `[A-Za-z0-9._-]`. */
export const SESSION_KEY = "iomt.session";

/**
 * Create the native store backed by expo-secure-store.
 *
 * @returns A store that keeps the session encrypted with the Android Keystore.
 */
export function createSecureTokenStore(): TokenStore {
  return {
    load: () => SecureStore.getItemAsync(SESSION_KEY),
    save: (value) => SecureStore.setItemAsync(SESSION_KEY, value),
    clear: () => SecureStore.deleteItemAsync(SESSION_KEY),
  };
}

/**
 * Create the web store: memory plus `sessionStorage`.
 *
 * @param storage - Browser storage, or null when unavailable (private mode, tests).
 * @returns A store that survives a page reload but not closing the tab.
 * @remarks
 * Browsers have no secure enclave; `sessionStorage` limits the token to one
 * tab and dies with it, the same trade-off as the clinician web app.
 */
export function createWebTokenStore(storage: Pick<Storage, "getItem" | "setItem" | "removeItem"> | null): TokenStore {
  let memory: string | null = null;
  return {
    load: async () => {
      try {
        return storage?.getItem(SESSION_KEY) ?? memory;
      } catch {
        return memory;
      }
    },
    save: async (value) => {
      memory = value;
      try {
        storage?.setItem(SESSION_KEY, value);
      } catch {
        // Storage can be disabled; the in-memory copy still works for this tab.
      }
    },
    clear: async () => {
      memory = null;
      try {
        storage?.removeItem(SESSION_KEY);
      } catch {
        // Nothing to remove when storage is disabled.
      }
    },
  };
}

function browserSessionStorage(): Storage | null {
  try {
    return typeof sessionStorage === "undefined" ? null : sessionStorage;
  } catch {
    return null;
  }
}

/**
 * Pick the token store for the running platform.
 *
 * @returns The secure store on Android, the web store in a browser.
 */
export function createPlatformTokenStore(): TokenStore {
  return Platform.OS === "web" ? createWebTokenStore(browserSessionStorage()) : createSecureTokenStore();
}
