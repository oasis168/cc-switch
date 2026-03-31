/**
 * 全局出站代理 API
 *
 * 提供获取、设置和测试全局代理的功能。
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * 代理测试结果
 */
export interface ProxyTestResult {
  success: boolean;
  latencyMs: number;
  error: string | null;
}

/**
 * 出站代理状态
 */
export interface UpstreamProxyStatus {
  enabled: boolean;
  proxyUrl: string | null;
}

/**
 * 检测到的代理
 */
export interface DetectedProxy {
  url: string;
  proxyType: string;
  port: number;
}

/**
 * 获取全局代理 URL
 *
 * @returns 代理 URL，null 表示未配置（直连）
 */
export async function getGlobalProxyUrl(): Promise<string | null> {
  return invoke<string | null>("get_global_proxy_url");
}

/**
 * 设置全局代理 URL
 *
 * @param url - 代理 URL（如 http://127.0.0.1:7890 或 socks5://127.0.0.1:1080）
 *              空字符串表示清除代理（直连）
 */
export async function setGlobalProxyUrl(url: string): Promise<void> {
  try {
    return await invoke("set_global_proxy_url", { url });
  } catch (error) {
    // Tauri invoke 错误可能是字符串
    throw new Error(typeof error === "string" ? error : String(error));
  }
}

/**
 * 测试代理连接
 *
 * @param url - 要测试的代理 URL
 * @returns 测试结果，包含是否成功、延迟和错误信息
 */
export async function testProxyUrl(url: string): Promise<ProxyTestResult> {
  return invoke<ProxyTestResult>("test_proxy_url", { url });
}

/**
 * 获取当前出站代理状态
 *
 * @returns 代理状态，包含是否启用和代理 URL
 */
export async function getUpstreamProxyStatus(): Promise<UpstreamProxyStatus> {
  return invoke<UpstreamProxyStatus>("get_upstream_proxy_status");
}

/**
 * 扫描本地代理
 *
 * @returns 检测到的代理列表
 */
export async function scanLocalProxies(): Promise<DetectedProxy[]> {
  return invoke<DetectedProxy[]>("scan_local_proxies");
}

/**
 * 实际生效的代理诊断信息
 */
export interface EffectiveProxyStatus {
  /** 代理来源："explicit"（用户配置）/ "system"（系统自动检测）/ "direct"（直连） */
  source: string;
  /** 显式配置的代理 URL（脱敏） */
  explicitProxy: string | null;
  /** 连通性测试结果 */
  reachable: boolean;
  /** 延迟（毫秒） */
  latencyMs: number;
  /** 错误信息 */
  error: string | null;
}

/**
 * 同步代理结果
 */
export interface SyncProxyResult {
  /** 执行的动作："proxy"（应用了代理）/ "direct"（切换为直连） */
  action: string;
  /** 应用的代理 URL（脱敏） */
  proxyUrl: string | null;
  /** 附加信息 */
  message: string | null;
}

/**
 * 获取当前实际生效的代理状态（含连通性测试）
 */
export async function getEffectiveProxyStatus(): Promise<EffectiveProxyStatus> {
  return invoke<EffectiveProxyStatus>("get_effective_proxy_status");
}

/**
 * 同步系统代理（智能检测：有代理就用，没有就直连）
 */
export async function syncSystemProxy(): Promise<SyncProxyResult> {
  return invoke<SyncProxyResult>("sync_system_proxy");
}
