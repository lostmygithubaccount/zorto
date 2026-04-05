# Create interactive charts with plotly

Use [plotly](https://plotly.com/python/) in executable code blocks to generate interactive charts with hover, zoom, and pan.

## Setup

Add `plotly` to your site's `pyproject.toml`:

```toml
[project]
dependencies = ["plotly"]
```

Then run `uv sync` in your site directory.

## Basic line chart

````markdown
```{python}
import plotly.graph_objects as go

fig = go.Figure()
fig.add_trace(go.Scatter(
    x=[1, 2, 3, 4, 5],
    y=[2, 4, 7, 11, 16],
    mode='lines+markers',
    line=dict(color='#7c3aed', width=3),
))
fig.update_layout(title='Growth over time', height=400)
```
````

Here it is rendered — try hovering over the points:

```{python}
import plotly.graph_objects as go

fig = go.Figure()
fig.add_trace(go.Scatter(
    x=[1, 2, 3, 4, 5],
    y=[2, 4, 7, 11, 16],
    mode='lines+markers',
    line=dict(color='#7c3aed', width=3),
))
fig.update_layout(title='Growth over time', template='plotly_dark', height=400)
```

## Scatter plot

```{python}
import plotly.graph_objects as go
import random

random.seed(42)
x = [random.gauss(0, 1) for _ in range(200)]
y = [xi * 0.7 + random.gauss(0, 0.5) for xi in x]

fig = go.Figure()
fig.add_trace(go.Scatter(
    x=x, y=y, mode='markers',
    marker=dict(color='#7c3aed', size=6, opacity=0.6),
))
fig.update_layout(
    title='Correlation',
    xaxis_title='x', yaxis_title='y',
    template='plotly_dark', height=400,
)
```

## Bar chart

```{python}
import plotly.graph_objects as go

months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun']
revenue = [12, 15, 13, 17, 21, 24]
costs = [10, 11, 12, 13, 14, 15]

fig = go.Figure()
fig.add_trace(go.Bar(x=months, y=revenue, name='Revenue', marker_color='#7c3aed'))
fig.add_trace(go.Bar(x=months, y=costs, name='Costs', marker_color='#06b6d4'))
fig.update_layout(
    title='Monthly financials',
    barmode='group',
    template='plotly_dark', height=400,
)
```

## How it works

Zorto detects `plotly.graph_objects.Figure` instances in your code's local variables after execution. Each figure is converted to standalone HTML (with the plotly.js CDN) and embedded directly in the page.

The output is fully interactive — readers can hover for tooltips, zoom, pan, and export.

## Related guides

- [Use executable code blocks](executable-code-blocks.md) — setup and general usage
- [Create charts with altair](altair.md) — another interactive charting library
