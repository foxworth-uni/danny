// Examples of UNUSED EXPORTS - various patterns that Knip detects

// 1. Simple unused named export
export function unusedHelper() {
  return "This function is never imported anywhere";
}

// 2. Used export (for comparison)
export function formatCurrency(amount: number): string {
  return `$${amount.toFixed(2)}`;
}

// 3. Multiple unused exports
export const UNUSED_CONSTANT = "never used";
export const ANOTHER_UNUSED = 42;

// 4. Unused type export
export type UnusedType = {
  id: string;
  name: string;
};

// 5. Unused interface
export interface UnusedInterface {
  value: string;
}

// 6. Enum with unused members
export enum Status {
  PENDING = 'pending',
  APPROVED = 'approved',
  REJECTED = 'rejected', // This might be unused
  ARCHIVED = 'archived'  // This might be unused
}

// 7. Class with unused members
export class DataProcessor {
  // Used method
  public process(data: string): string {
    return data.toUpperCase();
  }

  // UNUSED method - never called
  public validate(data: string): boolean {
    return data.length > 0;
  }

  // UNUSED private method
  private internalHelper(): void {
    console.log('never used');
  }
}

// 8. Namespace with unused exports
export namespace Utils {
  export function used() {
    return true;
  }

  export function unused() {
    return false;
  }
}

// 9. Re-export that might be unused
export { validateEmail } from './validators';

// 10. Default export (might be unused)
export default function unusedDefaultExport() {
  return "This default export is never imported";
}
