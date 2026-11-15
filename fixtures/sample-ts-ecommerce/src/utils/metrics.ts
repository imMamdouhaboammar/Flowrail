export class Metrics {
    static increment(metric: string): void {
        console.log(`[METRIC] ${metric} +1`);
    }
}
