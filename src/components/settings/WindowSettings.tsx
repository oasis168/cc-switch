import { useTranslation } from "react-i18next";
import type { SettingsFormState } from "@/hooks/useSettings";
import { AppWindow, MonitorUp, Power, EyeOff, Search, CheckCircle2, AlertCircle, FolderOpen } from "lucide-react";
import { ToggleRow } from "@/components/ui/toggle-row";
import { AnimatePresence, motion } from "framer-motion";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { useEffect, useState } from "react";
import { settingsApi } from "@/lib/api";

interface WindowSettingsProps {
  settings: SettingsFormState;
  onChange: (updates: Partial<SettingsFormState>) => void;
}

export function WindowSettings({ settings, onChange }: WindowSettingsProps) {
  const { t } = useTranslation();
  const [detectedPath, setDetectedPath] = useState<string | null>(null);
  const [pathChecked, setPathChecked] = useState(false);

  // 开启插件联动 或 切换 ideType 时，重新探测路径（延迟 300ms 等后端保存完成）
  useEffect(() => {
    if (!settings.enableClaudePluginIntegration) {
      setDetectedPath(null);
      setPathChecked(false);
      return;
    }
    let cancelled = false;
    const timer = setTimeout(() => {
      settingsApi.getVscodeSettingsPath().then((p) => {
        if (cancelled) return;
        setDetectedPath(p);
        setPathChecked(true);
      }).catch(() => {
        if (cancelled) return;
        setDetectedPath(null);
        setPathChecked(true);
      });
    }, 300);
    return () => { cancelled = true; clearTimeout(timer); };
  }, [settings.enableClaudePluginIntegration, settings.ideType, settings.vscodeSettingsPath]);

  return (
    <section className="space-y-4">
      <div className="flex items-center gap-2 pb-2 border-b border-border/40">
        <AppWindow className="h-4 w-4 text-primary" />
        <h3 className="text-sm font-medium">{t("settings.windowBehavior")}</h3>
      </div>

      <div className="space-y-3">
        <ToggleRow
          icon={<Power className="h-4 w-4 text-orange-500" />}
          title={t("settings.launchOnStartup")}
          description={t("settings.launchOnStartupDescription")}
          checked={!!settings.launchOnStartup}
          onCheckedChange={(value) => onChange({ launchOnStartup: value })}
        />

        <AnimatePresence initial={false}>
          {settings.launchOnStartup && (
            <motion.div
              key="silent-startup"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.3 }}
            >
              <ToggleRow
                icon={<EyeOff className="h-4 w-4 text-green-500" />}
                title={t("settings.silentStartup")}
                description={t("settings.silentStartupDescription")}
                checked={!!settings.silentStartup}
                onCheckedChange={(value) => onChange({ silentStartup: value })}
              />
            </motion.div>
          )}
        </AnimatePresence>

        <ToggleRow
          icon={<MonitorUp className="h-4 w-4 text-purple-500" />}
          title={t("settings.enableClaudePluginIntegration")}
          description={t("settings.enableClaudePluginIntegrationDescription")}
          checked={!!settings.enableClaudePluginIntegration}
          onCheckedChange={(value) =>
            onChange({ enableClaudePluginIntegration: value })
          }
        />

        {settings.enableClaudePluginIntegration && (
          <div className="ml-6 text-xs text-amber-600 dark:text-amber-400">
            {t("settings.enableClaudePluginIntegrationFirstTimeHint")}
          </div>
        )}

        <AnimatePresence initial={false}>
          {settings.enableClaudePluginIntegration && (
            <motion.div
              key="ide-type-select"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.3 }}
              className="ml-6 flex flex-col gap-2"
            >
              <div className="flex flex-col gap-1">
                <Label className="text-xs text-muted-foreground">
                  {t("settings.ideTypeDescription")}
                </Label>
                <Select
                  value={settings.ideType ?? "vscode"}
                  onValueChange={(value) => onChange({ ideType: value })}
                >
                  <SelectTrigger className="w-48 h-8 text-sm">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="vscode">{t("settings.ideTypeVscode")}</SelectItem>
                    <SelectItem value="cursor">{t("settings.ideTypeCursor")}</SelectItem>
                    <SelectItem value="windsurf">{t("settings.ideTypeWindsurf")}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* 路径状态 */}
              {pathChecked && (
                detectedPath ? (
                  <div className="flex items-center gap-1.5 text-xs text-green-600 dark:text-green-400">
                    <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate" title={detectedPath}>{detectedPath}</span>
                  </div>
                ) : (
                  <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-1.5 text-xs text-amber-600 dark:text-amber-400">
                      <AlertCircle className="h-3.5 w-3.5 shrink-0" />
                      <span>{t("settings.vscodeSettingsPathNotFound")}</span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      <FolderOpen className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                      <Input
                        className="h-7 text-xs"
                        placeholder={t("settings.vscodeSettingsPathHint")}
                        value={settings.vscodeSettingsPath ?? ""}
                        onChange={(e) => onChange({ vscodeSettingsPath: e.target.value || undefined })}
                      />
                    </div>
                  </div>
                )
              )}
            </motion.div>
          )}
        </AnimatePresence>

        <ToggleRow
          icon={<MonitorUp className="h-4 w-4 text-cyan-500" />}
          title={t("settings.skipClaudeOnboarding")}
          description={t("settings.skipClaudeOnboardingDescription")}
          checked={!!settings.skipClaudeOnboarding}
          onCheckedChange={(value) => onChange({ skipClaudeOnboarding: value })}
        />

        <ToggleRow
          icon={<Search className="h-4 w-4 text-amber-500" />}
          title={t("settings.toolSearchBypass")}
          description={t("settings.toolSearchBypassDescription")}
          checked={!!settings.toolSearchBypass}
          onCheckedChange={(value) => onChange({ toolSearchBypass: value })}
        />

        <ToggleRow
          icon={<AppWindow className="h-4 w-4 text-blue-500" />}
          title={t("settings.minimizeToTray")}
          description={t("settings.minimizeToTrayDescription")}
          checked={settings.minimizeToTrayOnClose}
          onCheckedChange={(value) =>
            onChange({ minimizeToTrayOnClose: value })
          }
        />
      </div>
    </section>
  );
}
