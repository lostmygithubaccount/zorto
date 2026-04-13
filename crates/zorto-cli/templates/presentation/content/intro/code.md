+++
title = "Executable code"
weight = 30
+++

## Code that runs at build time

```{python}
# Stdlib only -- no extra installs required.
import statistics

values = [12, 15, 13, 17, 21, 24]
print(f"Mean:  {statistics.mean(values):.2f}")
print(f"Stdev: {statistics.stdev(values):.2f}")
```

The Python above is executed when Zorto builds the site. Its output is captured and embedded as static HTML.

For richer charts, install `plotly` or `matplotlib` and return a Figure.
