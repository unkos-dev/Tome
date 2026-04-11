# Frontend — React + Vite + TypeScript

## Conventions

- **Components:** Functional components only. No class components.
- **Styling:** Tailwind CSS utility classes. Use the project's design tokens
  (defined in `tailwind.config.ts`) — never arbitrary hex values.
- **shadcn/ui:** Components added via CLI (`npx shadcn@latest add <component>`).
  Do not manually create shadcn components.
- **State management:** Start with React built-ins (`useState`, `useReducer`,
  `useContext`). Add external state management only when a clear need emerges.
- **API calls:** Centralise in `src/api/` module. Never call `fetch` directly from
  components.
- **TypeScript:** Strict mode enabled. No `any` types. No `@ts-ignore` unless
  documented.
- **Testing:** Vitest + React Testing Library. Test behaviour, not implementation.
- **Formatting:** Enforced by ESLint. Do not disable rules without documented reason.

## Project Structure (as it grows)

```text
frontend/
├── public/              # Static assets
├── src/
│   ├── api/             # API client functions
│   ├── components/      # Reusable UI components
│   │   └── ui/          # shadcn/ui components (generated)
│   ├── hooks/           # Custom React hooks
│   ├── pages/           # Route-level page components
│   ├── lib/             # Utilities
│   ├── App.tsx          # Root component
│   └── main.tsx         # Entrypoint
├── index.html
├── tailwind.config.ts
├── tsconfig.json
└── vite.config.ts
```
