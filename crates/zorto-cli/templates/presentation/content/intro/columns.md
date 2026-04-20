+++
title = "Columns"
weight = 50
+++

## Side-by-side content

{% columns(widths="50%|50%") %}

### Left

- Use the `columns` shortcode
- Split content with `&lt;!-- column --&gt;`

<!-- column -->

### Right

- Set `widths="60%|40%"` for uneven columns
- Or omit `widths` for equal flex columns

{% end %}
