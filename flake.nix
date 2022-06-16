{
  description = "A daemon to retrieve RTT logs using probe-rs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils }:
    nixpkgs.lib.recursiveUpdate
      (flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ]
        (system:
          let
            pkgs = nixpkgs.legacyPackages.${system};
            cargoToml = nixpkgs.lib.importTOML ./Cargo.toml;
            craneLib = crane.lib.${system};

            commonArgs = {
              src = ./.;
              nativeBuildInputs = with pkgs; [
                pkg-config
              ];
              buildInputs = with pkgs; [
                libusb1
                udev
              ];
            };

            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          in
          rec {
            packages.default = craneLib.buildPackage (commonArgs // {
              inherit cargoArtifacts;
            });

            checks = {
              pkg = packages.default;

              clippy = craneLib.cargoClippy (commonArgs // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "-- --deny warnings";
              });

              rustfmt = craneLib.cargoFmt { src = ./.; };

              nixpkgs-fmt = pkgs.runCommand "nixpkgs-fmt" { } ''
                ${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt --check ${./.}
                touch $out
              '';

              statix = pkgs.runCommand "statix" { } ''
                ${pkgs.statix}/bin/statix check ${./.}
                touch $out
              '';
            };
          }
        ))
      {
        overlays.default = final: prev: {
          rtt-daemon = self.packages.${prev.system}.default;
        };
        nixosModules.default = import ./module.nix;
      };
}
