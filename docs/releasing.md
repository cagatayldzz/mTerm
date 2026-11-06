# Releasing

mTerm has no automated release pipeline yet. A local build is produced with:

```sh
bun run build
```

This writes `dist/mTerm.app` for the current machine, signed ad hoc
(`codesign --sign -`). An ad-hoc signature runs locally only. Distributing the app
to other Macs needs a Developer ID signature and notarization, which are not set up.

Before building a release, run `bun run check`.
