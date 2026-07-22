**Terminal** — a window-chrome card (traffic-light dots + title) wrapping monospace content. Use for command demos, the how-it-works pipeline, and self-host snippets. Color output with the sub-token spans.

```jsx
<Terminal title="~/projects/legacy-app — zsh">
  <Terminal.Comment># which commit shipped python 2.7?{'\n'}</Terminal.Comment>
  <Terminal.Prompt>$</Terminal.Prompt> nxv search python 2.7{'\n'}
  <Terminal.Emph>python27</Terminal.Emph>  2.7.18  <Terminal.Hash>e4a45f9</Terminal.Hash>
</Terminal>
```

Sub-components: `Terminal.Prompt`, `Terminal.Comment`, `Terminal.Hash`, `Terminal.Emph`.
