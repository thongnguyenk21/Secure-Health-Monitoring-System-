import type { BottomTabBarProps } from "expo-router/tabs";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { colors } from "../theme/colors";
import { fonts } from "../theme/fonts";

/** Extra props of {@link TabBar}. */
export interface TabBarExtraProps {
  /** Labels per route name. */
  labels: Record<string, string>;
  /** Route names that show a red dot (e.g. open alerts). */
  dotted: ReadonlySet<string>;
}

/**
 * Text-only bottom tab bar of the wireframe: white bar, accent colour on the active tab.
 *
 * @param props - Navigator props plus labels and dots.
 * @returns The tab bar.
 */
export function TabBar({ state, emitter, navigateToTab, labels, dotted }: BottomTabBarProps & TabBarExtraProps) {
  const insets = useSafeAreaInsets();
  return (
    <View style={[styles.bar, { paddingBottom: insets.bottom }]} accessibilityRole="tablist">
      {state.routes.map((route, index) => {
        const focused = state.index === index;
        const label = labels[route.name] ?? route.name;
        const onPress = () => {
          const event = emitter.emit({ type: "tabPress", target: route.key, canPreventDefault: true });
          if (!focused && !event.defaultPrevented) navigateToTab(route.key);
        };
        return (
          <Pressable
            key={route.key}
            accessibilityRole="tab"
            accessibilityState={{ selected: focused }}
            accessibilityLabel={label}
            onPress={onPress}
            style={styles.tab}
            testID={`tab-${route.name}`}
          >
            <View style={[styles.indicator, focused && styles.indicatorOn]} />
            <View style={styles.labelRow}>
              <Text style={[styles.label, focused && styles.labelOn]}>{label}</Text>
              {dotted.has(route.name) ? <View style={styles.dot} /> : null}
            </View>
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  bar: { flexDirection: "row", backgroundColor: colors.paper, borderTopWidth: 1, borderTopColor: colors.line },
  tab: { flex: 1, minHeight: 60, alignItems: "center", justifyContent: "center" },
  indicator: { position: "absolute", top: 0, width: 36, height: 3, borderRadius: 2, backgroundColor: "transparent" },
  indicatorOn: { backgroundColor: colors.accent },
  labelRow: { flexDirection: "row", alignItems: "flex-start" },
  label: { fontFamily: fonts.bodySemi, fontSize: 15, color: colors.muted },
  labelOn: { color: colors.accent, fontFamily: fonts.bodyBold },
  dot: { width: 8, height: 8, borderRadius: 4, backgroundColor: colors.alert, marginLeft: 3 },
});
