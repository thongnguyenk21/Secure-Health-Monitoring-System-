import type { ReactNode } from "react";
import {
  ActivityIndicator,
  Pressable,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  View,
  type StyleProp,
  type TextStyle,
  type ViewStyle,
} from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import Svg, { Path, Rect } from "react-native-svg";
import { vi } from "../i18n/vi";
import { colors } from "../theme/colors";
import { fonts } from "../theme/fonts";

/** Props of {@link Screen}. */
export interface ScreenProps {
  /** Screen content below the app bar. */
  children: ReactNode;
  /** App bar title. */
  title: string;
  /** Element on the right of the app bar (status chip, sub-title). */
  right?: ReactNode;
  /** Pull-to-refresh handler; omit to disable. */
  onRefresh?: () => void;
  /** Whether a pull-to-refresh is running. */
  refreshing?: boolean;
}

/**
 * Scrollable tab screen with the wireframe app bar.
 *
 * @param props - See {@link ScreenProps}.
 * @returns The screen.
 */
export function Screen({ children, title, right, onRefresh, refreshing = false }: ScreenProps) {
  return (
    <SafeAreaView edges={["top"]} style={styles.safe}>
      <View style={styles.appbar}>
        <Text style={styles.appbarTitle} numberOfLines={1} accessibilityRole="header">
          {title}
        </Text>
        {right}
      </View>
      <ScrollView
        contentContainerStyle={styles.content}
        refreshControl={onRefresh ? <RefreshControl refreshing={refreshing} onRefresh={onRefresh} /> : undefined}
      >
        {children}
      </ScrollView>
    </SafeAreaView>
  );
}

/** Props of {@link Tile}. */
export interface TileProps {
  /** Tile content. */
  children: ReactNode;
  /** Extra styles (border colour, background, padding). */
  style?: StyleProp<ViewStyle>;
}

/**
 * White rounded card with a thin border, the wireframe `.tile`.
 *
 * @param props - See {@link TileProps}.
 * @returns The tile.
 */
export function Tile({ children, style }: TileProps) {
  return <View style={[styles.tile, style]}>{children}</View>;
}

/**
 * Small upper-case caption in monospace, the wireframe `.cap`.
 *
 * @param props - Caption text and optional style.
 * @returns The caption.
 */
export function Caption({ children, style }: { children: string; style?: StyleProp<TextStyle> }) {
  return <Text style={[styles.caption, style]}>{children.toUpperCase()}</Text>;
}

/**
 * Coloured status dot followed by a label.
 *
 * @param props - Dot colour, label and label colour.
 * @returns The chip.
 */
export function StatusChip({ color, label, labelColor }: { color: string; label: string; labelColor?: string }) {
  return (
    <View style={styles.chip}>
      <View style={[styles.dot, { backgroundColor: color }]} />
      <Text style={[styles.chipText, { color: labelColor ?? colors.ink }]}>{label}</Text>
    </View>
  );
}

/** Props of {@link Button}. */
export interface ButtonProps {
  /** Button label. */
  label: string;
  /** Press handler. */
  onPress: () => void;
  /** Visual style; primary is filled with the accent colour. */
  variant?: "primary" | "secondary";
  /** Disable the button (e.g. while a request runs). */
  disabled?: boolean;
  /** Test id for screen tests. */
  testID?: string;
}

/**
 * Full-width button with a 48 dp touch target.
 *
 * @param props - See {@link ButtonProps}.
 * @returns The button.
 */
export function Button({ label, onPress, variant = "primary", disabled = false, testID }: ButtonProps) {
  const primary = variant === "primary";
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityState={{ disabled }}
      disabled={disabled}
      onPress={onPress}
      testID={testID}
      style={({ pressed }) => [
        styles.button,
        primary ? styles.buttonPrimary : styles.buttonSecondary,
        (pressed || disabled) && styles.buttonPressed,
      ]}
    >
      <Text style={[styles.buttonText, { color: primary ? colors.paper : colors.accent }]}>{label}</Text>
    </Pressable>
  );
}

/**
 * Centred spinner with a loading label.
 *
 * @returns The loading block.
 */
export function Loading() {
  return (
    <View style={styles.center}>
      <ActivityIndicator color={colors.accent} />
      <Text style={styles.muted}>{vi.common.loading}</Text>
    </View>
  );
}

/**
 * Inline error note with an optional retry button.
 *
 * @param props - Message and retry handler.
 * @returns The note.
 */
