{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rust-analyzer

    # Build tools
    gcc
    clang
    pkg-config
    gnumake
    binutils
    cmake

    # SDL2 for graphics
    SDL2
    SDL2_ttf
    SDL2_image

    # SSL/TLS
    openssl

    # Misc
    python315
  ];

  # Environment variables for building
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.SDL2.dev}/lib/pkgconfig";
  SDL2_PATH = "${pkgs.SDL2}";

  shellHook = ''
    echo "Gugalanna development environment"
    echo "Run 'cargo build' to build the browser"
  '';
}
