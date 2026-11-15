export function formatCentsToIso(cents: number, currencyCode: string): string {
  const majorUnits = (cents / 100).toFixed(2);
  return `${currencyCode.toUpperCase()} ${majorUnits}`;
}

export function roundBankersCents(amount: number): number {
  return Math.round(amount);
}

export function assertNonNegativeCents(cents: number, fieldName: string): void {
  if (cents < 0 || !Number.isFinite(cents)) {
    throw new Error(`Invalid monetary value for ${fieldName}: ${cents}`);
  }
}
