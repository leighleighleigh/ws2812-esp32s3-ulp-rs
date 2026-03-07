{ pkgs ? import <nixpkgs> {}}:
let 
    esp-rs-src = builtins.fetchGit {
        url = "https://github.com/leighleighleigh/esp-rs-nix";
        # mainline
        rev = "8baa40f096e7f52a10e8438b0bd55ef5dc280164";
    };

    # This will build esp-rs-src, chosen above
    esp-rs = pkgs.callPackage "${esp-rs-src}/esp-rs/default.nix" {
        pkgs = pkgs;
        version = "1.88.0.0"; # Rust version
        crosstool-version = "15.2.0_20251204"; # Cross-compiler toolchain version (GCC)
        binutils-version = "16.3_20250913"; # Binutils version (GDB)
    };
in
pkgs.mkShell rec {
    name = "esp-rs-nix";
  

    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [
        esp-rs 
        #pkgs.espflash
        #pkgs.rust-analyzer
        #pkgs.rustup 
        pkgs.stdenv.cc 
        pkgs.just 
        pkgs.inotify-tools
        pkgs.picocom
        pkgs.libusb1
        # for libudev
        pkgs.systemdMinimal
    ];

    LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";

    shellHook = ''
    # set the shell logline or whatever it's called
    export PS1="''${debian_chroot:+($debian_chroot)}\[\033[01;39m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\W\[\033[00m\]\$ "
    export PS1="(esp-hal-ulp-tests)$PS1"

    # Load shell completions for espflash
    if (which espflash >/dev/null 2>&1); then
    . <(espflash completions $(basename $SHELL))
    fi
    '';
}
