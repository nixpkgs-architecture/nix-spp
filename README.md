
## Implementation

- [x] Index the tree for references. If `.git` exists, use `ls-tree` equivalent
- [ ] Check the validity of the `pkgs/unit` directory, https://github.com/nixpkgs-architecture/rfcs/blob/master/rfcs/0140-simple-package-paths.md#detailed-design
  - Structure: Check not needed, we can check that in Nix
  - [ ] Only derivations
  - [ ] Stable boundary: Shouldn't reference files outside, neither be referenced from outside
  - [ ] Custom arguments: `all-packages.nix` can reference unit directories in a limited way

- [x] Loop through all definitions in all-packages.nix
  - [ ] If the definition could be migrated:
    - [ ] If `--mode=migrate`, migrate the code, output a message
    - [ ] If `--mode=warn`, output a warning message only
      - If in GitHub Actions, create a code annotation
    - [ ] If `--mode=error`, output an error message, fail at the end
      - If in GitHub Actions, create a code annotation

Perhaps create code annotations only when they're in a file that's changed by the PR
