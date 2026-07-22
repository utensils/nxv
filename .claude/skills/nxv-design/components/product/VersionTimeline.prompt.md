**VersionTimeline** — horizontal lifespan bars for a package's versions across a shared time axis. Year gridlines + a dashed 2020 "flakes epoch" marker; insecure versions render red. The centrepiece of the history drawer.

```jsx
<VersionTimeline versions={[
  {version:'2.7.18', first:'2020-05-01', last:'2022-01-14'},
  {version:'2.7.17', first:'2019-10-01', last:'2020-04-30', insecure:true},
]} />
```
