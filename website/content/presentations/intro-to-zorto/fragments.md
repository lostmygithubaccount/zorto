+++
title = "Progressive reveal"
weight = 65
+++

## Progressive reveal

Press → or space to reveal one at a time:

{% fragment(style="fade-in") %}
- **fade-in**: the default, gentle entrance
{% end %}

{% fragment(style="fade-up") %}
- **fade-up**: slides in from below
{% end %}

{% fragment(style="grow") %}
- **grow**: scales up to emphasize
{% end %}

{% fragment(style="highlight-blue") %}
- **highlight-blue**: flashes a color highlight
{% end %}

{% fragment(style="fade-right") %}
- **fade-right**: slides in from the left
{% end %}

{% speaker_notes() %}
Fragments are Zorto-rendered HTML. The native deck runtime handles the
keyboard-driven reveal sequence.
{% end %}
