# Create charts with seaborn

Use [seaborn](https://seaborn.pydata.org/) in executable code blocks to create statistical visualizations. Seaborn builds on matplotlib, so charts render as inline PNGs.

## Setup

Add `seaborn` and `matplotlib` to your site's `pyproject.toml`:

```toml
[project]
dependencies = ["seaborn", "matplotlib"]
```

Then run `uv sync` in your site directory.

## Distribution plot

````markdown
```{python}
import matplotlib
matplotlib.use('Agg')
import seaborn as sns
import matplotlib.pyplot as plt
import random

random.seed(42)
data = [random.gauss(0, 1) for _ in range(500)]

plt.figure(figsize=(8, 4))
sns.histplot(data, bins=30, kde=True, color='#7c3aed', alpha=0.7)
plt.title('Normal distribution (n=500)')
plt.tight_layout()
```
````

Here it is rendered:

```{python}
import matplotlib
matplotlib.use('Agg')
import seaborn as sns
import matplotlib.pyplot as plt
import random

random.seed(42)
data = [random.gauss(0, 1) for _ in range(500)]

plt.figure(figsize=(8, 4))
sns.histplot(data, bins=30, kde=True, color='#7c3aed', alpha=0.7, edgecolor='white')
plt.title('Normal distribution (n=500)')
plt.xlabel('Value')
plt.ylabel('Frequency')
plt.grid(True, alpha=0.3, axis='y')
plt.tight_layout()
```

## Box plot

```{python}
import matplotlib
matplotlib.use('Agg')
import seaborn as sns
import matplotlib.pyplot as plt
import random

random.seed(42)
data = {
    'Language': ['Rust'] * 50 + ['Python'] * 50 + ['Go'] * 50,
    'Build time (s)': (
        [random.gauss(2, 0.5) for _ in range(50)] +
        [random.gauss(5, 1.5) for _ in range(50)] +
        [random.gauss(3, 0.8) for _ in range(50)]
    ),
}

plt.figure(figsize=(8, 4))
sns.boxplot(x='Language', y='Build time (s)', data=data, palette=['#7c3aed', '#06b6d4', '#10b981'])
plt.title('Build times by language')
plt.grid(True, alpha=0.3, axis='y')
plt.tight_layout()
```

## Heatmap

```{python}
import matplotlib
matplotlib.use('Agg')
import seaborn as sns
import matplotlib.pyplot as plt
import random

random.seed(42)
matrix = [[random.random() for _ in range(6)] for _ in range(6)]
labels = ['A', 'B', 'C', 'D', 'E', 'F']

plt.figure(figsize=(7, 5))
sns.heatmap(matrix, annot=True, fmt='.2f', xticklabels=labels, yticklabels=labels, cmap='viridis')
plt.title('Correlation matrix')
plt.tight_layout()
```

## How it works

Seaborn creates matplotlib figures under the hood, so Zorto captures them exactly the same way — by detecting open figures via `plt.get_fignums()` and saving them as inline PNGs.

Use `matplotlib.use('Agg')` before importing seaborn to ensure the non-interactive backend is used during builds.

## Related guides

- [Use executable code blocks](executable-code-blocks.md) — setup and general usage
- [Create charts with matplotlib](matplotlib.md) — lower-level matplotlib usage
