# Create charts with matplotlib

Use [matplotlib](https://matplotlib.org/) in executable code blocks to generate static charts that render as inline PNGs.

## Setup

Add `matplotlib` to your site's `pyproject.toml`:

```toml
[project]
dependencies = ["matplotlib"]
```

Then run `uv sync` in your site directory.

## Basic line chart

````markdown
```{python}
import matplotlib.pyplot as plt

x = [1, 2, 3, 4, 5]
y = [2, 4, 7, 11, 16]

plt.figure(figsize=(8, 4))
plt.plot(x, y, color='#7c3aed', linewidth=2, marker='o')
plt.title('Growth over time')
plt.xlabel('Month')
plt.ylabel('Users (k)')
plt.grid(True, alpha=0.3)
plt.tight_layout()
```
````

Here it is rendered:

```{python}
import matplotlib.pyplot as plt

x = [1, 2, 3, 4, 5]
y = [2, 4, 7, 11, 16]

plt.figure(figsize=(8, 4))
plt.plot(x, y, color='#7c3aed', linewidth=2, marker='o')
plt.title('Growth over time')
plt.xlabel('Month')
plt.ylabel('Users (k)')
plt.grid(True, alpha=0.3)
plt.tight_layout()
```

## Bar chart

```{python}
import matplotlib.pyplot as plt

categories = ['Rust', 'Python', 'Go', 'TypeScript']
values = [95, 88, 72, 68]

plt.figure(figsize=(8, 4))
plt.bar(categories, values, color=['#7c3aed', '#06b6d4', '#10b981', '#f59e0b'])
plt.title('Language satisfaction (%)')
plt.ylabel('Satisfaction')
plt.grid(True, alpha=0.3, axis='y')
plt.tight_layout()
```

## Subplots

Use `plt.subplots()` to create multiple plots in one figure:

```{python}
import matplotlib.pyplot as plt
import math

fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(10, 4))

x = [i * 0.1 for i in range(100)]
ax1.plot(x, [math.sin(v) for v in x], color='#7c3aed', linewidth=2)
ax1.set_title('sin(x)')
ax1.grid(True, alpha=0.3)

ax2.plot(x, [math.cos(v) for v in x], color='#06b6d4', linewidth=2)
ax2.set_title('cos(x)')
ax2.grid(True, alpha=0.3)

plt.tight_layout()
```

## How it works

Zorto detects matplotlib figures after your code executes by checking `plt.get_fignums()`. Each figure is saved as a PNG and embedded inline as a base64 data URI. No files are written to disk.

You don't need `plt.show()` — Zorto captures figures automatically. Call `plt.tight_layout()` for best results.

## Related guides

- [Use executable code blocks](executable-code-blocks.md) — setup and general usage
- [Create charts with seaborn](seaborn.md) — statistical plots built on matplotlib
