#!/bin/bash
# Smart TCP Manager - GitHub Release 发布脚本
# 用法: ./scripts/github-release.sh [版本号]
# 示例: ./scripts/github-release.sh 1.0.0
# 
# 前提条件:
#   - 已安装 gh (GitHub CLI): brew install gh
#   - 已登录: gh auth login

set -e

VERSION="${1:-1.0.0}"
TAG="v$VERSION"
RELEASE_DIR="release"
REPO="LegnaOS/smart-tcp-manager"

echo "=========================================="
echo "  GitHub Release 发布工具"
echo "  版本: $VERSION"
echo "  Tag: $TAG"
echo "=========================================="

# 检查 gh 是否安装
if ! command -v gh &> /dev/null; then
    echo "❌ 错误: 未安装 GitHub CLI (gh)"
    echo "   安装方法: brew install gh"
    exit 1
fi

# 检查是否已登录
if ! gh auth status &> /dev/null; then
    echo "❌ 错误: 未登录 GitHub CLI"
    echo "   请运行: gh auth login"
    exit 1
fi

# 检查发布目录
if [[ ! -d "$RELEASE_DIR" ]]; then
    echo "❌ 错误: 发布目录不存在: $RELEASE_DIR"
    echo "   请先运行: ./scripts/release.sh $VERSION"
    exit 1
fi

# 生成发布说明 (双语)
generate_release_notes() {
    cat << EOF
# Smart TCP Manager $VERSION

## 🎉 Features / 功能

- **i18n Support / 国际化**: Chinese/English interface switching 中英文界面切换
- **Config Persistence / 配置持久化**: Auto-save policies and settings 自动保存策略和设置
- **Windows Connection Control / Windows连接控制**: Close TCP via SetTcpEntry API
- **Process Monitoring / 进程监控**: TCP connection distribution per process 每进程连接状态分布
- **Health Scoring / 健康评分**: Detect problematic processes 检测问题进程
- **Policy Engine / 策略引擎**: App-specific optimization policies 应用级优化策略

## 📦 Downloads / 下载

| Platform / 平台 | File / 文件 |
|-----------------|-------------|
| macOS Intel | \`smart-tcp-manager-${VERSION}-x86_64-apple-darwin.tar.gz\` |
| macOS Apple Silicon | \`smart-tcp-manager-${VERSION}-aarch64-apple-darwin.tar.gz\` |
| Windows 64-bit | \`smart-tcp-manager-${VERSION}-x86_64-pc-windows-gnu.zip\` |

## 🚀 Quick Start / 快速开始

\`\`\`bash
# macOS: Extract and run / 解压并运行
tar -xzf smart-tcp-manager-${VERSION}-*.tar.gz
cd smart-tcp-manager-${VERSION}-*/
./netopt-gui

# Admin required for system settings / 修改系统设置需要管理员权限
sudo ./netopt-service
\`\`\`

## ⚠️ Notes / 注意事项

- Admin privileges required for TCP parameter changes / 修改TCP参数需要管理员权限
- Windows: Run as Administrator to close connections / Windows下需管理员身份运行
- Some settings require system restart / 部分设置需重启系统生效

## 📋 Checksums / 校验和

See \`checksums-sha256.txt\` to verify file integrity / 查看校验和文件验证完整性
EOF
}

# 创建 Git Tag
create_tag() {
    echo ""
    echo "🏷️  创建 Git Tag: $TAG"
    
    if git rev-parse "$TAG" >/dev/null 2>&1; then
        echo "⚠️  Tag $TAG 已存在，跳过创建"
    else
        git tag -a "$TAG" -m "Release $VERSION"
        git push origin "$TAG"
        echo "✅ Tag 创建并推送成功"
    fi
}

# 创建 GitHub Release
create_release() {
    echo ""
    echo "📤 创建 GitHub Release..."
    
    # 生成发布说明到临时文件
    local notes_file=$(mktemp)
    generate_release_notes > "$notes_file"
    
    # 收集所有发布文件
    local files=()
    for f in "$RELEASE_DIR"/*.tar.gz "$RELEASE_DIR"/*.zip "$RELEASE_DIR"/checksums-sha256.txt; do
        if [[ -f "$f" ]]; then
            files+=("$f")
        fi
    done
    
    if [[ ${#files[@]} -eq 0 ]]; then
        echo "❌ 错误: 没有找到发布文件"
        rm "$notes_file"
        exit 1
    fi
    
    echo "📦 上传文件:"
    for f in "${files[@]}"; do
        echo "   - $(basename $f)"
    done
    
    # 创建 Release
    gh release create "$TAG" \
        --repo "$REPO" \
        --title "Smart TCP Manager $VERSION" \
        --notes-file "$notes_file" \
        "${files[@]}"
    
    rm "$notes_file"
    
    echo ""
    echo "✅ GitHub Release 创建成功!"
    echo "🔗 https://github.com/$REPO/releases/tag/$TAG"
}

# 主流程
main() {
    create_tag
    create_release
    
    echo ""
    echo "=========================================="
    echo "  ✅ 发布完成!"
    echo "  版本: $VERSION"
    echo "  链接: https://github.com/$REPO/releases/tag/$TAG"
    echo "=========================================="
}

main

