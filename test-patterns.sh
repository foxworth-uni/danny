#!/bin/bash
cd test-files/nextjs-app
echo "Testing glob patterns..."
echo ""
echo "Pattern: **/app/**/page.{ts,tsx,js,jsx}"
shopt -s globstar
ls -1 **/app/**/page.{ts,tsx,js,jsx} 2>&1 | head -5 || echo "No match"
echo ""
echo "Pattern: pages/**/*.{ts,tsx,js,jsx}"
ls -1 pages/**/*.{ts,tsx,js,jsx} 2>&1 | head -5 || echo "No match"
