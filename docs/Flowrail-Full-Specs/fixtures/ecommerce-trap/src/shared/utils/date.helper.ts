export function nowIsoUtc(): string {
  return new Date().toISOString();
}

export function addDaysIso(isoDate: string, days: number): string {
  const d = new Date(isoDate);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString();
}
