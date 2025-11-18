import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'

// Next.js 16 proxy file (replaces middleware.ts)
export default function proxy(request: NextRequest) {
  // Add custom header
  const requestHeaders = new Headers(request.headers)
  requestHeaders.set('x-proxy-version', '16')

  // Check if request is for admin
  if (request.nextUrl.pathname.startsWith('/admin')) {
    // Rewrite to admin dashboard
    return NextResponse.rewrite(new URL('/dashboard', request.url))
  }

  return NextResponse.next({
    request: {
      headers: requestHeaders,
    },
  })
}

export const config = {
  matcher: [
    '/((?!api|_next/static|_next/image|favicon.ico).*)',
  ],
}
