// actionlint version pinned to v1.7.12 — keep in lockstep with the
// `workflow-lint` job in .github/workflows/ci.yml so local pre-commit
// and CI never drift. See CONTRIBUTING.md for install instructions.
module.exports = {
  "*.md": "markdownlint-cli2",
  "*.{ts,tsx,js,jsx,json,yaml,yml,css,md}": "prettier --check",
  ".github/workflows/*.{yml,yaml}": "actionlint -color",
};
