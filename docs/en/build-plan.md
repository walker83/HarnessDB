# RorisDB Build and Packaging Plan

## Project Overview

RorisDB is a real-time OLAP database reimplemented in Rust, adopting an MPP architecture with FE (Frontend) and BE (Backend) components.

### Technology Stack
- **Language**: Rust (2024 Edition)
- **Build Tool**: Cargo
- **Binaries**:
  - `roris-fe` - Frontend Server (Catalog management, metadata service)
  - `roris-be` - Backend Server (Storage engine, execution engine)
  - `roris-cli` - Command-line client

### Key Dependencies
- tokio (async runtime)
- tonic (gRPC)
- rocksdb (storage engine)
- sqlparser (SQL parsing)

## Build Plan

### Environment Requirements

**Mac System Dependencies**:
```bash
brew install cmake
brew install clang
```

**Rust Environment**:
```bash
rustc --version  # Requires 2024 edition support
cargo --version
```

### Build Commands

**Development Build** (for debugging):
```bash
cargo build
```

**Release Build** (for distribution):
```bash
cargo build --release
```

Build artifacts are located at: `target/release/`

### Build Verification

```bash
# Verify binaries
./target/release/roris-fe --help
./target/release/roris-be --help
./target/release/roris-cli --help
```

## Packaging Plan

### Solution Comparison

| Feature | tar.gz | DMG |
|---------|--------|-----|
| Use Case | Server deployment, CLI tools | GUI applications, general users |
| Installation | Extract to any directory | Drag to Applications |
| Flexibility | Can be placed anywhere | Usually fixed location |
| User Habit | Familiar to devs/ops | More familiar to Mac users |
| Launch Method | Command-line launch | Can create shortcuts |

### Recommended Solution

**Primary: tar.gz** (recommended for database services):
- Suitable for server deployment
- Users can install to `/usr/local/rorisdb` or custom paths
- Simple and direct, aligns with backend service distribution conventions

**Optional: DMG** (as supplement):
- If you want a "more Mac-like" experience
- Can include binaries and quick start instructions in the DMG

### Package Directory Structure

```
rorisdb-v0.1.2-macos/
├── bin/
│   ├── roris-fe
│   ├── roris-be
│   └── roris-cli
├── conf/
│   ├── fe.conf
│   └── be.conf
├── scripts/
│   ├── start-fe.sh
│   ├── start-be.sh
│   ├── stop-all.sh
│   └── install.sh
└── README.md
```

### Packaging Scripts

**build-release.sh** - Build script:
```bash
#!/bin/bash
set -e

echo "=== RorisDB Release Build ==="

# Check environment
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust."
    exit 1
fi

# Build
echo "Building release binaries..."
cargo build --release

# Verify
echo "Verifying binaries..."
ls -lh target/release/roris-fe target/release/roris-be target/release/roris-cli

echo "Build complete!"
```

**package-macos.sh** - Packaging script:
```bash
#!/bin/bash
set -e

VERSION="0.1.2"
PACKAGE_NAME="rorisdb-v${VERSION}-macos"
TARGET_DIR="target/release"

echo "=== Packaging RorisDB for macOS ==="

# Create temporary directory
rm -rf "${PACKAGE_NAME}"
mkdir -p "${PACKAGE_NAME}/bin"
mkdir -p "${PACKAGE_NAME}/conf"
mkdir -p "${PACKAGE_NAME}/scripts"

# Copy binaries
cp "${TARGET_DIR}/roris-fe" "${PACKAGE_NAME}/bin/"
cp "${TARGET_DIR}/roris-be" "${PACKAGE_NAME}/bin/"
cp "${TARGET_DIR}/roris-cli" "${PACKAGE_NAME}/bin/"

# Copy configuration files
cp conf/fe.conf "${PACKAGE_NAME}/conf/"
cp conf/be.conf "${PACKAGE_NAME}/conf/"

# Create startup scripts
cat > "${PACKAGE_NAME}/scripts/start-fe.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.."
./bin/roris-fe --http-port 8030 --rpc-port 9020
EOF

cat > "${PACKAGE_NAME}/scripts/start-be.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")/.."
./bin/roris-be --http-port 8060 --rpc-port 9060
EOF

cat > "${PACKAGE_NAME}/scripts/stop-all.sh" << 'EOF'
#!/bin/bash
pkill -f roris-fe || true
pkill -f roris-be || true
EOF

# Create install script
cat > "${PACKAGE_NAME}/scripts/install.sh" << 'EOF'
#!/bin/bash
set -e
PREFIX="${1:-/usr/local/rorisdb}"
mkdir -p "${PREFIX}"
cp -r bin conf "${PREFIX}/"
ln -sf "${PREFIX}/bin/roris-cli" /usr/local/bin/roris-cli || true
echo "Installed to ${PREFIX}"
EOF

# Set permissions
chmod +x "${PACKAGE_NAME}/bin/"*
chmod +x "${PACKAGE_NAME}/scripts/"*

# Package
tar -czf "${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"

echo "Package created: ${PACKAGE_NAME}.tar.gz"
```

## Usage Instructions

### Installation

```bash
# Extract
tar -xzf rorisdb-v0.1.2-macos.tar.gz
cd rorisdb-v0.1.2-macos

# Install to system directory (optional)
sudo ./scripts/install.sh

# Or specify installation path
./scripts/install.sh ~/rorisdb
```

### Starting Services

```bash
# Start FE
./scripts/start-fe.sh &

# Start BE
./scripts/start-be.sh &

# Connect using CLI
./bin/roris-cli
```

### Stopping Services

```bash
./scripts/stop-all.sh
```

## Reference Implementations

- **MySQL/PostgreSQL**: Provide tar.gz, also offer DMG installers
- **Redis**: Primarily provide tar.gz
- **ClickHouse**: Provide tgz and deb/rpm, also have Mac tap

## Future Optimizations

1. Add `build.rs` support for version information injection
2. Consider using `cargo-bundle` or `cargo-packager` tools to simplify packaging
3. Provide Homebrew tap support
4. Add code signing (if distributing to general users)
