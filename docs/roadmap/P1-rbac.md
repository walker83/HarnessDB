# P1: RBAC 角色权限控制

**优先级**: P1
**模块**: fe-catalog, fe-sql-parser, mysql-protocol
**状态**: ❌ 未开始

## 背景

RorisDB 当前仅支持 MySQL 协议认证（用户名/密码），无任何授权机制。所有连接用户拥有全部权限，不符合生产环境要求。

## 任务清单

### 1. 用户管理
- [ ] `CREATE USER` / `DROP USER` / `ALTER USER` 语法
- [ ] 用户信息存储（用户名、密码哈希、默认角色）
- [ ] `GRANT` / `REVOKE` 语法
- [ ] `SET PASSWORD` 语法

### 2. 角色管理
- [ ] `CREATE ROLE` / `DROP ROLE` 语法
- [ ] 角色层级（角色可以包含其他角色）
- [ ] `SET DEFAULT ROLE` / `SET ROLE`
- [ ] 内置角色: admin, public

### 3. 权限模型
- [ ] 定义权限类型: SELECT, INSERT, UPDATE, DELETE, CREATE, DROP, ALTER, ALL
- [ ] 定义权限范围: GLOBAL, DATABASE, TABLE, COLUMN
- [ ] 权限检查点: 在 SQL 执行前验证当前用户权限
- [ ] 行级权限（Row-Level Security，长期目标）
- [ ] 列级权限（控制特定列的访问）

### 4. Catalog 集成
- [ ] 在 Catalog 中持久化用户、角色、权限信息
- [ ] 权限缓存机制（避免每次查询都查权限表）

### 5. 测试
- [ ] 用户创建/删除/修改密码
- [ ] 角色创建/授权/撤销
- [ ] 权限检查（有权限通过、无权限拒绝）
- [ ] 权限继承

## 涉及文件

- `crates/fe-catalog/src/auth.rs` - 新建，用户/角色/权限管理
- `crates/fe-sql-parser/src/parser.rs` - 权限相关 SQL 解析
- `crates/mysql-protocol/src/` - 连接时角色设置
