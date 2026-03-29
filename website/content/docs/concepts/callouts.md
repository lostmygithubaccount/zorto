+++
title = "Callouts"
template = "docs.html"
+++

Zorto supports GitHub-style callout alerts in markdown using blockquote syntax.

## Syntax

```markdown
> [!NOTE]
> Your note content here.
```

## Types

There are five callout types:

> [!NOTE]
> Highlights information that users should take into account.

> [!TIP]
> Optional information to help users succeed.

> [!WARNING]
> Critical content that requires user attention due to potential risks.

> [!CAUTION]
> Negative potential consequences of an action.

> [!IMPORTANT]
> Key information users need to know.

## Usage

Callouts can contain any markdown:

> [!TIP]
> You can use **bold**, `code`, and [links](https://zorto.dev) inside callouts.
>
> Multiple paragraphs work too.
