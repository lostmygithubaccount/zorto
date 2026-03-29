+++
title = "Callouts reference"
template = "docs.html"
+++

Zorto supports GitHub-style callout alerts. Use blockquote syntax with a type marker.

## Syntax

```markdown
> [!TYPE]
> Content goes here.
```

## All callout types

### Note

> [!NOTE]
> Highlights information that users should take into account, even when skimming.

```markdown
> [!NOTE]
> Highlights information that users should take into account, even when skimming.
```

### Tip

> [!TIP]
> Optional information to help a user be more successful.

```markdown
> [!TIP]
> Optional information to help a user be more successful.
```

### Important

> [!IMPORTANT]
> Crucial information necessary for users to succeed.

```markdown
> [!IMPORTANT]
> Crucial information necessary for users to succeed.
```

### Warning

> [!WARNING]
> Critical content demanding immediate user attention due to potential risks.

```markdown
> [!WARNING]
> Critical content demanding immediate user attention due to potential risks.
```

### Caution

> [!CAUTION]
> Negative potential consequences of an action.

```markdown
> [!CAUTION]
> Negative potential consequences of an action.
```

## Rich content

Callouts support full markdown inside:

> [!TIP]
> You can use:
>
> - **Bold** and *italic* text
> - `inline code` and code blocks
> - [Links](https://zorto.dev)
> - Lists and other block elements
