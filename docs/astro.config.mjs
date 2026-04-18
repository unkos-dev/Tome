// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://unkos-dev.github.io",
  base: "/Reverie",
  integrations: [
    starlight({
      title: "Reverie",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/unkos-dev/Reverie",
        },
      ],
      sidebar: [
        {
          label: "Getting Started",
          items: [
            { label: "Introduction", slug: "getting-started/introduction" },
          ],
        },
        {
          label: "Reference",
          autogenerate: { directory: "reference" },
        },
      ],
    }),
  ],
});
