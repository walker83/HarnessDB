# P2: 外部认证集成

**优先级**: P2
**模块**: mysql-protocol, fe-common, fe-catalog, fe-sql-parser
**状态**: 🟢 进行中

## 背景

RorisDB 仅支持 MySQL 原生密码认证。企业环境通常需要集成 LDAP、Kerberos 等统一认证系统。

## 已完成

### 1. 认证插件接口 ✅
- [x] `crates/mysql-protocol/src/auth/plugin.rs` - AuthPlugin trait 和 AuthError
- [x] `crates/mysql-protocol/src/auth/native_password.rs` - NativePasswordAuth 实现
- [x] `crates/mysql-protocol/src/auth/token.rs` - TokenAuth/JWT 实现
- [x] `crates/mysql-protocol/src/auth/mod.rs` - 模块导出

### 2. Token/JWT 认证 ✅
- [x] JWT Claims 结构 (username, roles, exp, iat, iss)
- [x] `generate_jwt_token()` - 生成签名 JWT
- [x] `validate_jwt_token()` - 验证 JWT 有效性
- [x] Token 过期检查
- [x] `crates/fe-common/src/token.rs` - fe-common 中的 JWT 工具

### 3. 用户认证存储 ✅
- [x] `crates/fe-catalog/src/auth.rs` - AuthManager 用户管理
- [x] UserAuth 结构 (username, auth_plugin, password_hash, roles)
- [x] create_user / drop_user / get_user 操作
- [x] 角色 grant/revoke
- [x] 持久化到 `users.json`

### 4. SQL Parser 扩展 ✅
- [x] `CREATE USER ... IDENTIFIED BY 'password'` 语法
- [x] `CREATE USER ... IDENTIFIED WITH auth_plugin` 语法
- [x] `DROP USER ...` 语法
- [x] `SHOW USERS` 语法

### 5. 连接集成 ✅
- [x] `crates/mysql-protocol/src/connection.rs` - 认证流程
- [x] 支持 mysql_native_password 和 auth_token 插件
- [x] 认证失败返回 Error 1045

## 未完成

### 1. LDAP 认证 ⏳
- [ ] `CREATE USER ... IDENTIFIED WITH ldap` 语法
- [ ] LDAP 连接配置（URL、Base DN、Bind DN）
- [ ] 认证流程: FE 代理用户凭据到 LDAP Server 验证
- [ ] LDAP 组 → RorisDB 角色映射
- [ ] 连接池管理（避免每次查询都连接 LDAP）

### 2. Kerberos 认证（长期）
- [ ] Kerberos 配置管理
- [ ] 支持 GSSAPI 认证流程
- [ ] Keytab 文件管理
- [ ] Ticket 刷新

### 3. Planner 集成
- [ ] 实现 `plan_create_user` - 实际创建用户到 AuthManager
- [ ] 实现 `plan_drop_user` - 从 AuthManager 删除用户
- [ ] 实现 `plan_show_users` - 查询用户列表

### 4. 测试
- [ ] LDAP: 使用测试 LDAP Server 验证流程
- [ ] 认证失败场景（密码错误、用户不存在）
- [ ] 性能: 认证对连接建立延迟的影响

## 涉及文件

- `crates/mysql-protocol/src/auth.rs` - 认证插件 ✓
- `crates/fe-common/src/token.rs` - JWT 工具 ✓
- `crates/fe-catalog/src/auth.rs` - 用户存储 ✓
- `crates/fe-sql-parser/src/parser.rs` - SQL 解析 ✓
- `crates/fe-sql-planner/src/planner.rs` - Planner stub ✓

## 使用示例

```sql
-- 创建使用 native password 的用户
CREATE USER 'testuser'@'%' IDENTIFIED BY 'password123';

-- 创建使用 token 认证的用户
CREATE USER 'api_user'@'%' IDENTIFIED WITH auth_token;

-- 删除用户
DROP USER 'testuser'@'%';

-- 查看用户
SHOW USERS;
```