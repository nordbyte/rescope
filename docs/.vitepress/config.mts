import { defineConfig } from "vitepress";

const repo = "https://github.com/nordbyte/rescope";

export default defineConfig({
  title: "rescope",
  description: "Inspect and record resource usage by process and user.",
  lang: "en-US",
  base: "/rescope/",
  cleanUrls: false,
  lastUpdated: true,
  sitemap: {
    hostname: "https://nordbyte.github.io/rescope/"
  },
  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: "/rescope/favicon.svg" }],
    ["link", { rel: "alternate icon", type: "image/svg+xml", href: "/rescope/rescope-logo.svg" }],
    ["link", { rel: "apple-touch-icon", href: "/rescope/rescope-logo.svg" }],
    ["meta", { name: "theme-color", content: "#116466" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:title", content: "rescope documentation" }],
    ["meta", { property: "og:description", content: "CLI documentation for live and recorded process resource analysis." }],
    ["meta", { property: "og:url", content: "https://nordbyte.github.io/rescope/" }]
  ],
  themeConfig: {
    logo: "/rescope-logo.svg",
    siteTitle: "rescope docs",
    nav: [
      { text: "Home", link: "/" },
      { text: "GitHub", link: repo },
      { text: "npm", link: "https://www.npmjs.com/package/rescope" }
    ],
    search: {
      provider: "local"
    },
    outline: {
      level: [2, 3],
      label: "On this page"
    },
    editLink: {
      pattern: `${repo}/edit/main/docs/:path`,
      text: "Edit page"
    },
    socialLinks: [
      { icon: "github", link: repo }
    ],
    docFooter: {
      prev: "Previous",
      next: "Next"
    },
    footer: {
      message: "Released under the MIT License.",
      copyright: "Copyright (c) Nordbyte"
    },
    sidebar: [
      {
        text: "Start",
        items: [
          { text: "Overview", link: "/" },
          { text: "Installation", link: "/start/install" },
          { text: "Quickstart", link: "/start/quickstart" },
          { text: "Core concepts", link: "/start/core-concepts" },
          { text: "Troubleshooting", link: "/start/troubleshooting" }
        ]
      },
      {
        text: "Guides",
        items: [
          { text: "Live monitoring", link: "/guides/live" },
          { text: "Recording reports", link: "/guides/recording" },
          { text: "Filters and grouping", link: "/guides/filters-grouping" },
          { text: "Exports", link: "/guides/exports" },
          { text: "Privacy", link: "/guides/privacy" }
        ]
      },
      {
        text: "Reference",
        items: [
          { text: "CLI command reference", link: "/commands/" },
          { text: "CLI options", link: "/reference/options" },
          { text: "Metrics", link: "/reference/metrics" },
          { text: "Output formats", link: "/reference/output-formats" },
          { text: "Exit codes", link: "/reference/exit-codes" }
        ]
      },
      {
        text: "Command docs",
        collapsed: true,
        items: [
          { text: "snapshot", link: "/commands/snapshot" },
          { text: "live", link: "/commands/live" },
          { text: "record", link: "/commands/record" },
          { text: "help and version", link: "/commands/help-version" }
        ]
      },
      {
        text: "Internals",
        items: [
          { text: "Architecture", link: "/internals/architecture" },
          { text: "Development", link: "/internals/development" },
          { text: "GitHub Pages", link: "/internals/github-pages" }
        ]
      }
    ]
  }
});
