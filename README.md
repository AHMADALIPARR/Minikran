# Minikran

Lean 4 library occupying the `Minikran` namespace, depending on Mathlib so that `import Mathlib.Tactic.Core` resolves correctly.

**License:** Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2). Converts to Apache-2.0 two years after each release.

## Setup

```bash
lake update
lake exe cache get
lake build
```

Pinned to Mathlib `v4.36.0-rc2` matching the toolchain.

## Custom Syntax

`.mkr` files use the Minikran custom syntax (see `example.mkr`). GitHub Linguist is instructed via `.gitattributes` to treat them as the `Minikran` language.

## Eclipse

Includes `.project` and `.classpath` for Eclipse import.

# LEAN_OK — no errors
