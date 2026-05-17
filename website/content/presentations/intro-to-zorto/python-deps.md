+++
title = "Python deps"
weight = 56

[extra]
layout = "wide"
slide_theme = "paper"
+++

## Python deps render artifacts

```{python}
import matplotlib.pyplot as plt
import pandas as pd

df = pd.DataFrame({
    "week": range(1, 9),
    "content": [18, 21, 25, 32, 40, 48, 57, 69],
    "data apps": [2, 3, 3, 5, 8, 13, 21, 34],
})

ax = df.plot(x="week", y=["content", "data apps"], marker="o", linewidth=3, figsize=(7.2, 3.4))
ax.set_title("Generated during the Zorto build")
ax.grid(alpha=0.25)
ax.legend(frameon=False)

print(f"rows: {len(df)}")
print("deps: pandas + matplotlib")
```

The build runs inside the uv-managed site environment. The resulting figure is baked into static HTML.
