{
  description = "Memory-safe Rust rewrite of the LDAC Bluetooth audio decoder";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.platforms.littleEndian;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        rec {
          default = ldac-dec;

          # C-ABI drop-in replacement for libldacBT_dec.so.
          ldac-dec = pkgs.rustPlatform.buildRustPackage {
            pname = "ldac-dec";
            version = "0.1.0";

            # The Rust workspace lives in rust/ but build.rs reaches into
            # ../src/ and the capi test compiles against ../inc/, so ship
            # the whole tree.
            src = self;
            sourceRoot = "source/rust";

            cargoLock.lockFile = ./rust/Cargo.lock;

            cargoBuildFlags = [ "--workspace" ];
            cargoTestFlags = [ "--workspace" ];

            # Valgrind/diff tests need external binaries; wire them in
            # so the full test matrix runs in the sandbox rather than
            # skipping.  The diff test dlopens ../build/libldacBT_dec.so
            # so provide the C reference at that path.
            nativeCheckInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
              pkgs.valgrind
            ];
            # Point the diff tests at the C reference built separately
            # so they run for real in the sandbox instead of skipping.
            LDAC_C_REFERENCE = "${ldac-dec-c}/lib/libldacBT_dec.so";

            # Cargo only installs [[bin]] targets by default; install the
            # cdylib + staticlib by hand so this derivation can stand in
            # for nixpkgs#ldacbt as a decode library.
            postInstall = ''
              install -Dm755 target/*/release/libldacBT_dec.so  $out/lib/libldacBT_dec.so
              install -Dm644 target/*/release/libldacBT_dec.a   $out/lib/libldacBT_dec.a
              install -Dm644 ../inc/ldacBT.h                    $out/include/ldac/ldacBT.h
            '';

            meta = {
              description = "Memory-safe LDAC decoder (drop-in libldacBT_dec.so)";
              license = pkgs.lib.licenses.asl20;
              platforms = pkgs.lib.platforms.littleEndian;
            };
          };

          # The C reference decoder, for running the diff test locally.
          ldac-dec-c = pkgs.stdenv.mkDerivation {
            pname = "ldac-dec-c";
            version = "0.1.0";
            src = self;
            nativeBuildInputs = [ pkgs.cmake ];
            installPhase = ''
              install -Dm755 libldacBT_dec.so $out/lib/libldacBT_dec.so
            '';
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.clippy
              pkgs.rust-analyzer
              pkgs.valgrind
              pkgs.cmake
              pkgs.gcc
              pkgs.ldacbt # encoder, for regenerating fixtures
            ];
            # Pre-build the C reference so the diff test picks it up.
            shellHook = ''
              if [ ! -f build/libldacBT_dec.so ]; then
                echo "building C reference decoder for diff tests..."
                cmake -B build >/dev/null && make -C build ldacBT_dec >/dev/null
              fi
            '';
          };
        }
      );

      checks = forAllSystems (system: {
        inherit (self.packages.${system}) ldac-dec;
      });

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-rfc-style);
    };
}
