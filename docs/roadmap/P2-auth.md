# P2: 外部认证集成

**优先级**: P2
**模块**: mysql-protocol, fe-common
**状态**: ❌ 未开始

## 背景

RorisDB 仅支持 MySQL 原生密码认证。企业环境通常需要集成 LDAP、Kerberos 等统一认证系统。

## 任务清单

### 1. LDAP 认证
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

### 3. Token 认证
- [ ] JWT Token 验证
- [ ] Token 生成和刷新
- [ ] HTTP API 中的 Bearer Token 支持

### 4. 测试
- [ ] LDAP: 使用测试 LDAP Server 验证流程
- [ ] 认证失败场景（密码错误、用户不存在）
- [ ] 性能: 认证对连接建立延迟的影响

## 涉及文件

- `crates/mysql-protocol/src/auth.rs` - 新建/扩展，认证插件
- `crates/fe-common/src/ldap.rs` - 新建，LDAP 客户端
- `crates/fe-catalog/src/auth.rs` - 认证方式存储
