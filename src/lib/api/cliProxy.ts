import { invoke } from '@tauri-apps/api/core'

export interface CliProxyConfig {
  mode: 'Disabled' | 'SyncOutbound' | 'Independent'
  independent_url?: string
}

export const getCliProxyConfig = () => invoke<CliProxyConfig>('get_cli_proxy_config')

export const setCliProxyConfig = (config: CliProxyConfig) =>
  invoke('set_cli_proxy_config', { config })

export const applyCliProxy = () => invoke('apply_cli_proxy')

export const getCurrentCliProxyEnv = () => invoke<string | null>('get_current_cli_proxy_env')

export interface LocalGitProxyResult {
  scanned: number
  cleared: number
  details: string[]
}

export const scanAndClearLocalGitProxy = (folder: string) =>
  invoke<LocalGitProxyResult>('scan_and_clear_local_git_proxy', { folder })
