+++
title = "Executable visualizations"
date = "2026-04-04"
author = "Cody"
description = "Zorto now renders Python visualizations inline — matplotlib, plotly, seaborn, altair. Just write the code."
tags = ["zorto", "python", "visualizations"]
+++

Zorto can now render Python visualizations inline. No configuration, no magic comments — just write your code and the chart appears.

<!-- more -->

## Matplotlib

The most popular Python plotting library. Just `import matplotlib.pyplot as plt` and plot:

```{python}
import matplotlib.pyplot as plt
import math

x = [i * 0.1 for i in range(100)]
y = [math.sin(v) for v in x]

plt.figure(figsize=(8, 4))
plt.plot(x, y, color='#7c3aed', linewidth=2)
plt.title('sin(x)')
plt.xlabel('x')
plt.ylabel('y')
plt.grid(True, alpha=0.3)
plt.tight_layout()
```

## Plotly

Interactive charts that respond to hover, zoom, and pan:

```{python}
import plotly.graph_objects as go

months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
revenue = [12, 15, 13, 17, 21, 24, 22, 28, 31, 29, 35, 42]
costs = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21]

fig = go.Figure()
fig.add_trace(go.Scatter(x=months, y=revenue, name='Revenue', line=dict(color='#7c3aed', width=3)))
fig.add_trace(go.Scatter(x=months, y=costs, name='Costs', line=dict(color='#06b6d4', width=3)))
fig.update_layout(
    title='Monthly financials',
    xaxis_title='Month',
    yaxis_title='$K',
    template='plotly_dark',
    height=400,
)
```

## Seaborn

Statistical visualizations built on matplotlib — automatically captured:

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

## Altair

Declarative statistical visualization:

```{python}
import altair as alt

data = alt.Data(values=[
    {'x': i, 'y': i ** 2, 'category': 'quadratic'} for i in range(20)
] + [
    {'x': i, 'y': i * 3, 'category': 'linear'} for i in range(20)
])

chart = alt.Chart(data).mark_line(strokeWidth=3).encode(
    x='x:Q',
    y='y:Q',
    color=alt.Color('category:N', scale=alt.Scale(range=['#7c3aed', '#06b6d4'])),
).properties(
    title='Growth curves',
    width=600,
    height=300,
)
```

## Multiple plots in one block

```{python}
import matplotlib.pyplot as plt

fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(10, 4))

categories = ['Rust', 'Python', 'Go', 'JS']
values = [95, 88, 72, 68]
ax1.bar(categories, values, color=['#7c3aed', '#06b6d4', '#10b981', '#f59e0b'])
ax1.set_title('Language satisfaction')
ax1.set_ylabel('%')

import random
random.seed(42)
x = [random.gauss(0, 1) for _ in range(100)]
y = [xi * 0.7 + random.gauss(0, 0.5) for xi in x]
ax2.scatter(x, y, alpha=0.6, color='#7c3aed', s=20)
ax2.set_title('Correlation')
ax2.set_xlabel('x')
ax2.set_ylabel('y')

plt.tight_layout()
```

## How it works

Zorto's executable code blocks run Python at build time via an embedded interpreter. After your code executes, Zorto checks if any visualization libraries produced output:

- **matplotlib**: Detects open figures via `plt.get_fignums()`, saves as PNG, embeds inline
- **plotly**: Detects `plotly.graph_objects.Figure` instances, embeds as interactive HTML
- **seaborn**: Uses matplotlib under the hood — automatically captured
- **altair**: Detects `altair.Chart` instances, embeds as interactive HTML

Zero configuration. Zero magic comments. Just write Python.