export function ErrorNote({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <Tile style={styles.errorTile}>
      <Text style={styles.body}>{message}</Text>
      {onRetry ? (
        <Pressable accessibilityRole="button" onPress={onRetry} hitSlop={8}>
          <Text style={styles.link}>{vi.common.retry}</Text>
        </Pressable>
      ) : null}
    </Tile>
  );
}

/**
 * Friendly card shown when the patient has no paired watch.
 *
 * @returns The card.
 */
export function NoDeviceCard() {
  return (
    <Tile style={styles.noDevice}>
      <View style={styles.noDeviceIcon} aria-hidden>
        <Svg width={30} height={30} viewBox="0 0 24 24">
          <Rect x={8} y={1.5} width={8} height={4} rx={1} fill={colors.accent} />
          <Rect x={8} y={18.5} width={8} height={4} rx={1} fill={colors.accent} />
          <Rect x={5.5} y={5} width={13} height={14} rx={3.5} fill="none" stroke={colors.accent} strokeWidth={2} />
          <Path d="M12 8.5v3.5l2.2 1.6" fill="none" stroke={colors.accent} strokeWidth={1.8} strokeLinecap="round" />
        </Svg>
      </View>
      <Text style={styles.noDeviceTitle}>{vi.noDevice.title}</Text>
      <Text style={styles.noDeviceBody}>{vi.noDevice.body}</Text>
    </Tile>
  );
}

/** Shared text styles for screens. */
export const text = StyleSheet.create({
  body: { fontFamily: fonts.body, fontSize: 16, color: colors.ink },
  bodySemi: { fontFamily: fonts.bodySemi, fontSize: 16, color: colors.ink },
  muted: { fontFamily: fonts.body, fontSize: 14.5, color: colors.muted },
  title: { fontFamily: fonts.display, fontSize: 18, color: colors.ink },
  num: { fontFamily: fonts.display, color: colors.ink, fontVariant: ["tabular-nums"] },
});

const styles = StyleSheet.create({
  safe: { flex: 1, backgroundColor: colors.bg },
  appbar: {
    paddingHorizontal: 20,
    paddingTop: 14,
    paddingBottom: 10,
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "center",
    gap: 12,
  },
  appbarTitle: { fontFamily: fonts.display, fontSize: 22, color: colors.ink, flexShrink: 1 },
  content: { paddingHorizontal: 16, paddingBottom: 24, gap: 10 },
  tile: {
    backgroundColor: colors.paper,
    borderWidth: 1,
    borderColor: colors.line,
    borderRadius: 16,
    paddingVertical: 12,
    paddingHorizontal: 14,
  },
  caption: { fontFamily: fonts.mono, fontSize: 12.5, letterSpacing: 0.6, color: colors.muted },
  chip: { flexDirection: "row", alignItems: "center", gap: 6 },
  dot: { width: 10, height: 10, borderRadius: 5 },
  chipText: { fontFamily: fonts.bodySemi, fontSize: 14.5 },
  button: { minHeight: 48, borderRadius: 14, alignItems: "center", justifyContent: "center", paddingHorizontal: 16 },
  buttonPrimary: { backgroundColor: colors.accent },
  buttonSecondary: { backgroundColor: colors.paper, borderWidth: 1, borderColor: colors.line },
  buttonPressed: { opacity: 0.7 },
  buttonText: { fontFamily: fonts.bodyBold, fontSize: 17 },
  center: { alignItems: "center", justifyContent: "center", paddingVertical: 40, gap: 10 },
  muted: { fontFamily: fonts.body, fontSize: 15, color: colors.muted },
  body: { fontFamily: fonts.body, fontSize: 16, color: colors.ink },
  link: { fontFamily: fonts.bodyBold, fontSize: 16, color: colors.accent, marginTop: 8 },
  errorTile: { borderLeftWidth: 4, borderLeftColor: colors.warn },
  noDevice: { alignItems: "center", paddingVertical: 28, paddingHorizontal: 20, gap: 8 },
  noDeviceIcon: {
    width: 56,
    height: 56,
    borderRadius: 28,
    backgroundColor: colors.accentBg,
    alignItems: "center",
    justifyContent: "center",
    marginBottom: 4,
  },
  noDeviceTitle: { fontFamily: fonts.display, fontSize: 18, color: colors.ink, textAlign: "center" },
  noDeviceBody: { fontFamily: fonts.body, fontSize: 15.5, color: colors.muted, textAlign: "center", lineHeight: 22 },
});
