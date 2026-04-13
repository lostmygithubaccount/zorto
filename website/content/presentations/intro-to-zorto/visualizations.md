+++
title = "Live visualizations"
weight = 55
+++

{{ slide_image(src="/zorto-mark-transparent.png", alt="Zorto", top="20px", right="20px", width="56px") }}

## Live visualizations at build time

```{python}
# Stdlib only -- a tiny ASCII sparkline of a damped oscillation.
import math

x = [i * 0.4 for i in range(40)]
y = [math.sin(v) * math.exp(-v * 0.12) for v in x]
ramp = " ▁▂▃▄▅▆▇█"
lo, hi = min(y), max(y)
spark = "".join(ramp[round((v - lo) / (hi - lo) * (len(ramp) - 1))] for v in y)

print(f"damped sin(x) * e^(-0.12x), 40 samples")
print(spark)
```

For real charts, install `matplotlib`, `plotly`, `altair`, or `seaborn` -- Zorto captures Figure objects and embeds them as static HTML automatically.
