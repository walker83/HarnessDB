# 安全模块 - 用户和权限

## 概述
当前用户管理、角色、RBAC 权限控制尚未实现，测试错误数达 794 个。

## 现状分析
测试结果:
- `security/01_user_management.sql`: 66 errors
- `security/02_role_management.sql`: 198 errors
- `security/03_privilege_operations.sql`: 180 errors
- `security/04_rbac_scenarios.sql`: 278 errors
- `security/05_security_features.sql`: 72 errors
- `permissions/01_user_permission_positive.sql`: 40 errors
- `permissions/02_permission_negative.sql`: 4 errors

主要缺失:
- CREATE/DROP USER
- CREATE/DROP ROLE
- GRANT/REVOKE 权限语句
- 角色继承和绑定

## 子任务

### Task 1: 用户管理
- 实现 CREATE USER 语句
- 实现 DROP USER 语句
- 实现 ALTER USER 语句
- 支持用户属性 (密码, 认证方式)
- 验证: `security/01_user_management.sql` 基本语句通过

### Task 2: 角色管理
- 实现 CREATE ROLE 语句
- 实现 DROP ROLE 语句
- 实现 SET ROLE 语句
- 实现角色激活/禁用
- 验证: `security/02_role_management.sql` 基本语句通过

### Task 3: 权限授予
- 实现 GRANT privilege ON object TO user/role
- 实现 REVOKE privilege ON object FROM user/role
- 支持表级权限 (SELECT, INSERT, UPDATE, DELETE)
- 支持库级权限
- 验证: `security/03_privilege_operations.sql` GRANT/REVOKE 部分通过

### Task 4: 权限验证
- 在查询执行前检查权限
- 实现角色继承
- 实现默认角色
- 验证: `security/04_rbac_scenarios.sql` 权限检查部分通过

## 验收标准
- [ ] CREATE/DROP USER 正常工作
- [ ] CREATE/DROP ROLE 正常工作
- [ ] GRANT/REVOKE 语句正常解析
- [ ] 查询执行时正确检查权限
- [ ] 安全模块测试通过率 > 80%

## 影响范围
- `fe-sql-parser`: 用户/角色/GRANT 语法解析
- `fe-catalog`: 用户角色元数据存储
- `fe-scheduler`: 权限检查拦截
- `mysql-protocol`: 认证处理
