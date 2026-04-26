# Frontend — React + Vite + TypeScript

## Components

- Functional components only. No class components.
- One primary export per file (small helpers may share a file).
- Components render UI only — business logic lives in custom hooks.
- Props:
  - TypeScript interface named `XxxProps`.
  - Callback props use `onXxx` naming (`onClick`, `onSubmit`).
- Return type: `ReactElement` (not `JSX.Element`, deprecated in React 19).
- No complex expressions inline in JSX — extract to named variables or helper
  functions.
- Page-level components must have an error boundary.
- Every async operation handles all four UI states: loading, error, empty,
  success. Show user-friendly messages; log raw errors to console.
- No inline style objects (except for genuinely dynamic values). No
  `!important`. Use `clsx` / `cn` for conditional classes.

## Hooks

- Custom hook file and export names prefixed with `use` (`useAuth.ts`).
- A hook does one thing only.
- `useEffect` has a complete dependency array. Never suppress with
  `// eslint-disable-next-line`.
- `useEffect` with side effects returns a cleanup function.
- Never pass an async function directly to `useEffect`. Use the
  `AbortController` pattern:

  ```tsx
  useEffect(() => {
    const controller = new AbortController()
    const load = async () => {
      try {
        const data = await fetchData({ signal: controller.signal })
        setData(data)
      } catch (error) {
        if (!controller.signal.aborted) setError(error)
      }
    }
    load()
    return () => controller.abort()
  }, [])
  ```

## Performance

- `React.memo` / `useMemo` / `useCallback`: use only when a measured need
  exists — not preemptively. Valid triggers: expensive derivations; hook
  dependencies; props passed to memoized children.
- List `key` values must be stable and unique. Never use array index.
- For large lists, use virtualization (`react-virtual` / `react-window`).

## TypeScript

- Strict mode is mandatory. No `any` — use `unknown` when the type is
  genuinely uncertain, then narrow with type guards.
- Public functions have explicit return types; internal helpers may rely on
  inference.
- No `as` type assertions (unless narrowing from `unknown` with a documented
  reason).
- No `!` non-null assertions — use optional chaining (`?.`) or explicit null
  checks.
- No `enum` — prefer `as const` objects + union types
  (`type X = typeof X[keyof typeof X]`).
- Typed catch blocks: `catch (error) { if (error instanceof Foo) … }`; never
  `catch (e: any)`.
- `exactOptionalPropertyTypes`: pass optional props via conditional spread
  (`{...(flag ? { helper: "x" } : {})}`), never
  `helper={flag ? "x" : undefined}`.
- `import type` separate from value imports.
- No `@ts-ignore` / `@ts-expect-error` without a comment explaining why.

## State & data

- Start with React built-ins (`useState`, `useReducer`, `useContext`). Add an
  external state manager only when a clear need emerges.
- Prop drilling beyond 2 levels → Context (or a state manager).
- **Runtime validation at system boundaries:** all API response bodies, URL
  params, and form inputs parsed through a schema (Zod or equivalent) before
  use. Derive the compile-time type with `z.infer<typeof schema>`.
- API calls centralise in `src/api/`. Components never call `fetch` directly.

## Styling

- Tailwind CSS (v4) utility classes. Tailwind is configured via
  `@tailwindcss/vite` in `vite.config.ts`; design tokens live in
  `src/index.css` (`@theme` block) — never use arbitrary hex values.
- **shadcn/ui:** components added via CLI (`npx shadcn@latest add <component>`).
  Do not manually create shadcn components.

## Testing & tooling

- Vitest + React Testing Library. Test behaviour, not implementation.
- Two test projects in `vite.config.ts`: `vite-plugins` (node env, plugin
  tests under `vite-plugins/__tests__/`) and `frontend` (jsdom env, component
  and unit tests under `src/**/*.{test,spec}.{ts,tsx}` with setup file at
  `tests/setup.ts`). Both run together via `npm test`.
- Formatting enforced by ESLint. Do not disable rules without a documented
  reason.

## Project Structure (as it grows)

```text
frontend/
├── public/              # Static assets
├── src/
│   ├── api/             # API client functions
│   ├── components/      # Reusable UI components
│   │   └── ui/          # shadcn/ui components (generated)
│   ├── fouc/            # Pre-paint script hashed into HTML CSP at build
│   ├── hooks/           # Custom React hooks
│   ├── pages/           # Route-level page components
│   ├── lib/             # Utilities
│   ├── App.tsx          # Root component
│   └── main.tsx         # Entrypoint
├── vite-plugins/        # Custom Vite plugins (csp-hash.ts)
├── tests/               # Vitest setup (setup.ts)
├── index.html
├── tsconfig.json
└── vite.config.ts       # Tailwind v4 + Vitest projects configured here
```
