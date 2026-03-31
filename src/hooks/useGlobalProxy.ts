/**
 * 全局出站代理 React Hooks
 *
 * 提供获取、设置和测试全局代理的 React Query hooks。
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import {
  getGlobalProxyUrl,
  setGlobalProxyUrl,
  testProxyUrl,
  getUpstreamProxyStatus,
  scanLocalProxies,
  getEffectiveProxyStatus,
  syncSystemProxy,
  type ProxyTestResult,
  type UpstreamProxyStatus,
  type DetectedProxy,
  type EffectiveProxyStatus,
  type SyncProxyResult,
} from "@/lib/api/globalProxy";

/**
 * 获取全局代理 URL
 */
export function useGlobalProxyUrl() {
  return useQuery({
    queryKey: ["globalProxyUrl"],
    queryFn: getGlobalProxyUrl,
    staleTime: 30 * 1000, // 30秒内不重新获取，避免展开时闪烁
  });
}

/**
 * 设置全局代理 URL
 */
export function useSetGlobalProxyUrl() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: setGlobalProxyUrl,
    onSuccess: () => {
      toast.success(t("settings.globalProxy.saved"));
      queryClient.invalidateQueries({ queryKey: ["globalProxyUrl"] });
      queryClient.invalidateQueries({ queryKey: ["upstreamProxyStatus"] });
    },
    onError: (error: unknown) => {
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Unknown error";
      toast.error(t("settings.globalProxy.saveFailed", { error: message }));
    },
  });
}

/**
 * 测试代理连接
 */
export function useTestProxy() {
  const { t } = useTranslation();

  return useMutation({
    mutationFn: testProxyUrl,
    onSuccess: (result: ProxyTestResult) => {
      if (result.success) {
        toast.success(
          t("settings.globalProxy.testSuccess", { latency: result.latencyMs }),
        );
      } else {
        toast.error(
          t("settings.globalProxy.testFailed", { error: result.error }),
        );
      }
    },
    onError: (error: Error) => {
      toast.error(error.message);
    },
  });
}

/**
 * 获取当前出站代理状态
 */
export function useUpstreamProxyStatus() {
  return useQuery<UpstreamProxyStatus>({
    queryKey: ["upstreamProxyStatus"],
    queryFn: getUpstreamProxyStatus,
  });
}

/**
 * 扫描本地代理
 */
export function useScanProxies() {
  const { t } = useTranslation();

  return useMutation({
    mutationFn: scanLocalProxies,
    onError: (error: Error) => {
      toast.error(
        t("settings.globalProxy.scanFailed", { error: error.message }),
      );
    },
  });
}

export type { DetectedProxy };

/**
 * 获取当前实际生效的代理状态（含连通性测试）
 */
export function useEffectiveProxyStatus() {
  return useQuery<EffectiveProxyStatus>({
    queryKey: ["effectiveProxyStatus"],
    queryFn: getEffectiveProxyStatus,
    staleTime: 0, // 每次都重新获取
  });
}

/**
 * 同步系统代理（智能检测：有代理就用，没有就直连）
 */
export function useSyncSystemProxy() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: syncSystemProxy,
    onSuccess: (result: SyncProxyResult) => {
      if (result.action === "proxy") {
        toast.success(
          t("settings.globalProxy.syncApplied", { proxy: result.proxyUrl }),
        );
      } else {
        toast.success(t("settings.globalProxy.syncDirect"));
      }
      queryClient.invalidateQueries({ queryKey: ["globalProxyUrl"] });
      queryClient.invalidateQueries({ queryKey: ["upstreamProxyStatus"] });
      queryClient.invalidateQueries({ queryKey: ["effectiveProxyStatus"] });
    },
    onError: (error: unknown) => {
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Unknown error";
      toast.error(t("settings.globalProxy.syncFailed", { error: message }));
    },
  });
}
