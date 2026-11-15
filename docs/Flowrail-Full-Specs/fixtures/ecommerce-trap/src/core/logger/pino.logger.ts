export class Logger {
  public static info(context: string, message: string, meta?: Record<string, unknown>): void {
    const timestamp = new Date().toISOString();
    console.log(`[${timestamp}] [INFO] [${context}] ${message}`, meta ?? '');
  }

  public static warn(context: string, message: string, meta?: Record<string, unknown>): void {
    const timestamp = new Date().toISOString();
    console.warn(`[${timestamp}] [WARN] [${context}] ${message}`, meta ?? '');
  }

  public static error(context: string, message: string, err?: unknown): void {
    const timestamp = new Date().toISOString();
    console.error(`[${timestamp}] [ERROR] [${context}] ${message}`, err ?? '');
  }
}
