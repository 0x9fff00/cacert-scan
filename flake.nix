{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  outputs = { self, nixpkgs }:
    let
      forAll = f: nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (s: f nixpkgs.legacyPackages.${s});
    in
    {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "cacert-scan";
          version = "0.1.0";
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./src ./patterns ./scripts/cacert-scan-closure ];
          };
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
          buildInputs = [ pkgs.openssl pkgs.xz pkgs.zstd ];
          # nix-store is left to the caller's PATH so the closure is read from the caller's store
          postInstall = ''
            install -Dm644 -t $out/share/cacert-scan patterns/roots.pem patterns/watch.pem
            wrapProgram $out/bin/cacert-scan \
              --set-default CACERT_SCAN_ROOTS $out/share/cacert-scan/roots.pem \
              --set-default CACERT_SCAN_WATCH $out/share/cacert-scan/watch.pem
            install -Dm755 scripts/cacert-scan-closure $out/bin/cacert-scan-closure
            wrapProgram $out/bin/cacert-scan-closure --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.jq ]}:$out/bin
          '';
          meta.mainProgram = "cacert-scan-closure";
        };
      });
      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc pkg-config openssl curl zstd xz jq python3 ];
        };
      });
    };
}
