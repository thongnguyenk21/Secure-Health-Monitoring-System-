import { useEffect, useRef } from "react";
import { buildUrl } from "../api/http";
import type { LiveEvent } from "../api/types";
import { freshAccessToken } from "../auth/session";

const BASE_DELAY_MS = 1_000;
const MAX_DELAY_MS = 30_000;

/**
 * Delay before the next reconnect attempt (exponential backoff, capped).
 *
 * @param attempt - Number of failed attempts so far, from 0.
 * @returns Delay in ms: 1 s, 2 s, 4 s, … up to 30 s.
 */
export function reconnectDelayMs(attempt: number): number {
  return Math.min(MAX_DELAY_MS, BASE_DELAY_MS * 2 ** Math.max(0, attempt));
}

/**
 * Parse one text frame of the live stream.
 *
 * @param data - Frame payload.
 * @returns The event, or null when the frame is not a known event.
 */
export function parseLiveEvent(data: unknown): LiveEvent | null {
  if (typeof data !== "string") return null;
  try {
    const value = JSON.parse(data) as Partial<LiveEvent>;
    if ((value.type === "alert_opened" || value.type === "alert_updated") && value.alert && value.alert.id) {
      return value as LiveEvent;
    }
  } catch {
    return null;
  }
  return null;
}

/**
 * Build the WebSocket URL of the live stream on the current origin.
 *
 * @param token - Access token.
 * @returns A `ws://` or `wss://` URL, so the Vite proxy also carries it in development.
 */
export function liveUrl(token: string): string {
  const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${window.location.host}${buildUrl("/live", { access_token: token })}`;
}

/**
 * Keep a WebSocket to `/api/v1/live` open while the component is mounted.
 *
 * @param onEvent - Called with every alert event.
 * @remarks
 * The server closes the socket when the token expires; each reconnect asks the
 * session for a fresh token and waits with exponential backoff.
 */
export function useLiveAlerts(onEvent: (event: LiveEvent) => void): void {
  const handler = useRef(onEvent);
  useEffect(() => {
    handler.current = onEvent;
  }, [onEvent]);

  useEffect(() => {
    let socket: WebSocket | null = null;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let attempt = 0;
    let stopped = false;

    const retry = () => {
      if (stopped) return;
      timer = setTimeout(connect, reconnectDelayMs(attempt));
      attempt += 1;
    };

    async function connect(): Promise<void> {
      let token: string;
      try {
        token = await freshAccessToken();
      } catch {
        // No session any more: the router is already on its way to the login page.
        return;
      }
      if (stopped) return;
      socket = new WebSocket(liveUrl(token));
      socket.onopen = () => {
        attempt = 0;
      };
      socket.onmessage = (msg: MessageEvent) => {
        const event = parseLiveEvent(msg.data);
        if (event) handler.current(event);
      };
      socket.onclose = retry;
    }

    void connect();
    return () => {
      stopped = true;
      if (timer) clearTimeout(timer);
      if (socket) {
        socket.onclose = null;
        socket.close();
      }
    };
  }, []);
}
