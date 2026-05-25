#!/bin/bash
# RorisDB GitHub Star 获取行动清单
# 按优先级排序，每天执行 1-2 个，避免被视为垃圾信息

set -e

echo "=== RorisDB 社区发布行动清单 ==="
echo ""
echo "当前 GitHub Stars: $(gh api repos/walker83/RorisDB --jq .stargazers_count)"
echo ""
echo "目标：100 stars"
echo ""
echo "所有发布内容已准备在：docs/community-posts.md"
echo ""

cat << 'CHECKLIST'

## 第 1 天：Reddit（最活跃的技术社区）

### r/rust（优先级最高）
URL: https://reddit.com/r/rust/submit
标题: RorisDB: A single-node OLAP database in Rust (Doris SQL compatible)
内容: 从 docs/community-posts.md 的 "r/rust" 部分复制

### r/database
URL: https://reddit.com/r/database/submit
标题: RorisDB: Single-node OLAP database with Doris SQL dialect (Rust)
内容: 从 docs/community-posts.md 的 "r/database" 部分复制

## 第 2 天：中文社区

### V2EX
URL: https://www.v2ex.com/new/tech
标题: [分享] RorisDB: 用 Rust 实现的单节点 OLAP 数据库，兼容 Doris SQL 语法
内容: 从 docs/community-posts.md 的 "V2EX" 部分复制

### 掘金
URL: https://juejin.cn/editor/drafts/new
标题: RorisDB：用 Rust 实现的单节点 OLAP 数据库（兼容 Doris SQL）
内容: 从 docs/community-posts.md 的 "掘金" 部分复制

## 第 3 天：知乎

### 知乎专栏
URL: https://zhuanlan.zhihu.com/write
标题: 如何用 Rust 实现一个兼容 Doris SQL 的 OLAP 数据库？
内容: 从 docs/community-posts.md 的 "知乎" 部分复制

## 第 4 天：Hacker News（需要 careful timing，建议美西时间上午 9-11 点）

### Show HN
URL: https://news.ycombinator.com/submit
标题: Show HN: RorisDB – A single-node OLAP database in Rust (Doris-compatible)
内容: 从 docs/community-posts.md 的 "Show HN" 部分复制

## 第 5 天：社交媒体

### Twitter/X
发 3 条推文（从 docs/community-posts.md 的 "Twitter/X" 部分复制）
记得 @rustlang @apache_doris @DataFusion

### LinkedIn
发布专业帖子（从 docs/community-posts.md 的 "LinkedIn" 部分复制）
添加话题：#Rust #Database #OLAP #OpenSource

## 后续跟进

- 回复所有评论和反馈
- 在 Reddit/知乎上回答相关问题时提到 RorisDB
- 在相关 GitHub issues/discussions 中提及
- 考虑写一篇技术博客发布在 Medium/Dev.to

CHECKLIST

echo ""
echo "=== 执行步骤 ==="
echo ""
echo "1. 打开上面的 URL"
echo "2. 复制对应的标题和内容"
echo "3. 提交发布"
echo "4. 记录发布时间，方便后续跟进"
echo ""
echo "提示："
echo "- 每天发布 1-2 个平台，避免被视为垃圾信息"
echo "- 积极回复评论，增加互动"
echo "- 如果有问题或反馈，及时修复并更新"
echo ""
