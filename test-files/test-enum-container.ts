// Test file to verify enum container reporting (Feature 1)

// This enum is completely unused and NOT exported
// Should report only "UnusedStatus" (1 line), not each member (4 lines)
enum UnusedStatus {
  PENDING = 'pending',
  APPROVED = 'approved',
  REJECTED = 'rejected',
  ARCHIVED = 'archived'
}

// This namespace is completely unused and NOT exported
// Should report only "UnusedUtils" (1 line), not each member
namespace UnusedUtils {
  export function helper1() { return 1; }
  export function helper2() { return 2; }
  export const CONSTANT = 42;
}

// This is a regular unused function for comparison
function unusedFunction() {
  return "test";
}
