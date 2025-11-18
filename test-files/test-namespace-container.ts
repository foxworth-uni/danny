// Test file to verify namespace container reporting (Feature 1)

// This namespace is completely unused and NOT exported
namespace UnusedNamespace {
  export function helper1() { return 1; }
  export function helper2() { return 2; }
  export const CONSTANT = 42;
}

// Regular unused function for comparison
function regularUnusedFunction() {
  return "test";
}
