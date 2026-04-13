+++
title = "Fragments"
weight = 40

[extra]
background_color = "#0f3460"
+++

## Reveal a piece at a time

Press the right arrow or space to advance.

{% fragment(style="fade-in") %}
- This appears first
{% end %}

{% fragment(style="fade-up") %}
- This slides up from below
{% end %}

{% fragment(style="highlight-blue") %}
- This flashes blue
{% end %}

{% speaker_notes() %}
Press S to open the speaker view. Speaker notes live in a `speaker_notes` shortcode.
{% end %}
