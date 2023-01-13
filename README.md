
## Implementation

- Index the tree for references. If `.git` exists, use `ls-tree` equivalent
- Check the validity of the `pkgs/unit` directory
  - Should only contain correctly-named subdirectories
  - Shouldn't reference files outside, neither be referenced from outside
- all-packages.nix, aliases.nix and unit directories are non-intersecting
- Loop through all definitions in all-packages.nix
  For all definitions that could be migrated:
  - If `--mode=migrate`, migrate the code, output a message
  - If `--mode=warn`, output a warning message only
    - If in GitHub Actions, create a code annotation
  - If `--mode=error`, output an error message, fail at the end
    - If in GitHub Actions, create a code annotation

Perhaps create code annotations only when they're in a file that's changed by the PR
