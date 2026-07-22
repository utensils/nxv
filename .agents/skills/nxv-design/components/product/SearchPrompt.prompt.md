**SearchPrompt** — nxv's signature terminal-style search bar. Prompt sigil + command word + input + blinking caret, with a focus-ring lift. The hero's centrepiece.

```jsx
<SearchPrompt value={q} onChange={setQ} onSubmit={run} />
<SearchPrompt command="info" placeholder="python311" button />
```

Set `button` to swap the caret for an inline run button. `command`/`host` customize the prompt (`nxv:~$ search`).
