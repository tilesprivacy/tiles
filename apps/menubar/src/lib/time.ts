const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * not Intl.RelativeTimeFormat, its "3 days ago" is wider than the column. these
 * sit in a right-aligned tabular column, so they stay as short as they can
 */
export function relativeTime(ms: number): string {
  const elapsed = Date.now() - ms;

  // a clock skew or a seed written a moment ahead
  if (elapsed < MINUTE) return "now";
  if (elapsed < HOUR) return `${Math.floor(elapsed / MINUTE)}m`;
  if (elapsed < DAY) return `${Math.floor(elapsed / HOUR)}h`;

  const days = Math.floor(elapsed / DAY);
  if (days < 7) return `${days}d`;

  return new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}
