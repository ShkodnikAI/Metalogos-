---
name: nextjs-architecture
description: Architectural patterns for Next.js 16+ applications (current stack). App router, server components, server actions, streaming, error boundaries, route groups. When to use what. Common pitfalls in modern Next.js. Sets defaults for new Next.js projects in Fosved Office.
---

# Next.js Architecture — Patterns for App Router Era

Next.js 16 (current 2026) is fundamentally different from pages-router era. App router with server components is default. This skill defines how we build Next.js apps in Fosved Office.

## Prerequisites

- Tech radar confirms Next.js as adopt state
- Project type: web_app or site
- Familiarity with React 19 server components concept

## Core principle

> Server components are the default. Client components are the exception. The reverse mindset (everything client unless forced server) is the legacy from pages-router era. New mental model: render on server, send HTML, hydrate only what needs interactivity.

## Project structure (Next.js 16)

```
src/
├── app/                          # App router
│   ├── layout.tsx                # Root layout
│   ├── page.tsx                  # Home page
│   ├── globals.css               # Global styles
│   ├── (marketing)/              # Route group (no URL segment)
│   │   ├── about/page.tsx
│   │   └── pricing/page.tsx
│   ├── dashboard/                # Authenticated area
│   │   ├── layout.tsx            # Dashboard layout (sidebar, etc.)
│   │   ├── page.tsx
│   │   └── settings/
│   │       └── page.tsx
│   ├── api/                      # API routes
│   │   └── users/route.ts
│   └── error.tsx                 # Error boundary
├── components/
│   ├── ui/                       # Shadcn-style primitives
│   ├── layouts/                  # Layout components
│   └── features/                 # Feature-specific components
├── lib/                          # Shared utilities
│   ├── db.ts                     # Prisma client singleton
│   ├── auth.ts                   # Auth helpers
│   └── utils.ts                  # Generic utilities
├── hooks/                        # Client-only hooks
├── server/                       # Server-only modules
│   ├── actions/                  # Server actions
│   └── queries/                  # DB queries
└── types/                        # Shared TypeScript types
```

## Server vs Client Components — decision rules

