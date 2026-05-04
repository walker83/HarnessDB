# RorisDB 编译与打包方案

## 项目概述

RorisDB 是一个使用 Rust 重新实现的实时 OLAP 数据库，采用 MPP 架构，分为 FE（前端）和 BE（后端）组件。

### 技术栈
- **语言**: Rust (2024 Edition)
- **构建工具**: Cargo
- **二进制文件**:
  - `roris-fe` - Frontend Server (Catalog 管理、元数据服务)
  - `roris-be` - Backend Server (存储引擎、执行引擎)
  - `roris-cli` - 命令行客户端

### 关键依赖
- tokio (异步运行时)
- tonic (gRPC)
- rocksdb (存储引擎)
- sqlparser (SQL 解析)

## 编译方案

### 环境要求

**Mac 系统依赖**:
```bash
brew install cmake
brew install clang
```

**Rust 环境**:
```bash
rustc --version  # 需要 2024 edition 支持
cargo --version
```

### 编译命令

**开发构建** (调试用):
```bash
cargo build
```

**发布构建** (分发用):
```bash
cargo build --release
```

编译产物位于: `target/release/`

### 编译验证

```bash
# 验证二进制文件
./target/release/roris-fe --help
./target/release/roris-be --help
./target/release/roris-cli --help
```

## 打包方案

### 方案对比

| 特性 | tar.gz | DMG |
|------|--------|-----|
| 适用场景 | 服务器部署、命令行工具 | GUI 应用、普通用户 |
| 安装方式 | 解压到任意目录 | 拖拽到 Applications |
| 灵活性 | 可放任意路径 | 通常固定位置 |
| 用户习惯 | 开发者/运维熟悉 | Mac 用户更熟悉 |
| 启动方式 | 命令行启动 | 可创建快捷方式 |

### 推荐方案

**主要提供 tar.gz** (推荐用于数据库服务):
- 适合服务器部署
- 用户可以放到 `/usr/local/rovisdb` 或自定义路径
- 简单直接，符合后端服务的分发习惯

**可选提供 DMG** (作为补充):
- 如果希望"更 Mac 化"
- 可以在 DMG 中包含二进制文件和快速启动说明

### 打包目录结构

```
rovisdb-v0.1.2-macos/
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

### 打包脚本

**build-release.sh** - 编译脚本:
```bash
#!/bin/bash
set -e

echo "=== RorisDB Release Build ==="

# 检查环境
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust."
    exit 1
fi

# 编译
echo "Building release binaries..."
cargo build --release

# 验证
echo "Verifying binaries..."
ls -lh target/release/roris-fe target/release/roris-be target/release/roris-cli

echo "Build complete!"
```

**package-macos.sh** - 打包脚本:
```bash
#!/bin/bash
set -e

VERSION="0.1.2"
PACKAGE_NAME="rovisdb-v${VERSION}-macos"
TARGET_DIR="target/release"

echo "=== Packaging RorisDB for macOS ==="

# 创建临时目录
rm -rf "${PACKAGE_NAME}"
mkdir -p "${PACKAGE_NAME}/bin"
mkdir -p "${PACKAGE_NAME}/conf"
mkdir -p "${PACKAGE_NAME}/scripts"

# 复制二进制文件
cp "${TARGET_DIR}/roris-fe" "${PACKAGE_NAME}/bin/"
cp "${TARGET_DIR}/roris-be" "${PACKAGE_NAME}/bin/"
cp "${TARGET_DIR}/roris-cli" "${PACKAGE_NAME}/bin/"

# 复制配置文件
cp conf/fe.conf "${PACKAGE_NAME}/conf/"
cp conf/be.conf "${PACKAGE_NAME}/conf/"

# 创建启动脚本
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

# 创建安装脚本
cat > "${PACKAGE_NAME}/scripts/install.sh" << 'EOF'
#!/bin/bash
set -e
PREFIX="${1:-/usr/local/rovisdb}"
mkdir -p "${PREFIX}"
cp -r bin conf "${PREFIX}/"
ln -sf "${PREFIX}/bin/roris-cli" /usr/local/bin/roris-cli || true
echo "Installed to ${PREFIX}"
EOF

# 设置权限
chmod +x "${PACKAGE_NAME}/bin/"*
chmod +x "${PACKAGE_NAME}/scripts/"*

# 打包
tar -czf "${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"

echo "Package created: ${PACKAGE_NAME}.tar.gz"
```

## 使用说明

### 安装

```bash
# 解压
tar -xzf rovisdb-v0.1.2-macos.tar.gz
cd rovisdb-v0.1.2-macos

# 安装到系统目录（可选）
sudo ./scripts/install.sh

# 或指定安装路径
./scripts/install.sh ~/rovisdb
```

### 启动服务

```bash
# 启动 FE
./scripts/start-fe.sh &

# 启动 BE
./scripts/start-be.sh &

# 使用 CLI 连接
./bin/roris-cli
```

### 停止服务

```bash
./scripts/stop-all.sh
```

## 典型做法参考

- **MySQL/PostgreSQL**: 提供 tar.gz，也有 DMG 安装包
- **Redis**: 主要提供 tar.gz
- **ClickHouse**: 提供 tgz 和 deb/rpm，也有 Mac 的 tap

## 后续优化

1. 添加 `build.rs` 支持版本信息注入
2. 考虑使用 `cargo-bundle` 或 `cargo-packager` 工具简化打包
3. 提供 Homebrew tap 支持
4. 添加代码签名（如果需要分发到普通用户）
