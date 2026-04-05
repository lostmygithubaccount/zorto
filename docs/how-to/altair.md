# Create charts with altair

Use [Altair](https://altair-viz.github.io/) in executable code blocks to create declarative statistical visualizations that render as interactive HTML.

## Setup

Add `altair` to your site's `pyproject.toml`:

```toml
[project]
dependencies = ["altair"]
```

Then run `uv sync` in your site directory.

## Basic chart

````markdown
```{python}
import altair as alt

data = alt.Data(values=[
    {'x': i, 'y': i ** 2} for i in range(20)
])

chart = alt.Chart(data).mark_line(strokeWidth=3).encode(
    x='x:Q',
    y='y:Q',
).properties(title='Quadratic growth', width=600, height=300)
```
````

Here it is rendered:

```{python}
import altair as alt

data = alt.Data(values=[
    {'x': i, 'y': i ** 2} for i in range(20)
])

chart = alt.Chart(data).mark_line(
    strokeWidth=3, color='#7c3aed'
).encode(
    x='x:Q',
    y='y:Q',
).properties(title='Quadratic growth', width=600, height=300)
```

## Layered chart

Combine multiple marks by layering charts with `+`:

```{python}
import altair as alt
import math

data = alt.Data(values=[
    {'x': i * 0.1, 'sin': math.sin(i * 0.1), 'cos': math.cos(i * 0.1)}
    for i in range(100)
])

sin_line = alt.Chart(data).mark_line(strokeWidth=2, color='#7c3aed').encode(
    x='x:Q', y='sin:Q',
)
cos_line = alt.Chart(data).mark_line(strokeWidth=2, color='#06b6d4').encode(
    x='x:Q', y='cos:Q',
)

chart = (sin_line + cos_line).properties(
    title='Trigonometric functions', width=600, height=300,
)
```

## Interactive selection

Altair supports interactive selections — click and drag to highlight:

```{python}
import altair as alt
import random

random.seed(42)
data = alt.Data(values=[
    {'x': random.gauss(0, 1), 'y': random.gauss(0, 1), 'group': 'A' if random.random() > 0.5 else 'B'}
    for _ in range(200)
])

selection = alt.selection_point(fields=['group'])

chart = alt.Chart(data).mark_circle(size=60).encode(
    x='x:Q',
    y='y:Q',
    color=alt.condition(selection, 'group:N', alt.value('lightgray'),
        scale=alt.Scale(range=['#7c3aed', '#06b6d4'])),
    opacity=alt.condition(selection, alt.value(0.8), alt.value(0.2)),
).add_params(selection).properties(
    title='Click a legend item to filter', width=600, height=300,
)
```

## How it works

Zorto detects `altair.Chart` instances (and `LayerChart`, `HChart`, `VChart`, `ConcatChart`) in your code's local variables after execution. Each chart is converted to standalone HTML using Vega-Embed and embedded directly in the page.

The output is interactive — readers can hover for tooltips and interact with selections.

## Related guides

- [Use executable code blocks](executable-code-blocks.md) — setup and general usage
- [Create interactive charts with plotly](plotly.md) — another interactive charting library
