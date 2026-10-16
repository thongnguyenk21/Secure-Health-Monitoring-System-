import { QueryClientProvider } from "@tanstack/react-query";
import { useFonts } from "expo-font";
import { Stack } from "expo-router";
import * as SplashScreen from "expo-splash-screen";
import { StatusBar } from "expo-status-bar";
import { useEffect, useState } from "react";
import { SafeAreaProvider } from "react-native-safe-area-context";
import { NoticeProvider } from "../components/Notice";
import { PhoneFrame } from "../components/PhoneFrame";
import { restoreSession } from "../features/auth/session";
import { useSessionState } from "../state/patient";
import { createQueryClient } from "../state/queryClient";
import { colors } from "../theme/colors";
import { fontAssets } from "../theme/fonts";

SplashScreen.preventAutoHideAsync().catch(() => undefined);

/**
 * Root layout: providers, fonts, session restore and the auth guard.
 *
 * @returns The navigation tree.
 * @remarks
 * `Stack.Protected` keeps the tabs and the alert screen out of reach until a
 * session exists, and the login screen out of reach once it does.
 */
export default function RootLayout() {
  const [client] = useState(createQueryClient);
  const [fontsLoaded, fontError] = useFonts(fontAssets);
  const { restoring, session } = useSessionState();

  useEffect(() => {
    restoreSession().catch(() => undefined);
  }, []);

  const ready = (fontsLoaded || fontError !== null) && !restoring;
  useEffect(() => {
    if (ready) SplashScreen.hideAsync().catch(() => undefined);
  }, [ready]);

  if (!ready) return null;
  const signedIn = session !== null;

  return (
    <SafeAreaProvider>
      <QueryClientProvider client={client}>
        <PhoneFrame>
          <NoticeProvider>
            <StatusBar style="dark" />
            <Stack screenOptions={{ headerShown: false, contentStyle: { backgroundColor: colors.bg } }}>
              <Stack.Protected guard={signedIn}>
                <Stack.Screen name="(tabs)" />
                <Stack.Screen name="emergency/[alertId]" options={{ presentation: "fullScreenModal", animation: "fade" }} />
              </Stack.Protected>
              <Stack.Protected guard={!signedIn}>
                <Stack.Screen name="login" />
              </Stack.Protected>
            </Stack>
          </NoticeProvider>
        </PhoneFrame>
      </QueryClientProvider>
    </SafeAreaProvider>
  );
}
