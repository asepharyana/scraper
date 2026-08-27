{
  description = "Asepharyana Scraper — Rust/Axum web scraper & image proxy API";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        scraper = pkgs.stdenv.mkDerivation {
          name = "scraper-0.1.0";
          src = ./.;

          nativeBuildInputs = with pkgs; [
            cacert curl gcc gnumake openssl pkg-config python3 libclang
            rustc cargo clang cmake zlib
          ];
          buildInputs = with pkgs; [ openssl stdenv.cc.cc.lib ];

          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          LD_LIBRARY_PATH = "${pkgs.libclang.lib}/lib:${pkgs.stdenv.cc.cc.lib}/lib";
          NIX_ENFORCE_PURITY = "0";

          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";

          phases = [ "unpackPhase" "buildPhase" "installPhase" ];
          buildPhase = ''
            export HOME="$TMPDIR" CARGO_HOME="$TMPDIR/.cargo-scraper"
            echo "=== Building scraper ==="
            cargo build --release 2>&1
          '';
          installPhase = ''
            mkdir -p $out/bin
            cp target/release/scraper $out/bin/scraper
          '';
        };
      in
      {
        packages = {
          inherit scraper;
          default = scraper;
        };

        apps.scraper = {
          type = "app";
          program = "${scraper}/bin/scraper";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ rustc cargo clang cmake openssl pkg-config ];
        };
      });
}