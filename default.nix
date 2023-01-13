{
  pkgs ? import (import ./nix/sources.nix).nixpkgs { config = {}; overlays = []; },
}:
pkgs.rustPlatform.buildRustPackage {
  name = "nix-spp";
  src = pkgs.lib.cleanSource ./.;
  cargoLock.lockFile = ./Cargo.lock;
  passthru.pkgs = pkgs;
  passthru.shell = pkgs.mkShell {
    packages = [
      pkgs.cargo
      pkgs.rust-analyzer
      pkgs.rustc
      pkgs.niv
    ];
  };
}
