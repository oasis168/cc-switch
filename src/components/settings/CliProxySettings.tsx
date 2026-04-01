import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Loader2, Search, RefreshCw } from 'lucide-react'
import { useToast } from '@/hooks/use-toast'
import {
  getCliProxyConfig,
  setCliProxyConfig,
  applyCliProxy,
  getCurrentCliProxyEnv,
  type CliProxyConfig,
} from '@/lib/api/cliProxy'
import { useScanProxies } from '@/hooks/useGlobalProxy'

export function CliProxySettings() {
  const { t } = useTranslation()
  const { toast } = useToast()
  const [config, setConfig] = useState<CliProxyConfig>({
    mode: 'Disabled',
    independent_url: '',
  })
  const [currentEnv, setCurrentEnv] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const scanProxies = useScanProxies()

  useEffect(() => {
    loadConfig()
    loadCurrentEnv()
  }, [])

  const loadConfig = async () => {
    try {
      const cfg = await getCliProxyConfig()
      setConfig(cfg)
    } catch (error) {
      console.error('Failed to load CLI proxy config:', error)
    }
  }

  const loadCurrentEnv = async () => {
    try {
      const env = await getCurrentCliProxyEnv()
      setCurrentEnv(env)
    } catch (error) {
      console.error('Failed to load current env:', error)
    }
  }

  const handleModeChange = async (mode: string) => {
    const newConfig = { ...config, mode: mode as CliProxyConfig['mode'] }
    setConfig(newConfig)
    try {
      await setCliProxyConfig(newConfig)
    } catch (error) {
      toast({ title: t('error'), description: String(error), variant: 'destructive' })
    }
  }

  const handleUrlChange = (url: string) => {
    setConfig({ ...config, independent_url: url })
  }

  const handleApply = async () => {
    setLoading(true)
    try {
      await setCliProxyConfig(config)
      await applyCliProxy()
      await loadCurrentEnv()
      toast({ title: t('success'), description: t('cliProxy.applied') })
    } catch (error) {
      toast({ title: t('error'), description: String(error), variant: 'destructive' })
    } finally {
      setLoading(false)
    }
  }

  const handleScan = async () => {
    try {
      const proxies = await scanProxies.mutateAsync()
      if (proxies.length > 0) {
        setConfig({ ...config, independent_url: proxies[0].url })
      }
    } catch (error) {
      toast({ title: t('error'), description: String(error), variant: 'destructive' })
    }
  }

  return (
    <div className="space-y-4">
      <div>
        <Label>{t('cliProxy.mode')}</Label>
        <RadioGroup value={config.mode} onValueChange={handleModeChange} className="mt-2">
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="Disabled" id="disabled" />
            <Label htmlFor="disabled">{t('cliProxy.modeDisabled')}</Label>
          </div>
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="SyncOutbound" id="sync" />
            <Label htmlFor="sync">{t('cliProxy.modeSyncOutbound')}</Label>
          </div>
          <div className="flex items-center space-x-2">
            <RadioGroupItem value="Independent" id="independent" />
            <Label htmlFor="independent">{t('cliProxy.modeIndependent')}</Label>
          </div>
        </RadioGroup>
      </div>

      {config.mode === 'Independent' && (
        <div className="space-y-2">
          <Label>{t('cliProxy.independentUrl')}</Label>
          <div className="flex gap-2">
            <Input
              value={config.independent_url || ''}
              onChange={(e) => handleUrlChange(e.target.value)}
              placeholder="http://127.0.0.1:7890"
            />
            <Button onClick={handleScan} disabled={scanProxies.isPending} size="sm" variant="outline">
              {scanProxies.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Search className="h-4 w-4" />}
            </Button>
          </div>
          <Button onClick={handleApply} disabled={loading}>
            {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
            {t('cliProxy.apply')}
          </Button>
        </div>
      )}

      <div className="text-sm text-muted-foreground">
        <div>{t('cliProxy.currentEnv')}: {currentEnv || t('cliProxy.notSet')}</div>
      </div>
    </div>
  )
}
