{
  description = "A daemon to retrieve RTT logs using probe-rs";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        cargoToml = nixpkgs.lib.importTOML ./Cargo.toml;
      in
      rec {
        packages.${cargoToml.package.name} = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          inherit (cargoToml.package) version;

          src = ./.;

          RUSTFLAGS = "-D warnings";

          nativeBuildInputs = with pkgs; [ pkg-config ];

          buildInputs = with pkgs; [
            libusb1
            udev
          ];

          cargoLock.lockFile = ./Cargo.lock;

          doCheck = false;

          meta = with pkgs.lib; {
            inherit (cargoToml.package) description;
            homepage = cargoToml.package.repository;
            license = with licenses; [ mit ];
          };
        };

        packages.default = packages.${cargoToml.package.name};

        devShells.default = packages.default;

        checks = {
          format = pkgs.runCommand "format"
            {
              inherit (packages.default) nativeBuildInputs;
              buildInputs = with pkgs; [ rustfmt cargo ] ++ packages.default.buildInputs;
            } ''
            ${pkgs.rustfmt}/bin/cargo-fmt fmt --manifest-path ${./.}/Cargo.toml -- --check
            ${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt --check ${./.}
            touch $out
          '';

          lint = pkgs.runCommand "lint" { } ''
            ${pkgs.statix}/bin/statix check ${./.}
            touch $out
          '';
        };
      }
    );
}
