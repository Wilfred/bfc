/**
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

// See https://docusaurus.io/docs/site-config for all the possible
// site configuration options.

const siteConfig = {
  title: "bfc", // Title for your website.
  tagline: "an industrial-grade Brainfuck compiler",
  url: "http://bfc.wilfred.me.uk",
  baseUrl: "/",
  projectName: "bfc-site",
  organizationName: "wilfred",

  headerLinks: [
    { doc: "getting-started", label: "Geting Started" },
    { doc: "optimisations", label: "Optimisations" },
    { doc: "faq", label: "FAQ" },
    { doc: "changelog", label: "Changelog" },
  ],

  /* path to images for header/footer */
  headerIcon: "img/header_icon.png",
  // footerIcon: "img/favicon.png",
  favicon: "img/favicon.png",

  /* Colors for website */
  colors: {
    primaryColor: "#791919",
    secondaryColor: "#202020", // Only visible on mobile site AFAICS
  },

  // This copyright info is used in /core/Footer.js and blog RSS/Atom feeds.
  copyright: `Copyright © ${new Date().getFullYear()} Wilfred Hughes`,

  highlight: {
    // Highlight.js theme to use for syntax highlighting in code blocks.
    theme: "default",
  },

  // Add custom scripts here that would be placed in <script> tags.
  scripts: [],

  // On page navigation for the current documentation page.
  onPageNav: "separate",
  // No .html extensions for paths.
  cleanUrl: true,

  // Open Graph and Twitter card images.
  ogImage: "img/logo.png",
  twitterImage: "img/logo.png",

  // For sites with a sizable amount of content, set collapsible to true.
  // Expand/collapse the links and subcategories under categories.
  // docsSideNavCollapsible: true,

  // Show documentation's last contributor's name.
  // enableUpdateBy: true,

  // Show documentation's last update time.
  // enableUpdateTime: true,

  // You may provide arbitrary config keys to be used as needed by your
  // template. For example, if you need your repo's URL...
  // repoUrl: 'https://github.com/facebook/test-site',
};

module.exports = siteConfig;
