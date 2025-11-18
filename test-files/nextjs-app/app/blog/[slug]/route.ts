import { NextRequest, NextResponse } from 'next/server'

// Route handler outside of /api/ directory
export async function GET(
  request: NextRequest,
  { params }: { params: { slug: string } }
) {
  const slug = params.slug

  return NextResponse.json({
    post: {
      slug,
      title: `Blog Post: ${slug}`,
      content: 'This is a test blog post',
    }
  })
}

export async function POST(
  request: NextRequest,
  { params }: { params: { slug: string } }
) {
  const body = await request.json()

  return NextResponse.json({
    message: 'Post updated',
    slug: params.slug,
    data: body,
  })
}
