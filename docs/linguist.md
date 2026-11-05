<!--
Copyright (c) 2026 AHMAD ALI PARR / SnapKitty Research Lab.
Licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2).
See LICENSE file for full terms. After 2 years from release, converts to Apache-2.0.
-->

# Minikran on GitHub (Linguist)

GitHub's language bar comes from [github-linguist](https://github.com/github-linguist/linguist).
`linguist-language=Minikran` in `.gitattributes` is ignored today because Linguist only accepts
language names listed in its `languages.yml`.

## Getting "Minikran" recognised

Open a PR to github-linguist that adds:

1. An entry in `lib/linguist/languages.yml` (name, type `programming`, extension `.mkr`,
   `tm_scope: source.minikran`, color).
2. The grammar: `grammars/minikran.tmLanguage.json` from this repo (scope `source.minikran`),
   added via `script/add-grammar` pointing at a public repo with a compatible license.
3. Real-world samples under `samples/Minikran/` (e.g. `example.mkr`).
4. Proof of usage: Linguist generally requires the extension to be in use in roughly
   **200+ unique repositories** (not forks, not all by one user), shown via a GitHub code search.

Until then, `.mkr` files do not appear in the language bar.

## VS Code extension

```bash
cd editors/vscode
npx @vscode/vsce package      # produces minikran-0.1.0.vsix
code --install-extension minikran-0.1.0.vsix
```

Or copy `editors/vscode` into `~/.vscode/extensions/minikran`. Files ending in `.mkr` then get
Minikran highlighting.
