# Themes

Zorto's theme system provides a complete visual starting point — templates, styles, and light/dark mode — that you can use as-is or progressively override.

## Architecture

A theme is a bundle of Tera templates and SCSS stylesheets embedded directly in the Zorto binary. Set `theme = "zorto"` in your config and the build pipeline uses that theme's templates and styles as defaults.

Zorto ships eight built-in themes: `zorto`, `dkdc`, `default`, `ember`, `forest`, `ocean`, `rose`, and `slate`.

## The override model

Themes follow a layered precedence model:

1. **Theme defaults** — the templates and styles bundled with the theme
2. **Local overrides** — any file you place in `templates/` or `sass/` in your project

{{ cascade(items="Fallback:Theme defaults — templates and styles bundled with the theme:default|Priority:Your project — files in templates/ and sass/:wins", caption="Local files always take priority. Override one template or all of them.") }}

If Zorto finds `templates/page.html` in your project, it uses that instead of the theme's `page.html`. The same applies to styles: a local `sass/style.scss` replaces the theme's `style.scss` entirely. Only files with the same name get replaced.

Override a single template while keeping everything else from the theme, or replace all templates entirely. You can also extend a theme's base layout and only replace specific blocks.

## Light and dark mode

Every built-in theme includes a toggle in the navbar. The user's preference is saved to `localStorage` and respected on return visits. The default follows the operating system preference via `prefers-color-scheme`. No configuration needed.

## When to use which theme

- **`default`** or **`slate`** — clean, minimal starting points. Good for documentation sites or simple blogs where you want to add your own personality through style overrides.
- **`zorto`** or **`dkdc`** — opinionated designs with color schemes and animations. Good for project landing pages or branded sites.
- **`ember`**, **`forest`**, **`ocean`**, **`rose`** — color-themed variants. Each provides a distinct palette while sharing the same template structure.

If none of the built-in themes fit, start with `default` and override the styles. The template structure is the same across all themes.

## Further reading

- [Templates](templates.md) — how Tera templating and block inheritance work
- [How to customize your theme](../how-to/customize-theme.md) — step-by-step overrides for templates and styles
