#!/bin/bash
# Smart TCP Manager - 多平台编译与发布脚本
# 用法: ./scripts/release.sh [版本号]
# 示例: ./scripts/release.sh 1.0.0

set -e

VERSION="${1:-1.0.0}"
PROJECT_NAME="smart-tcp-manager"
RELEASE_DIR="release"
TARGETS=(
    "x86_64-apple-darwin"      # macOS Intel
    "aarch64-apple-darwin"     # macOS Apple Silicon
    "x86_64-pc-windows-gnu"    # Windows 64-bit
)

echo "=========================================="
echo "  Smart TCP Manager Release Builder"
echo "  Version: $VERSION"
echo "=========================================="

# 创建发布目录
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# 检查并安装交叉编译工具链
check_target() {
    local target=$1
    if ! rustup target list --installed | grep -q "$target"; then
        echo "📦 安装目标平台: $target"
        rustup target add "$target"
    fi
}

# 编译指定平台
build_target() {
    local target=$1
    echo ""
    echo "🔨 编译目标: $target"
    echo "-------------------------------------------"
    
    # 对于 Windows 目标，检查是否有交叉编译器
    if [[ "$target" == *"windows"* ]]; then
        if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
            echo "⚠️  跳过 Windows 编译 (需要安装 mingw-w64)"
            echo "   安装方法: brew install mingw-w64"
            return 1
        fi
    fi
    
    cargo build --release --target "$target" 2>&1 || {
        echo "❌ 编译失败: $target"
        return 1
    }
    
    echo "✅ 编译成功: $target"
    return 0
}

# 打包发布文件
package_release() {
    local target=$1
    local ext=""
    local archive_ext="tar.gz"
    
    # Windows 使用 .exe 和 .zip
    if [[ "$target" == *"windows"* ]]; then
        ext=".exe"
        archive_ext="zip"
    fi
    
    local gui_bin="target/$target/release/netopt-gui$ext"
    local service_bin="target/$target/release/netopt-service$ext"
    
    if [[ ! -f "$gui_bin" ]]; then
        echo "⚠️  未找到编译产物: $gui_bin"
        return 1
    fi
    
    # 创建临时打包目录
    local pkg_dir="$RELEASE_DIR/${PROJECT_NAME}-${VERSION}-${target}"
    mkdir -p "$pkg_dir"
    
    # 复制文件
    cp "$gui_bin" "$pkg_dir/"
    cp "$service_bin" "$pkg_dir/" 2>/dev/null || true
    cp README.md "$pkg_dir/" 2>/dev/null || true
    cp LICENSE "$pkg_dir/" 2>/dev/null || true
    
    # 创建压缩包
    local archive_name="${PROJECT_NAME}-${VERSION}-${target}"
    cd "$RELEASE_DIR"
    
    if [[ "$archive_ext" == "zip" ]]; then
        zip -r "${archive_name}.zip" "$(basename $pkg_dir)"
    else
        tar -czvf "${archive_name}.tar.gz" "$(basename $pkg_dir)"
    fi
    
    cd ..
    rm -rf "$pkg_dir"
    
    echo "📦 打包完成: $RELEASE_DIR/${archive_name}.${archive_ext}"
}

# 生成 SHA256 校验和
generate_checksums() {
    echo ""
    echo "🔐 生成校验和..."
    cd "$RELEASE_DIR"
    shasum -a 256 *.tar.gz *.zip 2>/dev/null > checksums-sha256.txt || true
    cd ..
    echo "✅ 校验和已保存: $RELEASE_DIR/checksums-sha256.txt"
}

# 主流程
main() {
    echo ""
    echo "📋 目标平台:"
    for target in "${TARGETS[@]}"; do
        echo "   - $target"
    done
    echo ""
    
    # 检查目标平台
    for target in "${TARGETS[@]}"; do
        check_target "$target"
    done
    
    # 编译并打包
    for target in "${TARGETS[@]}"; do
        if build_target "$target"; then
            package_release "$target"
        fi
    done
    
    # 生成校验和
    generate_checksums
    
    echo ""
    echo "=========================================="
    echo "  ✅ 发布构建完成!"
    echo "  版本: $VERSION"
    echo "  输出目录: $RELEASE_DIR/"
    echo "=========================================="
    ls -la "$RELEASE_DIR/"
}

main

