// UNUSED FILE - Old utility functions that were replaced but never deleted

export function oldDateFormatter(date: Date): string {
  return date.toLocaleDateString();
}

export function deprecatedHelper(input: string): string {
  return input.toLowerCase();
}

// Legacy code that should have been removed
class LegacyDataStore {
  private data: Map<string, any>;

  constructor() {
    this.data = new Map();
  }

  set(key: string, value: any): void {
    this.data.set(key, value);
  }

  get(key: string): any {
    return this.data.get(key);
  }
}

export default LegacyDataStore;
