{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk = {
      url = "github:nix-community/naersk/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    inputs@{ flake-parts, naersk, nixpkgs, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.flake-parts.flakeModules.easyOverlay ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        { config, pkgs, system, ... }:
        let
          naersk' = pkgs.callPackage naersk { };
        in
        {
          overlayAttrs = { inherit (config.packages) default; };

          packages.default = naersk'.buildPackage {
            pname = "claude-prompt";
            src = ./.;
            nativeBuildInputs = with pkgs; [ git ];
            meta.mainProgram = "claude-prompt";
          };

          devShells.default =
            with pkgs;
            mkShell {
              buildInputs = [
                cargo
                rustc
                rustfmt
                pre-commit
                rustPackages.clippy
                rust-analyzer
              ];
              RUST_SRC_PATH = rustPlatform.rustLibSrc;
            };
        };
    };
}
