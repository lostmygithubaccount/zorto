+++
title = "Zorto roadmap to v1"
date = "2026-04-03"
author = "Cody"
description = "Zorto's v1 roadmap."
tags = ["zorto"]
+++

The roadmap for Zorto to v1 release.

<!-- more -->

## Stable & production-ready

The release of v1.0.0 of Zorto will primarily indicate readiness for production; the APIs are stable, the code is well-tested and high-quality. Until that point, I'm favoring iteration speed. That said, Zorto is in production powering this website and all of my own static websites.

## Improved `check` for test/lint/compile

~~For people & agents, `zorto check` should provide confidence that a website is following best practices.~~ Done — `zorto check` validates broken links, frontmatter, and missing assets.

## Good default themes

~~I want at least 8, ideally 16 or more good default themes.~~ Done — 16 built-in themes shipped, all with light and dark mode support.

## Skills & more for agents

I want to build in skills that people or agents can install for ease of use. I may consider adding an MCP server.

## Desktop app and local web app (GUIs)

I want to ship a desktop app (iced GUI) and web app (TBD) that will make it even easier for users to get started with their first website. I may allow you to plug your agent into these UIs to see your website come to life as well.

## Improved executable code blocks

Currently, the executable code blocks are quite limited. I want to add support for freezing pages (i.e. caching the results), visualizations through common Python libraries, and perhaps more languages. This is an extremely powerful feature to continue building on.

## ~~Built-in search~~ (done)

Zorto now ships with built-in full-text search. Set `generate_search = true` in `config.toml` and Zorto generates a SQLite FTS5 search index at build time. The built-in themes include a search UI powered by sql.js (SQLite compiled to WebAssembly) — no external services, no API keys, entirely client-side.

## Ease of use

I want Zorto to be the easiest to use SSG for people & agents in this new era of AI. Docs must be excellent. Website creation should be easy.
