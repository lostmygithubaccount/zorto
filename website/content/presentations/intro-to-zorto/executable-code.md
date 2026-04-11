+++
title = "Executable code blocks"
weight = 50
+++

## Executable code blocks

Write Python or Bash in fenced code blocks -- Zorto runs them at build time:

~~~markdown
```{python}
import matplotlib.pyplot as plt
plt.plot([1, 2, 3], [1, 4, 9])
plt.title("Built at build time")
plt.show()
```
~~~

- Output captured and baked into static HTML
- Matplotlib, Plotly, Altair, and Seaborn charts rendered inline
- Results cached by content hash -- only re-runs when code changes
