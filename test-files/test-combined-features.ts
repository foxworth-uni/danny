// Test file to verify both features together

// EXPORTED enum - should be skipped by default, reported with --include-exports
export enum ExportedStatus {
  Active = 'active',
  Inactive = 'inactive',
  Pending = 'pending'
}

// INTERNAL enum - should always be reported (just the container, not members)
enum InternalStatus {
  Draft = 'draft',
  Published = 'published'
}

// EXPORTED namespace - should be skipped by default
export namespace ExportedUtils {
  export function helper() { return 1; }
}

// INTERNAL namespace - should be reported (just the container)
namespace InternalUtils {
  export function process() { return 2; }
}

// Regular exported function - should be skipped by default
export function exportedFunction() {
  return "exported";
}

// Regular internal function - should always be reported
function internalFunction() {
  return "internal";
}
