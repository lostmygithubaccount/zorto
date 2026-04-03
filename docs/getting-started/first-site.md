# Next steps

Now that you have a working site, here are some directions to explore.

## Recommended first: use a built-in theme

The tutorial used hand-written templates. Zorto ships with built-in themes (`zorto`, `dkdc`, `light`, `dark`) that provide ready-made templates and styles. Add one line to `config.toml`:

```toml
theme = "zorto"
```

With a theme active, you can delete the `templates/` directory entirely — the theme provides everything. You can still override individual templates by placing files in `templates/`.

See [Themes](../concepts/themes.md) for details.

## Deploy your site

Your site is ready to go live. See [Deploy your site](../how-to/deploy.md) for setup with Netlify, Vercel, Cloudflare Pages, or GitHub Pages.

## Learn the concepts

- [Content model](../concepts/content-model.md): sections, pages, frontmatter, and how Zorto organizes content
- [AI-native](../concepts/ai-native.md): explicit contracts, build-time validation, llms.txt
- [Executable code blocks](../concepts/executable-code.md): run Python and Bash at build time
- [Templates](../concepts/templates.md): the Tera template engine and available context variables
- [Configuration](../concepts/configuration.md): every `config.toml` option explained

## Follow the how-to guides

- [Add a blog](../how-to/add-blog.md): full guide with pagination and summaries
- [Customize a theme](../how-to/customize-theme.md): override templates and styles
- [Add a custom domain](../how-to/custom-domain.md): DNS records for each hosting provider
- [Organize content](../how-to/organize-content.md): nested sections, co-located assets
- [SEO](../how-to/seo.md): meta tags, Open Graph, llms.txt
