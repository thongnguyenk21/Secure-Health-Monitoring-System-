import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { colors } from "../theme/colors";
import { fonts } from "../theme/fonts";

/** Tone of a notice banner. */
export type NoticeTone = "ok" | "warn";

interface NoticeState {
  message: string;
  tone: NoticeTone;
  id: number;
}

type ShowNotice = (message: string, tone?: NoticeTone) => void;

const NoticeContext = createContext<ShowNotice>(() => undefined);

/** How long a notice stays on screen, in ms. */
export const NOTICE_MS = 6_000;

/**
 * Show short confirmation banners at the top of the app.
 *
 * @param props - The app tree.
 * @returns The provider with its banner.
 * @remarks
 * Used for confirmations that must stay visible after a screen closes,
 * e.g. "Đã báo cho bác sĩ" when the emergency screen times out.
 */
export function NoticeProvider({ children }: { children: ReactNode }) {
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const counter = useRef(0);
  const insets = useSafeAreaInsets();
  const show = useCallback<ShowNotice>((message, tone = "ok") => {
    counter.current += 1;
    setNotice({ message, tone, id: counter.current });
  }, []);
  useEffect(() => {
    if (!notice) return undefined;
    const timer = setTimeout(() => setNotice(null), NOTICE_MS);
    return () => clearTimeout(timer);
  }, [notice]);
  const value = useMemo(() => show, [show]);
  return (
    <NoticeContext.Provider value={value}>
      {children}
      {notice ? (
        <View pointerEvents="box-none" style={[styles.wrap, { top: insets.top + 8 }]}>
          <Pressable
            accessibilityRole="alert"
            accessibilityLiveRegion="polite"
            onPress={() => setNotice(null)}
            style={[styles.banner, { backgroundColor: notice.tone === "ok" ? colors.ok : colors.warn }]}
          >
            <Text style={styles.text}>{notice.message}</Text>
          </Pressable>
        </View>
      ) : null}
    </NoticeContext.Provider>
  );
}

/**
 * Get the function that shows a notice banner.
 *
 * @returns `show(message, tone)`.
 */
export function useNotice(): ShowNotice {
  return useContext(NoticeContext);
}

const styles = StyleSheet.create({
  wrap: { position: "absolute", left: 12, right: 12, alignItems: "center", zIndex: 10 },
  banner: {
    borderRadius: 14,
    paddingVertical: 12,
    paddingHorizontal: 16,
    width: "100%",
    shadowColor: colors.ink,
    shadowOpacity: 0.2,
    shadowRadius: 10,
    shadowOffset: { width: 0, height: 4 },
    elevation: 4,
  },
  text: { fontFamily: fonts.bodySemi, fontSize: 16, color: colors.paper, lineHeight: 22 },
});
