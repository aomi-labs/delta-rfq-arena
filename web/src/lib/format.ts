export function formatDistanceToNow(date: Date): string {
  const now = new Date();
  const diff = date.getTime() - now.getTime();
  const absDiff = Math.abs(diff);

  const seconds = Math.floor(absDiff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  const prefix = diff < 0 ? "" : "in ";
  const suffix = diff < 0 ? " ago" : "";

  if (days > 0) return `${prefix}${days}d${suffix}`;
  if (hours > 0) return `${prefix}${hours}h${suffix}`;
  if (minutes > 0) return `${prefix}${minutes}m${suffix}`;
  return `${prefix}${seconds}s${suffix}`;
}

export function formatTimestamp(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

export function formatUnits(value: number, decimals = 9): string {
  return (value / Math.pow(10, decimals)).toLocaleString(undefined, {
    maximumFractionDigits: 4,
  });
}
