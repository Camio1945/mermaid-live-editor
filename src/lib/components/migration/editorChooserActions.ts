import { openUrl } from '$/util/navigation';
import { logEvent, logMermaidChartClick } from '$/util/stats';
import { getCheckoutUrl, getMermaidAiLiveUrl } from '$/util/util';

const utmMedium = 'editorSelection';
const utmCampaign = 'live_2026';

export interface EditorChooserActions {
  log: (buttonClick: string) => void;
  startTrial: (buttonClick?: string) => void;
  dismiss: (buttonClick: string) => void;
  openMermaidAiLive: (buttonClick: string) => void;
}

export const createEditorChooserActions = (close: () => void): EditorChooserActions => {
  const log = (buttonClick: string) => {
    logEvent('chooseEditor', { buttonClick });
  };

  const startTrial = (buttonClick = 'startTrial') => {
    log(buttonClick);
    logMermaidChartClick('editorPicker');
    close();
    openUrl(getCheckoutUrl({ utmCampaign, utmMedium }));
  };

  const dismiss = (buttonClick: string) => {
    log(buttonClick);
    close();
  };

  const openMermaidAiLive = (buttonClick: string) => {
    log(buttonClick);
    close();
    openUrl(getMermaidAiLiveUrl({ utmCampaign, utmMedium }));
  };

  return { log, startTrial, dismiss, openMermaidAiLive };
};
