import js from '@eslint/js'                                                                                                                                                         
import globals from 'globals'                                                                                                                                                       
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'                                                                                                                              
import react from 'eslint-plugin-react'                                                                                     
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

// `no-restricted-syntax` aggregates three independent project bans:
//
//   1. Hex literals — every brand colour must come from the canonical
//      token system (--color-canvas, --color-fg, --color-accent, etc.).
//      The matching Stylelint rule covers .css files; this rule covers
//      .ts/.tsx call sites where a developer might be tempted to inline
//      e.g. `style={{ color: '#C9A961' }}`. Lockup.tsx (and its test,
//      which asserts on those very hex values) is the documented
//      exemption — see philosophy §11C.
//
//   2. TypeScript `enum` — frontend/CLAUDE.md mandates `as const`
//      objects + union types in place of enums. The rule machine-
//      enforces that ban so reviewers do not have to.
//
//   3. Inline JSX object-literal `style={{...}}` — frontend/CLAUDE.md
//      mandates Tailwind classes + design tokens. The shadcn/ui
//      directory is carved out below because the CLI emits inline
//      style objects for CSS-variable plumbing.
const restrictedSyntaxRule = {
  'no-restricted-syntax': [
    'error',
    {
      selector: "Literal[value=/^#[0-9a-fA-F]{3,8}$/]",
      message:
        'No raw hex codes in .tsx/.ts. Use semantic tokens (bg-canvas, text-fg, etc.).',
    },
    {
      selector: 'TSEnumDeclaration',
      message:
        'No `enum` — use `as const` objects + union types (frontend/CLAUDE.md).',
    },
    {
      selector:
        'JSXAttribute[name.name="style"] > JSXExpressionContainer > ObjectExpression',
      message:
        'No inline JSX style objects. Use Tailwind classes / design tokens (frontend/CLAUDE.md).',
    },
  ],
}

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.strictTypeChecked,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
      react.configs.flat.recommended,
      react.configs.flat['jsx-runtime'],
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    settings: {
      react: { version: 'detect' },
    },
    rules: {
      ...restrictedSyntaxRule,
      // Machine-enforces the frontend/CLAUDE.md ban on `as` casts against
      // object literals (`{ ... } as X`). The chained-through-`unknown`
      // form (`{ ... } as unknown as X`) — the documented escape hatch —
      // is unaffected. Two overrides below carve out the directories
      // where this pattern is unavoidable.
      '@typescript-eslint/consistent-type-assertions': [
        'error',
        { assertionStyle: 'as', objectLiteralTypeAssertions: 'never' },
      ],
      // Enforce `import type { X }` for type-only imports. Required by
      // verbatimModuleSyntax in tsconfig.app.json — without this rule,
      // tsc fails the build for missed type-only imports while ESLint
      // stays silent. Auto-fixable.
      '@typescript-eslint/consistent-type-imports': 'error',
      // Stable `key` prop required (frontend/CLAUDE.md). `react/jsx-key`
      // is already on via react/recommended; restate explicitly so the
      // intent is visible at the config level.
      'react/jsx-key': 'error',
      'react/no-array-index-key': 'error',
    },
  },
  // Lockup is the canonical brand-identifier component and intentionally
  // inlines #C9A961 / #0E0D0A / #E8E0D0 as constants — see philosophy
  // spec §11C: the Lockup must render correctly even before
  // themes/index.css resolves (e.g. on the OIDC error page). The test
  // file asserts on those very hex values so the brand invariant
  // surfaces if anyone changes them. Per-file `eslint-disable-next-line`
  // would scatter the rationale; this overrides block keeps it visible
  // at the config level.
  {
    files: ['src/components/Lockup.tsx', 'src/components/Lockup.test.tsx'],
    rules: {
      'no-restricted-syntax': 'off',
    },
  },
  // shadcn/ui primitives are CLI-generated (`npx shadcn@latest add`).
  // The CLI emits `style={{...} as React.CSSProperties}` to type-cast
  // CSS-variable inline-style objects — a documented gap in
  // `@types/react`'s `style` typing that every React+CSS-vars project
  // hits. Rewriting the CLI output would conflict with future shadcn
  // registry updates; the alias-layer in `styles/themes/index.css`
  // handles theming without per-file edits. Off the cast rule and the
  // inline-style ban here.
  {
    files: ['src/components/ui/**'],
    rules: {
      '@typescript-eslint/consistent-type-assertions': 'off',
      'no-restricted-syntax': 'off',
    },
  },
  // Test fixture casts (`{ ok, status, json } as Response`) are partial
  // mocks — only the surface the test exercises is implemented. The
  // `as unknown as X` chain is the documented escape hatch but reads as
  // a verbose workaround for what is already a recognised testing
  // idiom. Most OSS TS projects (TanStack, tRPC, Astro, Prisma) carve
  // tests out of this rule for the same reason. Production code stays
  // strict.
  {
    files: ['**/*.test.{ts,tsx}'],
    rules: {
      '@typescript-eslint/consistent-type-assertions': 'off',
    },
  },
])