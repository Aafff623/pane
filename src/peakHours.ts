// Peak / off-peak billing windows for providers whose credits cost more
// during busy hours. All windows are Beijing time (UTC+8) regardless of
// the machine's timezone. Rules verified against official docs on
// 2026-09-15:
//
//   - zai / linkso (GLM Coding Plan): peak Mon–Fri 14:00–18:00 bills 1×,
//     every other hour (weekends included) bills half credits (0.5×).
//   - commandcode (GOAT): the plan's own rolling windows are clock-free;
//     only DeepSeek-routed models inherit DeepSeek's peak Mon–Fri
//     09:00–12:00 & 14:00–18:00, weekends are idle-priced (0.5×).
//   - qodercn: daily 08:00–22:00 is standard rate; 22:00–08:00 off-peak
//     (weekends and holidays included) bills 0.04–0.2× by model.
//   - traecn: daily busy 08:00–22:00; idle 22:00–08:00 bills 0.08–0.36×
//     by model.

export interface PeakRule {
  /** Beijing-time weekly windows: day 0 = Sunday … 6 = Saturday. */
  windows: { days: number[]; fromMin: number; toMin: number }[];
  /** i18n key holding the full multiplier sentence for hover tips. */
  tipKey: string;
}

const WORKDAYS = [1, 2, 3, 4, 5];
const EVERYDAY = [0, 1, 2, 3, 4, 5, 6];

const at = (hours: number, minutes = 0) => hours * 60 + minutes;

export const PEAK_RULES: Record<string, PeakRule> = {
  zai: {
    windows: [{ days: WORKDAYS, fromMin: at(14), toMin: at(18) }],
    tipKey: "peak.rule.zai",
  },
  linkso: {
    windows: [{ days: WORKDAYS, fromMin: at(14), toMin: at(18) }],
    tipKey: "peak.rule.linkso",
  },
  commandcode: {
    windows: [
      { days: WORKDAYS, fromMin: at(9), toMin: at(12) },
      { days: WORKDAYS, fromMin: at(14), toMin: at(18) },
    ],
    tipKey: "peak.rule.commandcode",
  },
  qodercn: {
    windows: [{ days: EVERYDAY, fromMin: at(8), toMin: at(22) }],
    tipKey: "peak.rule.qodercn",
  },
  traecn: {
    windows: [{ days: EVERYDAY, fromMin: at(8), toMin: at(22) }],
    tipKey: "peak.rule.traecn",
  },
};

/// True while `family`'s peak window is active right now (Beijing time).
export function isProviderInPeak(family: string, now = Date.now()): boolean {
  const rule = PEAK_RULES[family];
  if (!rule) return false;
  // Beijing wall clock = UTC + 8h, read through UTC getters so the check
  // never depends on the machine's own timezone.
  const bj = new Date(now + 8 * 3_600_000);
  const day = bj.getUTCDay();
  const minute = bj.getUTCHours() * 60 + bj.getUTCMinutes();
  return rule.windows.some(
    (w) => w.days.includes(day) && minute >= w.fromMin && minute < w.toMin,
  );
}
