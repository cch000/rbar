{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    platforms = nixpkgs.lib.platforms.all;
    forAllplatforms = nixpkgs.lib.genAttrs platforms;

    name = "rbar";

    mkInputs = pkgs: {
      nativeBuildInputs = with pkgs; [
        pkg-config
        clang
        rustPlatform.bindgenHook
      ];

      buildInputs = with pkgs; [
        gtk4-layer-shell
        gtk4
      ];
    };
  in {
    packages = forAllplatforms (
      platform: let
        pkgs = nixpkgs.legacyPackages.${platform};
        inputs = mkInputs pkgs;

        rbar = pkgs.rustPlatform.buildRustPackage {
          inherit (inputs) nativeBuildInputs buildInputs;
          inherit name;

          version = "0.1.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          meta.mainProgram = name;
        };
      in {
        inherit rbar;
        default = rbar;
      }
    );

    devShells = forAllplatforms (
      platform: let
        pkgs = nixpkgs.legacyPackages.${platform};
        inputs = mkInputs pkgs;
      in {
        default = pkgs.mkShell {
          inherit (inputs) nativeBuildInputs buildInputs;
          inputsFrom = [self.packages.${platform}.default];
          packages = with pkgs; [
            nixd
            alejandra
            rustfmt
            clippy
            cargo
            rust-analyzer
            rustc
          ];
        };
      }
    );
  };
}
