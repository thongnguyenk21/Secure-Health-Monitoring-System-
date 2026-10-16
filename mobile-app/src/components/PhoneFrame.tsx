import type { ReactNode } from "react";
import { Platform, StyleSheet, View } from "react-native";
import { colors } from "../theme/colors";

/** Widest column the app uses in a browser, in px; a large phone is about 430. */
export const PHONE_MAX_WIDTH = 430;

/**
 * Centre the app in a phone-width column when it runs in a browser.
 *
 * @param props - The app tree.
 * @returns The tree unchanged on Android, framed on web.
 * @remarks
 * The owner reviews the UI in a desktop browser first; the column keeps the
 * phone proportions of the wireframe instead of stretching across the screen.
 */
export function PhoneFrame({ children }: { children: ReactNode }) {
  if (Platform.OS !== "web") return <>{children}</>;
  return (
    <View style={styles.backdrop}>
      <View style={styles.column}>{children}</View>
    </View>
  );
}

const styles = StyleSheet.create({
  backdrop: { flex: 1, backgroundColor: colors.backdrop, alignItems: "center" },
  column: {
    flex: 1,
    width: "100%",
    maxWidth: PHONE_MAX_WIDTH,
    backgroundColor: colors.bg,
    overflow: "hidden",
    shadowColor: colors.ink,
    shadowOpacity: 0.12,
    shadowRadius: 24,
    shadowOffset: { width: 0, height: 8 },
  },
});
