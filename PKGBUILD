pkgname=hyprsnip
pkgver=0.1.0
pkgrel=1
pkgdesc="Fast, predictable screenshot utility for Hyprland with window snapping and rounded corner support"
arch=('x86_64' 'aarch64')
url="https://github.com/MerinPrime/hyprsnip"
license=('MIT' 'Apache-2.0')
depends=('glibc' 'wayland' 'libxkbcommon')
optdepends=(
    'wl-clipboard: for system clipboard copy support (--clipboard, -c)'
    'libnotify: for desktop notifications'
)
makedepends=('cargo')
source=()
sha256sums=()

build() {
    cd "$startdir"
    export RUSTUP_TOOLCHAIN=stable
    cargo build --release --locked
}

check() {
    cd "$startdir"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --release --locked
}

package() {
    cd "$startdir"
    install -Dm755 target/release/hyprsnip "$pkgdir/usr/bin/hyprsnip"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
