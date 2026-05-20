export function formatTimelineAxisLabel(value: string) {
  const text = String(value ?? "");
  const dayMatch = text.match(/^(\d{4})-(\d{2})-(\d{2})(?:$|[^\d])/);
  if (dayMatch) {
    return `${dayMatch[2]}-${dayMatch[3]}`;
  }

  const monthMatch = text.match(/^(\d{4})-(\d{2})(?:$|[^\d])/);
  if (monthMatch) {
    return `${monthMatch[1]}\n${monthMatch[2]}`;
  }

  const weekMatch = text.match(/^Wk\s+(.+)$/i);
  if (weekMatch) {
    return weekMatch[1].replace(/^(\d{4})-/, "");
  }

  return text.length > 12 ? `${text.slice(0, 12)}...` : text;
}

export function formatCompactCategoryLabel(value: string) {
  const text = String(value ?? "");
  return text.length > 12 ? `${text.slice(0, 12)}...` : text;
}

export function buildCategoryAxisLabel(
  color: string,
  itemCount: number,
  formatter: (value: string) => string = formatCompactCategoryLabel,
) {
  return {
    color,
    hideOverlap: true,
    interval: "auto" as const,
    margin: itemCount > 8 ? 12 : 8,
    rotate: itemCount > 12 ? 28 : 0,
    formatter,
  };
}

export function categoryGridBottom(itemCount: number) {
  return itemCount > 12 ? 48 : itemCount > 8 ? 38 : 26;
}