**Server component (default)** when:
- Reads data from DB
- Reads from filesystem
- Uses secrets / API keys
- Renders content (doesn't need interaction)
- Renders other server components

**Client component (`'use client'`)** when:
- Uses `useState`, `useEffect`, other hooks
- Has event handlers (onClick, onChange)
- Uses browser-only APIs (window, document, localStorage)
- Uses third-party libraries that require client (most chart libraries, some UI libs)

**Rule:** make client component as **small as possible**. Wrap the interactive bit, leave the rest server.

Bad:
```tsx
'use client';
export default function Page() {
  return (
    <div>
      <Header />              {/* No interactivity needed */}
      <ProductList products={products} />  {/* Static rendering */}
      <ContactForm />         {/* Only this needs client */}
    </div>
  );
}
```

Good:
```tsx
// page.tsx (server)
export default async function Page() {
  const products = await db.products.findMany();
  return (
    <div>
      <Header />
      <ProductList products={products} />
      <ContactForm />  {/* This is the client component */}
    </div>
  );
}

// contact-form.tsx (client only this one)
'use client';
export function ContactForm() {
  const [email, setEmail] = useState('');
  ...
}
```

## Data fetching patterns

**Server component direct fetch (default):**
```tsx
export default async function Page() {
  const data = await db.posts.findMany();
  return <PostList posts={data} />;
}
```

**Server actions for mutations:**
```tsx
// server/actions/posts.ts
'use server';
export async function createPost(formData: FormData) {
  const title = formData.get('title');
  await db.posts.create({ data: { title } });
  revalidatePath('/posts');
}

// page.tsx
import { createPost } from '@/server/actions/posts';
export default function Page() {
  return <form action={createPost}>...</form>;
}
```

**Client-side fetch only when:**
- Real-time data (use SSE, WebSocket, or polling)
- Optimistic updates (use `useOptimistic`)
- Search-as-you-type (debounced fetch from client)

In these cases: `useSWR` or `@tanstack/react-query` for caching.

## Streaming and Suspense

Slow data shouldn't block the page. Use Suspense boundaries:

```tsx
export default function Page() {
  return (
    <>
      <Header />  {/* Renders immediately */}
      <Suspense fallback={<Spinner />}>
        <SlowDataComponent />  {/* Streams in when ready */}
      </Suspense>
      <Footer />
    </>
  );
}
```

For long-running operations: stream partial UI. User sees something fast.

## Route groups

Use `(groupName)` folders to group routes without affecting URL:

```
app/
├── (marketing)/        # URL doesn't include "marketing"
│   ├── about/page.tsx  # → /about
│   └── pricing/page.tsx
├── (authenticated)/    # URL doesn't include "authenticated"
│   ├── dashboard/page.tsx  # → /dashboard
│   └── settings/page.tsx
```

Reasons:
- Different layouts for different sections
- Logical separation
- Auth boundaries

## Layouts

Layout component wraps all routes in the segment. Persists across navigation.

```tsx
// app/(authenticated)/layout.tsx
import { auth } from '@/lib/auth';
import { redirect } from 'next/navigation';

export default async function DashboardLayout({ children }) {
  const session = await auth();
  if (!session) redirect('/login');

  return (
    <div>
      <Sidebar />
      <main>{children}</main>
    </div>
  );
}
```

Layouts are server components by default. Auth check at layout level is the standard pattern.

## Error boundaries

`error.tsx` catches errors in segment:

```tsx
'use client';
export default function Error({ error, reset }) {
  return (
    <div>
      <h2>Something went wrong</h2>
      <button onClick={reset}>Try again</button>
    </div>
  );
}
```

`not-found.tsx` for 404 pages. `loading.tsx` for loading states.

## API routes (when needed)

For external API consumers (mobile apps, webhooks, etc.):

```tsx
// app/api/posts/route.ts
import { NextResponse } from 'next/server';

export async function GET() {
  const posts = await db.posts.findMany();
  return NextResponse.json({ posts });
}

export async function POST(request: Request) {
  const body = await request.json();
  const post = await db.posts.create({ data: body });
  return NextResponse.json({ post }, { status: 201 });
}
```

For internal use (within Next.js app): use server actions, not API routes.

## State management

In order of preference:
1. **URL state** (search params) — for filters, pagination, tabs
2. **Server state** — useSWR/react-query for cached server data
3. **Form state** — React 19 useFormState for forms
4. **Component state** — useState for ephemeral UI
5. **Global client state** — Zustand or Context (rarely needed in app router)

Redux is overkill for most Next.js apps now. Zustand if you really need cross-component state.

## Styling

**Tailwind 4** (current adopt). Utility-first.

Components from **shadcn/ui** (copy-paste, not npm install). Owned components, no version conflicts.

Use Radix UI primitives for accessibility (shadcn builds on Radix).

CSS modules only for special cases (complex animations).

NO CSS-in-JS in new projects (styled-components, emotion) — performance penalties in app router.

## Deployment

Default: **Render** or **Vercel** (Vercel for cutting-edge features, Render for cost).

Configuration:
- `next.config.js` for any custom config
- Environment variables in deployment dashboard, never committed
- Image optimization configured (use next/image)
- Standalone output mode for Docker if needed

## Common pitfalls

- **`'use client'` at top of page.tsx**. Makes entire page client. Almost always wrong.
- **Fetching in client when server can do it**. Server component fetch is faster (no round trip), more secure (no leaking API URLs).
- **Forgetting `revalidatePath` after mutations**. UI shows stale data.
- **Mutations in GET handlers**. Use server actions or POST routes.
- **Inline `<style>` tags in JSX**. Use Tailwind classes.
- **Importing server-only modules in client components**. Use 'server-only' package to enforce.
- **Using `useEffect` for data fetching in server components**. Hooks don't work in server components. Fetch directly with await.
- **Not using Suspense for slow data**. Blocks entire page render.

## Anti-patterns

- **Pages-router patterns in app-router project.** getStaticProps, getServerSideProps don't exist. Use server components.
- **Custom server.js** unless absolutely required. Next.js handles it.
- **`getServerSideProps` mental model.** Replaced by server components. Mindset shift required.
- **Marshaling everything to client.** Use server components first, demote to client only for interaction.
- **Skipping error boundaries.** Unhandled errors show generic Next.js page.
- **Mixing app and pages router.** Pick one. Migrate fully if transitioning.

## Performance baseline

For Fosved Next.js apps, target:
- LCP < 2s on slow 3G
- FID < 100ms
- CLS < 0.1
- Bundle size for initial JS: < 200kb gzipped
- Lighthouse score: 90+ across categories

If metrics worse: investigate, file performance task.

## Integration

- New web app DevTask → loads this skill + `code-organization-standards`
- ADR for stack choice references this skill ("using Next.js per nextjs-architecture skill")
- `lib/dev.js` `createWebAppRepo()` scaffolds project per this structure
- `database-design`, `api-design`, `dev-handoff-specs` apply on top of this foundation
