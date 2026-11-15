export class Metrics {
  public static increment(metricName: string, tags?: Record<string, string>): void {
    void metricName;
    void tags;
  }

  public static timing(metricName: string, durationMs: number): void {
    void metricName;
    void durationMs;
  }
}

export class Tracer {
  public static startSpan(spanName: string): { end: () => void } {
    void spanName;
    return {
      end: () => {}
    };
  }
}
