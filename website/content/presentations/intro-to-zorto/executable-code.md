+++
title = "Executable code blocks"
weight = 50
+++

{{ slide_image(src="/zorto-mark-transparent.png", alt="Zorto", top="20px", right="20px", width="72px") }}

## Executable code blocks

Write Python in a fenced block -- Zorto runs it at build time and embeds the output:

```{python}
# Stdlib only -- this demo runs without extra installs.
import statistics

revenue = [12, 15, 13, 17, 21, 24]
costs   = [10, 11, 12, 13, 14, 15]

print(f"Mean revenue: {statistics.mean(revenue):.2f}")
print(f"Mean costs:   {statistics.mean(costs):.2f}")
print(f"Margin:       {statistics.mean(revenue) - statistics.mean(costs):.2f}")
```

Add `plotly`, `matplotlib`, `altair`, or `seaborn` to your venv to embed live charts the same way.
