# Callouts

Callouts are styled alert boxes rendered from standard markdown blockquote syntax. Zorto uses the [GitHub-style](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts) `[!TYPE]` format — no shortcodes or HTML needed.

## Syntax

```markdown
> [!NOTE]
> Your content here. Supports **bold**, `code`, [links](https://example.com),
> and multiple paragraphs.
```

## Types

There are five callout types, each with a distinct color and icon:

> [!NOTE]
> Highlights information that users should take into account.

> [!TIP]
> Optional information to help users succeed.

> [!IMPORTANT]
> Key information users need to know.

> [!WARNING]
> Critical content that requires user attention due to potential risks.

> [!CAUTION]
> Negative potential consequences of an action.

## Rich content

Callouts support the full range of markdown inside them:

```markdown
> [!TIP]
> You can nest **bold**, `inline code`, and [links](https://zorto.dev).
>
> Multiple paragraphs work. So do:
> - Bullet lists
> - Code blocks (indented)
> - Images
```

## When to use callouts vs. shortcodes

Callouts are best for inline alerts within prose — notes, warnings, tips. For more structured content like collapsible sections or styled cards, use [shortcodes](shortcodes.md).

Zorto also has a `note` shortcode (`{% note(type="info") %}`) that produces similar-looking boxes. Prefer callouts for standard prose alerts — they use standard Markdown syntax and render on GitHub too. Use the `note` shortcode only when you need programmatic control (e.g., in a template or shortcode body).

## Further reading

- [Callouts reference](../reference/callouts.md) — all five types with rich content examples
- [Shortcodes](shortcodes.md) — the `note` shortcode and other rich content components
